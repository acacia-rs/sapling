//! A shard ticked by hand: the network's side of the channels is played by the test, so every
//! packet is checked in order without sockets or clocks.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use acacia_auth::login::verify::{SigningKeys, Verifier};
use acacia_proto::packets::{
    InventoryTransaction, PlayerAuthInput, PlayerAuthInputBlockActionItem, PlayerAuthInputInputMode, PlayerAuthInputInteractionModel, PlayerAuthInputPlayMode,
    RequestChunkRadius, ResourcePackClientResponse, ResourcePackClientResponseResponseStatus as PackStatus, SetLocalPlayerAsInitialized,
};
use acacia_proto::types::{
    Action as BlockAction, BlockCoordinates, InputData, ItemV4, ItemV4Extra, Transaction, TransactionLegacy, TransactionTransactionData, TransactionTransactionType,
    TransactionUseItem, TransactionUseItemActionType, TransactionUseItemClientCooldownState, TransactionUseItemClientPrediction, TransactionUseItemHand,
    TransactionUseItemTriggerType, Vec2f, Vec3f, Vec3i,
};
use acacia_proto::{GAME_VERSION, Packet, RawPacket};
use p384::ecdsa::SigningKey;
use sapling_api::{Handler, Player, ShardCtx};
use sapling_net::{ConnId, Loopback, NetEvent, Outbound, VerifiedLogin};
use sapling_shard::{Shard, ShardConfig};
use sapling_world::World;

fn login(name: &str) -> Box<VerifiedLogin> {
    let key = SigningKey::random(&mut rand_core::OsRng);
    let client = acacia_auth::ClientData::default_for(name, "127.0.0.1:19132", GAME_VERSION);
    let request = acacia_auth::build_offline_connection_request(name, &key, &client);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    Box::new(Verifier::new(SigningKeys::empty()).verify(&request, now).unwrap())
}

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Handler for Log {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        self.0.lock().unwrap().push(format!("join {} ({} online)", player.name, ctx.player_count()));
    }

    fn on_quit(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        self.0.lock().unwrap().push(format!("quit {} ({} online)", player.name, ctx.player_count()));
    }
}

type Action = Box<dyn FnOnce(&mut ShardCtx<'_>) + Send>;

/// Lets a test act through the context: what [`Remote::run`] queues happens in the next tick's `on_tick`.
#[derive(Clone, Default)]
pub struct Remote(Arc<Mutex<Vec<Action>>>);

impl Remote {
    pub fn run(&self, action: impl FnOnce(&mut ShardCtx<'_>) + Send + 'static) {
        self.0.lock().unwrap().push(Box::new(action));
    }
}

impl Handler for Remote {
    fn on_tick(&mut self, ctx: &mut ShardCtx<'_>) {
        let actions = std::mem::take(&mut *self.0.lock().unwrap());
        actions.into_iter().for_each(|action| action(ctx));
    }
}

pub struct Harness {
    pub shard: Shard,
    pub net: Loopback,
    pub remote: Remote,
    log: Log,
}

/// A void world with a 16x16 stone platform at y 64 around the origin.
pub fn platform() -> World {
    let mut level = World::void();
    let stone = level.block_id("minecraft:stone").unwrap();
    level.fill([-8, 64, -8], [7, 64, 7], stone);
    level
}

pub fn config() -> ShardConfig {
    ShardConfig { world_name: "test".into(), max_view_radius: 4, columns_per_tick: 16, ..ShardConfig::default() }
}

impl Harness {
    pub fn new() -> Self {
        Self::with(config(), platform())
    }

    pub fn with(config: ShardConfig, level: World) -> Self {
        let (net, outbox, inbox) = sapling_net::loopback();
        let mut shard = Shard::new(config, level, inbox, outbox);
        let log = Log::default();
        shard.add_handler(Box::new(log.clone()));
        let remote = Remote::default();
        shard.add_handler(Box::new(remote.clone()));
        Self { shard, net, remote, log }
    }

    pub fn join(&mut self, conn: u64, name: &str) {
        let event = NetEvent::Joined { conn: ConnId::new(conn), addr: "127.0.0.1:5000".parse().unwrap(), login: login(name) };
        self.net.events.send(event).unwrap();
    }

    pub fn receive<T: Packet>(&mut self, conn: u64, packet: &T) {
        let packet = RawPacket::parse(sapling_net::encode(packet)).unwrap();
        self.net.events.send(NetEvent::Packet { conn: ConnId::new(conn), packet }).unwrap();
    }

    pub fn leave(&mut self, conn: u64) {
        self.net.events.send(NetEvent::Left { conn: ConnId::new(conn), reason: "test".into() }).unwrap();
    }

    /// Ticks once and returns what the shard sent to `conn` in that tick.
    pub fn tick(&mut self, conn: u64) -> Vec<RawPacket> {
        self.shard.tick();
        let mut sent = Vec::new();
        while let Ok(command) = self.net.sent.try_recv() {
            match command {
                Outbound::Send { conn: to, packets } if to == ConnId::new(conn) => sent.extend(packets.into_iter().map(|p| RawPacket::parse(p).unwrap())),
                other => panic!("unexpected {other:?}"),
            }
        }
        sent
    }

    /// Ticks once and returns what the shard sent, by connection.
    pub fn tick_all(&mut self) -> Sent {
        self.shard.tick();
        let mut sent = Sent::default();
        while let Ok(command) = self.net.sent.try_recv() {
            match command {
                Outbound::Send { conn, packets } => sent.0.entry(conn.get()).or_default().extend(packets.into_iter().map(|p| RawPacket::parse(p).unwrap())),
                other => panic!("unexpected {other:?}"),
            }
        }
        sent
    }

    /// Walks `name` through the whole join and returns what was sent on the tick it entered the
    /// world.
    pub fn enter(&mut self, conn: u64, name: &str) -> Sent {
        self.join(conn, name);
        self.tick_all();
        self.receive(conn, &pack_response(PackStatus::HaveAllPacks));
        self.tick_all();
        self.receive(conn, &pack_response(PackStatus::Completed));
        self.tick_all();
        self.receive(conn, &RequestChunkRadius { chunk_radius: 4, max_radius: 4 });
        // 49 columns at 16 a tick.
        (0..5).for_each(|_| drop(self.tick_all()));
        self.receive(conn, &SetLocalPlayerAsInitialized { runtime_entity_id: 0 });
        self.tick_all()
    }

    /// Walks `conn` through packs and StartGame up to its chunk radius request; the next tick
    /// starts the chunks.
    pub fn join_to_radius(&mut self, conn: u64, radius: i32) {
        self.join(conn, "Steve");
        self.tick(conn);
        self.receive(conn, &pack_response(PackStatus::HaveAllPacks));
        self.tick(conn);
        self.receive(conn, &pack_response(PackStatus::Completed));
        self.tick(conn);
        self.receive(conn, &RequestChunkRadius { chunk_radius: radius, max_radius: radius as u8 });
    }

    pub fn log(&self) -> Vec<String> {
        self.log.0.lock().unwrap().clone()
    }
}

/// One tick's packets by connection.
#[derive(Default)]
pub struct Sent(HashMap<u64, Vec<RawPacket>>);

impl Sent {
    pub fn to(&self, conn: u64) -> &[RawPacket] {
        self.0.get(&conn).map_or(&[], Vec::as_slice)
    }
}

pub fn ids(packets: &[RawPacket]) -> Vec<u32> {
    packets.iter().map(|p| p.id).collect()
}

/// Every `T` among `packets`, in order.
pub fn all<T: Packet>(packets: &[RawPacket]) -> Vec<T> {
    packets.iter().filter(|p| p.id == T::ID).map(|p| p.decode().unwrap()).collect()
}

pub fn pack_response(status: PackStatus) -> ResourcePackClientResponse {
    ResourcePackClientResponse { response_status: status, response_status_name: String::new(), resourcepackids: None }
}

/// The id a block's first state has on the wire.
pub fn hash(name: &str) -> u32 {
    let registry = sapling_world::BlockRegistry::vanilla();
    registry.network_hash(registry.states_of(name).next().expect("a vanilla block").0)
}

/// The input of a player at the platform's spawn whose client reports block actions on top faces.
pub fn block_actions(actions: &[(BlockAction, [i32; 3])]) -> PlayerAuthInput {
    let item = |&(action, [x, y, z]): &(BlockAction, [i32; 3])|PlayerAuthInputBlockActionItem { action, position: Vec3i { x, y, z }, face: 1 };
    PlayerAuthInput { input_data: vec![InputData::BlockAction], block_action: Some(actions.iter().map(item).collect()), ..input_at([0.5, 65.0, 0.5]) }
}

/// A click on `face` of the block at `pos` by a client that says it holds the block `held`.
pub fn click([x, y, z]: [i32; 3], face: u8, held: Option<&str>) -> InventoryTransaction {
    let held_item = ItemV4 {
        network_id: held.map_or(0, |_| 1),
        count: held.map_or(0, |_| 1),
        metadata: 0,
        has_stack_id: false,
        stack_id: None,
        block_runtime_id: held.map_or(0, hash),
        extra: ItemV4Extra::Default(None),
    };
    let data = TransactionUseItem {
        action_type: TransactionUseItemActionType::ClickBlock,
        trigger_type: TransactionUseItemTriggerType::PlayerInput,
        block_position: BlockCoordinates { x, y, z },
        face,
        hotbar_slot: 0,
        hand: TransactionUseItemHand::MainHand,
        held_item,
        player_pos: Vec3f { x: 0.5, y: 65.0 + sapling_data::EYE_HEIGHT, z: 0.5 },
        click_pos: Vec3f { x: 0.5, y: 1.0, z: 0.5 },
        block_runtime_id: 0,
        client_prediction: TransactionUseItemClientPrediction::Success,
        client_cooldown_state: TransactionUseItemClientCooldownState::Off,
    };
    let legacy = TransactionLegacy { legacy_request_id: 0, legacy_transactions: None };
    InventoryTransaction { transaction: Transaction { legacy, transaction_type: TransactionTransactionType::ItemUse, actions: Vec::new(), transaction_data: TransactionTransactionData::ItemUse(data) } }
}

/// The input of a player standing with its feet at `feet`.
pub fn input_at(feet: [f32; 3]) -> PlayerAuthInput {
    let zero = || Vec2f { x: 0.0, z: 0.0 };
    PlayerAuthInput {
        pitch: 0.0,
        yaw: 0.0,
        position: Vec3f { x: feet[0], y: feet[1] + sapling_data::EYE_HEIGHT, z: feet[2] },
        move_vector: zero(),
        head_yaw: 0.0,
        input_data: Vec::new(),
        input_mode: PlayerAuthInputInputMode::Mouse,
        play_mode: PlayerAuthInputPlayMode::Normal,
        interaction_model: PlayerAuthInputInteractionModel::Crosshair,
        interact_rotation: zero(),
        tick: 0,
        delta: Vec3f { x: 0.0, y: 0.0, z: 0.0 },
        transaction: None,
        item_stack_request: None,
        block_action: None,
        vehicle_rotation: None,
        predicted_vehicle: None,
        analogue_move_vector: zero(),
        camera_orientation: Vec3f { x: 0.0, y: 0.0, z: 0.0 },
        raw_move_vector: zero(),
    }
}
