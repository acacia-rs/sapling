//! A shard ticked by hand: the network's side of the channels is played by the test, so every
//! packet is checked in order without sockets or clocks.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use acacia_auth::login::verify::{SigningKeys, Verifier};
use acacia_proto::packets::{
    PlayerAuthInput, PlayerAuthInputInputMode, PlayerAuthInputInteractionModel, PlayerAuthInputPlayMode, RequestChunkRadius,
    ResourcePackClientResponse, ResourcePackClientResponseResponseStatus as PackStatus,
};
use acacia_proto::types::{Vec2f, Vec3f};
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

pub struct Harness {
    pub shard: Shard,
    pub net: Loopback,
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
        Self { shard, net, log }
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
