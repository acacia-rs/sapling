//! Items and orbs left lying are kept by the world folder with their column.

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::RawPacket;
use acacia_proto::packets::AddItemEntity;
use common::orbs::worths;
use common::stored::{enter_stored, floor_with};
use common::{Harness, all, config};
use sapling_world::{ChunkPos, World, WorldStorage, orb_value, stored_actors};

fn orbs(sent: &[RawPacket]) -> usize {
    worths(sent).len()
}

#[test]
fn what_a_dead_cow_left_lies_there_when_its_column_comes_back() {
    let mut h = Harness::with(config(), World::with_storage(floor_with(Vec::new())));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| ctx.set_game_mode(sapling_api::PlayerId(1), sapling_api::GameMode::Creative));
    let cow = Arc::new(Mutex::new(None));
    let spawned = cow.clone();
    h.remote.run(move |ctx| *spawned.lock().unwrap() = ctx.spawn_entity("minecraft:cow", [5.5, 64.0, 5.5]));
    h.tick(1);
    let cow = cow.lock().unwrap().expect("a cow");
    h.remote.run(move |ctx| assert!(ctx.kill_entity(cow)));
    let sent: Vec<RawPacket> = (0..60).flat_map(|_| h.tick(1)).collect();
    let (left, bobbing) = (all::<AddItemEntity>(&sent), orbs(&sent));
    assert!(!left.is_empty(), "a cow leaves beef");
    let mut lay: Vec<(i16, u16)> = left.iter().map(|item| (item.item.network_id, item.item.count)).collect();

    // Nobody sees the column: it is saved and unloaded, and its items come back with it.
    h.leave(1);
    (0..150).for_each(|_| h.shard.tick());
    let sent = enter_stored(&mut h, 2);
    let mut back: Vec<(i16, u16)> = all::<AddItemEntity>(&sent).iter().map(|item| (item.item.network_id, item.item.count)).collect();
    (lay.sort(), back.sort());
    assert_eq!((back, orbs(&sent)), (lay, bobbing), "its items, and the orbs nobody took");
}

/// Read off the saved records: orbs that come back join up again at once, which would hide it.
#[test]
fn orbs_that_joined_up_are_saved_worth_what_they_joined_into() {
    let storage = floor_with(Vec::new());
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| assert!((0..40).all(|_| ctx.spawn_entity("minecraft:xp_orb", [5.5, 64.0, 5.5]).is_some())));
    (0..60).for_each(|_| drop(h.tick(1)));
    h.leave(1);
    (0..150).for_each(|_| h.shard.tick());
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let saved: Vec<i32> = stored_actors(&column.actors).unwrap().iter().filter_map(|(.., compound)| orb_value(compound)).collect();
    assert_eq!(saved.iter().sum::<i32>(), 40, "the forty orbs' worth: {saved:?}");
    assert!(saved.len() < 40, "some joined up: {saved:?}");
}
