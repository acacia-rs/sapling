//! A shard ticked by hand: the network's side of the channels is played by the test, so every
//! packet of the join sequence is checked in order without sockets or clocks.

use std::sync::{Arc, Mutex};

use acacia_auth::login::verify::{SigningKeys, Verifier};
use acacia_proto::packets::{
    AvailableEntityIdentifiers, BiomeDefinitionList, ChunkRadiusUpdate, CreativeContent, ItemRegistry, LevelChunk, NetworkChunkPublisherUpdate,
    PlayStatus, PlayStatusStatus, RequestChunkRadius, ResourcePackClientResponse, ResourcePackClientResponseResponseStatus as PackStatus,
    ResourcePackStack, ResourcePacksInfo, SetEntityData, SetLocalPlayerAsInitialized, StartGame, Subchunk, SubchunkRequest, UpdateAbilities,
    UpdateAttributes,
};
use acacia_proto::types::{SubChunkEntryItemResult, Vec3i8, Vec3li};
use acacia_proto::{GAME_VERSION, Packet, RawPacket};
use p384::ecdsa::SigningKey;
use sapling_api::{Handler, Player, ShardCtx};
use sapling_net::{ConnId, Loopback, NetEvent, Outbound, VerifiedLogin};
use sapling_shard::{Shard, ShardConfig};

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

struct Harness {
    shard: Shard,
    net: Loopback,
    log: Log,
}

impl Harness {
    fn new() -> Self {
        let mut level = sapling_world::World::void();
        let stone = level.block_id("minecraft:stone").unwrap();
        level.fill([-8, 64, -8], [7, 64, 7], stone);
        let (net, outbox, inbox) = sapling_net::loopback();
        let mut shard = Shard::new(ShardConfig { world_name: "test".into(), max_view_radius: 4, ..ShardConfig::default() }, level, inbox, outbox);
        let log = Log::default();
        shard.add_handler(Box::new(log.clone()));
        Self { shard, net, log }
    }

    fn join(&mut self, conn: u64, name: &str) {
        let event = NetEvent::Joined { conn: ConnId::new(conn), addr: "127.0.0.1:5000".parse().unwrap(), login: login(name) };
        self.net.events.send(event).unwrap();
    }

    fn receive<T: Packet>(&mut self, conn: u64, packet: &T) {
        let packet = RawPacket::parse(sapling_net::encode(packet)).unwrap();
        self.net.events.send(NetEvent::Packet { conn: ConnId::new(conn), packet }).unwrap();
    }

    /// Ticks once and returns what the shard sent to `conn` in that tick.
    fn tick(&mut self, conn: u64) -> Vec<RawPacket> {
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

    fn log(&self) -> Vec<String> {
        self.log.0.lock().unwrap().clone()
    }
}

fn ids(packets: &[RawPacket]) -> Vec<u32> {
    packets.iter().map(|p| p.id).collect()
}

fn pack_response(status: PackStatus) -> ResourcePackClientResponse {
    ResourcePackClientResponse { response_status: status, response_status_name: String::new(), resourcepackids: None }
}

#[test]
fn a_player_is_walked_through_the_join_sequence() {
    let mut h = Harness::new();
    h.join(1, "Steve");
    assert_eq!(ids(&h.tick(1)), [ResourcePacksInfo::ID]);
    assert!(h.tick(1).is_empty(), "the shard waits for the client");

    h.receive(1, &pack_response(PackStatus::HaveAllPacks));
    assert_eq!(ids(&h.tick(1)), [ResourcePackStack::ID]);

    h.receive(1, &pack_response(PackStatus::Completed));
    let sent = h.tick(1);
    use acacia_proto::packets::{CameraPresets, FeatureRegistry, JigsawStructureData, TrimData, VoxelShapes};
    let expected = [
        JigsawStructureData::ID,
        VoxelShapes::ID,
        StartGame::ID,
        ItemRegistry::ID,
        BiomeDefinitionList::ID,
        AvailableEntityIdentifiers::ID,
        FeatureRegistry::ID,
        CameraPresets::ID,
        CreativeContent::ID,
        TrimData::ID,
        UpdateAbilities::ID,
        SetEntityData::ID,
        UpdateAttributes::ID,
    ];
    assert_eq!(ids(&sent), expected);
    let start: StartGame = sent[2].decode().unwrap();
    assert_eq!((start.runtime_entity_id, start.world_name.as_str()), (1, "test"));
    assert_eq!((start.player_position.x, start.player_position.y, start.player_position.z), (0.5, 65.0 + sapling_data::EYE_HEIGHT, 0.5));
    assert!(h.tick(1).is_empty(), "no chunks before the client asks for a radius");

    // Radius 9 is capped to the configured 4: 49 columns in the circle.
    h.receive(1, &RequestChunkRadius { chunk_radius: 9, max_radius: 9 });
    let sent = h.tick(1);
    assert_eq!(sent[0].decode::<ChunkRadiusUpdate>().unwrap().chunk_radius, 4);
    assert_eq!(sent[1].decode::<NetworkChunkPublisherUpdate>().unwrap().radius, 64);
    let chunks: Vec<LevelChunk> = sent[2..sent.len() - 1].iter().map(|p| p.decode().unwrap()).collect();
    assert_eq!(chunks.len(), 49);
    assert_eq!((chunks[0].x, chunks[0].z, chunks[0].highest_subchunk_count), (0, 0, Some(9)));
    assert_eq!(sent.last().unwrap().decode::<PlayStatus>().unwrap().status, PlayStatusStatus::PlayerSpawn);
    assert!(h.log().is_empty(), "not joined until the client has loaded");

    let requests = vec![Vec3i8 { x: 0, y: 4, z: 0 }, Vec3i8 { x: 3, y: 4, z: 0 }, Vec3i8 { x: 0, y: 30, z: 0 }];
    h.receive(1, &SubchunkRequest { dimension: 0, requests, origin: Vec3li { x: 0, y: 0, z: 0 } });
    let sent = h.tick(1);
    let answer: Subchunk = sent[0].decode().unwrap();
    let results: Vec<_> = answer.entries.iter().map(|e| e.result).collect();
    assert_eq!(results, [SubChunkEntryItemResult::Success, SubChunkEntryItemResult::SuccessAllAir, SubChunkEntryItemResult::YIndexOutOfBounds]);

    h.receive(1, &SetLocalPlayerAsInitialized { runtime_entity_id: 1 });
    assert!(h.tick(1).is_empty());
    assert_eq!(h.log(), ["join Steve (1 online)"]);

    h.net.events.send(NetEvent::Left { conn: ConnId::new(1), reason: "test".into() }).unwrap();
    h.tick(1);
    assert_eq!(h.log(), ["join Steve (1 online)", "quit Steve (0 online)"]);
}

#[test]
fn players_get_their_own_ids_and_leaving_early_is_not_a_quit() {
    let mut h = Harness::new();
    h.join(1, "Alex");
    h.join(2, "Steve");
    h.shard.tick();
    while h.net.sent.try_recv().is_ok() {}
    for conn in [1, 2] {
        h.receive(conn, &pack_response(PackStatus::HaveAllPacks));
    }
    h.shard.tick();
    while h.net.sent.try_recv().is_ok() {}
    h.receive(2, &pack_response(PackStatus::Completed));
    let start: StartGame = h.tick(2)[2].decode().unwrap();
    assert_eq!((start.runtime_entity_id, start.entity_id), (2, 2));

    // Out-of-order packets are ignored: Alex has no StartGame yet.
    h.receive(1, &RequestChunkRadius { chunk_radius: 4, max_radius: 4 });
    h.receive(1, &SetLocalPlayerAsInitialized { runtime_entity_id: 1 });
    assert!(h.tick(1).is_empty());

    h.net.events.send(NetEvent::Left { conn: ConnId::new(1), reason: "test".into() }).unwrap();
    h.shard.tick();
    assert!(h.log().is_empty(), "a player who never finished joining does not quit either");
}
