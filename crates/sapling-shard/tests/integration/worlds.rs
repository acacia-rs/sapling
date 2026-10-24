//! Several worlds in one shard: who sees what, and a player's move from one to another
//! (harness: common/cluster.rs).

use crate::common;

use acacia_proto::Packet;
use acacia_proto::packets::{
    AddPlayer, ClientCacheBlobStatus, ClientCacheMissResponse, ClientCacheStatus, LevelChunk, MovePlayer, MovePlayerMode, NetworkChunkPublisherUpdate, PlayStatus, PlayerList,
    RemoveEntity, Subchunk, SubchunkRequest, SyncWorldClocks, Text, UpdateBlock,
};
use acacia_proto::types::{Action as BlockAction, InputData, Vec3i8, Vec3li};
use common::cluster::Cluster;
use common::{all, block_actions, click, ids, input_at};
use sapling_api::{Block, PlayerId};
use sapling_data::EYE_HEIGHT;
use sapling_world::{BlockRegistry, Chunk, Dimension};

/// Alex (connection and entity 1) and Steve (2) in the hub of a shard that also has an arena.
fn hub_and_arena() -> Cluster {
    let mut c = Cluster::new(&[&["hub", "arena"]]);
    c.enter(1, "Alex");
    c.enter(2, "Steve");
    assert_eq!(c.journal(), ["0: join Alex in hub", "0: join Steve in hub"]);
    c
}

/// The block at the middle of the platform in an encoded section 4.
fn platform_block(section: &[u8]) -> &'static str {
    let registry = BlockRegistry::vanilla();
    let mut decoded = Chunk::empty(0, 0, Dimension::overworld(registry.air_id()));
    decoded.set_sub_chunk_mapped(4, section, &|hash| registry.runtime_id_from_hash(hash).unwrap()).unwrap();
    registry.get(decoded.block(0, 64, 0)).unwrap().name
}

fn request(sections: &[(i8, i8, i8)]) -> SubchunkRequest {
    let requests = sections.iter().map(|&(x, y, z)| Vec3i8 { x, y, z }).collect();
    SubchunkRequest { dimension: 0, requests, origin: Vec3li { x: 0, y: 0, z: 0 } }
}

#[test]
fn a_move_to_another_world_swaps_what_the_client_shows() {
    let mut c = hub_and_arena();
    c.transfer(0, 2, "arena");
    let sent = c.tick();
    assert_eq!(ids(sent.to(2)), [MovePlayer::ID, SyncWorldClocks::ID, RemoveEntity::ID], "to the spawn, the time there, the hub's players gone");
    let jump: MovePlayer = sent.to(2)[0].decode().unwrap();
    assert_eq!((jump.runtime_id, jump.mode, [jump.position.x, jump.position.y, jump.position.z]), (2, MovePlayerMode::Teleport, [0.5, 65.0 + EYE_HEIGHT, 0.5]));
    assert_eq!(all::<RemoveEntity>(sent.to(2)), [RemoveEntity { entity_id_self: 1 }]);
    assert_eq!((ids(sent.to(1)), all::<RemoveEntity>(sent.to(1))), (vec![RemoveEntity::ID], vec![RemoveEntity { entity_id_self: 2 }]));
    assert_eq!(c.journal(), ["0: transfer 2 to arena: true", "0: change Steve hub -> arena (1 here)"], "no quit, no join");

    // Every column of the view again, from the arena, in the same dimension: no loading screen.
    let sent = c.ticks(5);
    assert_eq!(sent.to(2)[0].id, NetworkChunkPublisherUpdate::ID);
    assert_eq!((all::<LevelChunk>(sent.to(2)).len(), sent.to(2).len()), (49, 50), "{:?}", ids(sent.to(2)));
    assert!(sent.to(1).is_empty() && all::<PlayStatus>(sent.to(2)).is_empty());
    c.receive(2, &request(&[(0, 4, 0)]));
    let answer = all::<Subchunk>(c.tick().to(2)).remove(0);
    assert_eq!(platform_block(answer.entries[0].payload.as_ref().unwrap()), "minecraft:sandstone");

    // Back: each is an entity for the other again, the one who came once its view is centred.
    c.transfer(0, 2, "hub");
    let sent = c.tick();
    assert_eq!((ids(sent.to(2)), ids(sent.to(1))), (vec![MovePlayer::ID, SyncWorldClocks::ID], vec![AddPlayer::ID]));
    let sent = c.ticks(5);
    assert_eq!((all::<AddPlayer>(sent.to(2)).len(), all::<LevelChunk>(sent.to(2)).len(), sent.to(1).len()), (1, 49, 0));
    assert_eq!(c.journal(), ["0: transfer 2 to hub: true", "0: change Steve arena -> hub (2 here)"]);
    c.transfer(0, 2, "hub");
    c.transfer(0, 2, "nowhere");
    c.transfer(0, 9, "arena");
    assert!(c.tick().to(2).is_empty());
    assert_eq!(c.journal(), ["0: transfer 2 to hub: false", "0: transfer 2 to nowhere: false", "0: transfer 9 to arena: false"]);
}

#[test]
fn two_worlds_of_a_shard_are_invisible_to_each_other() {
    let mut c = hub_and_arena();
    c.transfer(0, 2, "arena");
    c.ticks(6);
    c.journal();

    // Movement and chat stay in their world.
    c.receive(1, &input_at([2.5, 65.0, 0.5]));
    c.chat(1, "anyone here?");
    let sent = c.tick();
    assert_eq!((ids(sent.to(1)), sent.to(2).len()), (vec![Text::ID], 0));
    c.chat(2, "only me");
    let sent = c.tick();
    assert_eq!((ids(sent.to(2)), sent.to(1).len()), (vec![Text::ID], 0));
    assert_eq!(c.journal(), ["0: chat Alex anyone here? in hub", "0: chat Steve only me in arena"]);

    // A block and the time change for those in that world; `broadcast` is the shard's.
    c.note(0, |ctx| {
        let placed = ctx.set_block_in("arena", [0, 66, 0], &Block::new("minecraft:gold_block"));
        ctx.within("arena", |ctx| ctx.set_time(1000));
        ctx.broadcast("shard");
        ctx.broadcast_in("arena", "arena");
        ctx.broadcast_in("nowhere", "nobody");
        let arena = (ctx.block_in("arena", [0, 66, 0]).map(|b| b.name), ctx.within("arena", |ctx| ctx.time()));
        format!("{placed} {arena:?} {:?} {} {:?} {:?}", ctx.block([0, 66, 0]).map(|b| b.name), ctx.time() == 1000, ctx.worlds(), ctx.world())
    });
    let sent = c.tick();
    assert_eq!(ids(sent.to(2)), [UpdateBlock::ID, SyncWorldClocks::ID, Text::ID, Text::ID]);
    assert_eq!(ids(sent.to(1)), [Text::ID]);
    assert_eq!(c.journal(), [r#"0: true (Some("minecraft:gold_block"), Some(1000)) Some("minecraft:air") false ["arena", "hub"] "hub""#]);
    assert_eq!(c.census(), ["0: 2 [Alex in hub, Steve in arena]"]);

    // A newcomer joins the hub: listed for both, an entity only for the one in the hub.
    c.enter(3, "Late");
    c.tick();
    c.journal();
    c.leave(3);
    let sent = c.tick();
    assert_eq!((ids(sent.to(1)), ids(sent.to(2))), (vec![RemoveEntity::ID, PlayerList::ID], vec![PlayerList::ID]));
    assert_eq!(c.journal(), ["0: quit Late in hub"]);
}

#[test]
fn what_the_client_sends_from_the_old_world_is_ignored() {
    let mut c = hub_and_arena();
    c.receive(2, &input_at([5.5, 65.0, 0.5]));
    c.tick();
    c.transfer(0, 2, "arena");
    c.tick();
    c.journal();

    // Inputs on their way when the world changed: a position in the hub, a break and a click.
    c.receive(2, &input_at([5.5, 65.0, 0.5]));
    c.receive(2, &block_actions(&[(BlockAction::PredictBreak, [0, 64, 0])]));
    c.receive(2, &click([1, 64, 0], 1, Some("minecraft:stone")));
    let sent = c.ticks(5);
    assert!(all::<UpdateBlock>(sent.to(2)).is_empty() && all::<MovePlayer>(sent.to(2)).is_empty(), "{:?}", ids(sent.to(2)));
    let state = |ctx: &mut sapling_api::ShardCtx<'_>| format!("{:?} {:?}", ctx.position(PlayerId(2)).map(|p| p.feet), ctx.block_in("arena", [0, 64, 0]).map(|b| b.name));
    c.note(0, state);
    c.tick();
    assert_eq!(c.journal(), [r#"0: Some([0.5, 65.0, 0.5]) Some("minecraft:sandstone")"#]);

    // Once the client says it followed the teleport, it is heard again.
    let mut followed = block_actions(&[(BlockAction::PredictBreak, [0, 64, 0])]);
    followed.input_data.push(InputData::HandledTeleport);
    c.receive(2, &followed);
    c.receive(2, &input_at([1.5, 65.0, 0.5]));
    let sent = c.tick();
    assert_eq!(all::<UpdateBlock>(sent.to(2)).len(), 1, "the break in the arena counts: {:?}", ids(sent.to(2)));
    c.note(0, state);
    c.tick();
    assert_eq!(c.journal(), [r#"0: Some([1.5, 65.0, 0.5]) Some("minecraft:air")"#]);
}

#[test]
fn a_world_change_lets_go_of_the_old_worlds_sections_and_blobs() {
    let mut c = hub_and_arena();
    c.receive(2, &ClientCacheStatus { enabled: true });
    c.transfer(0, 2, "arena");
    c.ticks(6);
    c.receive(2, &request(&[(0, 4, 0)]));
    let sandstone = all::<Subchunk>(c.tick().to(2))[0].entries[0].blob_id.expect("a section hash");

    // What the client asked in the arena arrives after the move: a near and a far column.
    c.transfer(0, 2, "hub");
    c.tick();
    c.receive(2, &request(&[(4, 4, 0), (0, 4, 0)]));
    let sent = c.tick();
    let packets = sent.to(2);
    let answered = packets.iter().position(|p| p.id == Subchunk::ID).expect("the near column was announced this tick");
    let announced: Vec<(i32, i32)> = all::<LevelChunk>(&packets[..answered]).iter().map(|chunk| (chunk.x, chunk.z)).collect();
    assert!(announced.contains(&(0, 0)) && !announced.contains(&(4, 0)), "an answer follows its column: {announced:?}");
    let answers = all::<Subchunk>(packets);
    let near = &answers[0].entries[0];
    assert_eq!((answers.len(), answers[0].entries.len(), near.dx), (1, 1, 0), "the far section waits for its column");
    let stone = near.blob_id.expect("the hub's section, not the arena's");
    assert_ne!(stone, sandstone);

    // The far one still waits when the player moves on: it is dropped, not answered from anywhere.
    c.transfer(0, 2, "arena");
    let sent = c.ticks(7);
    assert_eq!((all::<Subchunk>(sent.to(2)).len(), all::<LevelChunk>(sent.to(2)).iter().filter(|chunk| chunk.x == 4 && chunk.z == 0).count()), (0, 1));

    // Reports on blobs of worlds the client left get the one reply every status gets, empty.
    c.receive(2, &ClientCacheBlobStatus { missing: vec![sandstone, stone], have: vec![] });
    let replies = all::<ClientCacheMissResponse>(c.tick().to(2));
    assert_eq!((replies.len(), replies[0].blobs.len()), (1, 0));
    c.receive(2, &request(&[(0, 4, 0)]));
    assert_eq!(all::<Subchunk>(c.tick().to(2))[0].entries[0].blob_id, Some(sandstone));
    c.receive(2, &ClientCacheBlobStatus { missing: vec![sandstone], have: vec![] });
    let replies = all::<ClientCacheMissResponse>(c.tick().to(2));
    assert_eq!(platform_block(&replies[0].blobs[0].payload), "minecraft:sandstone");
}
