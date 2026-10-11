//! Items left lying are kept by the world folder with their column.

mod common;

use std::sync::{Arc, Mutex};

use acacia_proto::packets::AddItemEntity;
use common::stored::{enter_stored, floor_with};
use common::{Harness, all, config};
use sapling_world::World;

#[test]
fn what_a_dead_cow_left_lies_there_when_its_column_comes_back() {
    let mut h = Harness::with(config(), World::with_storage(floor_with(Vec::new())));
    enter_stored(&mut h, 1);
    let cow = Arc::new(Mutex::new(None));
    let spawned = cow.clone();
    h.remote.run(move |ctx| *spawned.lock().unwrap() = ctx.spawn_entity("minecraft:cow", [5.5, 64.0, 5.5]));
    h.tick(1);
    let cow = cow.lock().unwrap().expect("a cow");
    h.remote.run(move |ctx| assert!(ctx.kill_entity(cow)));
    let left: Vec<AddItemEntity> = (0..60).flat_map(|_| all::<AddItemEntity>(&h.tick(1))).collect();
    assert!(!left.is_empty(), "a cow leaves beef");
    let mut lay: Vec<(i16, u16)> = left.iter().map(|item| (item.item.network_id, item.item.count)).collect();

    // Nobody sees the column: it is saved and unloaded, and its items come back with it.
    h.leave(1);
    (0..150).for_each(|_| h.shard.tick());
    let mut back: Vec<(i16, u16)> = all::<AddItemEntity>(&enter_stored(&mut h, 2)).iter().map(|item| (item.item.network_id, item.item.count)).collect();
    (lay.sort(), back.sort());
    assert_eq!(back, lay);
}
