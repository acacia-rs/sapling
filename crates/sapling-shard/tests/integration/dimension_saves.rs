//! What a world with a Nether keeps: each dimension's columns and actors under its own keys, and
//! a player who left in the Nether starts there again.

use crate::common;

use std::sync::Arc;

use acacia_proto::packets::{LevelChunk, PlayerAction, PlayerAuthInput};
use acacia_proto::types::{Action, BlockCoordinates, InputData};
use common::stored::{enter_stored, floor_with};
use common::{Harness, all, config, input_at};
use sapling_api::{Block, PlayerId};
use sapling_world::{ChunkPos, MemoryStorage, NETHER, OVERWORLD, World, WorldStorage, stored_actors};

fn with_nether(storage: &Arc<MemoryStorage>) -> Harness {
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    assert!(h.shard.add_dimension("default", World::with_storage_in(storage.clone(), NETHER)));
    h
}

fn ack() -> PlayerAction {
    let origin = BlockCoordinates { x: 0, y: 0, z: 0 };
    PlayerAction { runtime_entity_id: 1, action: Action::DimensionChangeAck, position: origin.clone(), result_position: origin, face: 0 }
}

#[test]
fn the_nether_saves_apart_and_a_player_comes_back_to_it() {
    let storage = floor_with(Vec::new());
    let mut h = with_nether(&storage);
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| assert!(ctx.travel(PlayerId(1), NETHER, [3.5, 40.0, 3.5])));
    (0..20).for_each(|_| drop(h.tick(1)));
    h.remote.run(|ctx| {
        let floor = |ctx: &mut sapling_api::ShardCtx<'_>| [[3, 39, 3], [4, 39, 3], [5, 39, 5]].iter().all(|at| ctx.set_block(*at, &Block::new("minecraft:netherrack")));
        assert_eq!(ctx.within("default:nether", |ctx| floor(ctx) && ctx.spawn_entity("minecraft:pig", [5.5, 40.0, 5.5]).is_some()), Some(true));
    });
    h.receive(1, &ack());
    h.tick(1);
    h.receive(1, &PlayerAuthInput { input_data: vec![InputData::HandledTeleport], ..input_at([4.5, 40.0, 3.5]) });
    (0..3).for_each(|_| drop(h.tick(1)));
    h.leave(1);
    h.tick(1);
    h.shard.save_worlds();

    // The Nether's column is under its own key, with its pig; the overworld's is as it was.
    assert_eq!((storage.columns(OVERWORLD).unwrap(), storage.columns(NETHER).unwrap()), (vec![ChunkPos::new(0, 0)], vec![ChunkPos::new(0, 0)]));
    let nether = storage.load_column(ChunkPos::new(0, 0), NETHER).unwrap().unwrap();
    let kinds: Vec<String> = stored_actors(&nether.actors).unwrap().into_iter().filter_map(|(actor, ..)| actor.map(|actor| actor.identifier)).collect();
    assert_eq!(kinds, ["minecraft:pig"]);
    assert!(stored_actors(&storage.load_column(ChunkPos::new(0, 0), OVERWORLD).unwrap().unwrap().actors).unwrap().is_empty());

    // Back on a server of the same storage: StartGame puts the player in the Nether, where it stood.
    let mut h = with_nether(&storage);
    let sent = enter_stored(&mut h, 1);
    let columns = all::<LevelChunk>(&sent);
    assert!(!columns.is_empty() && columns.iter().all(|column| column.dimension == NETHER), "its columns are the Nether's");
    h.remote.run(|ctx| assert_eq!((ctx.dimension_of(PlayerId(1)), ctx.position(PlayerId(1)).map(|p| [p.feet[0], p.feet[2]])), (Some(NETHER), Some([4.5, 3.5]))));
    h.tick(1);
}
