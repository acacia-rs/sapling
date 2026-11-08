//! Items and orbs left lying are kept by the world folder with their column.

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::RawPacket;
use acacia_proto::packets::AddItemEntity;
use common::orbs::worths;
use common::stored::{enter_stored, floor_with};
use common::{Harness, all, config};
use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::RemoveEntity;
use common::stored::actor_with;
use sapling_world::{ChunkPos, World, WorldStorage, actor_age, orb_value, stored_actors};

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
    let ages: Vec<i16> = stored_actors(&column.actors).unwrap().iter().filter(|(.., compound)| orb_value(compound).is_some()).map(|(.., compound)| actor_age(compound)).collect();
    assert!(ages.iter().all(|age| (60..=210).contains(age)), "kept with the ticks they lived, as BDS's `Age`: {ages:?}");
}

/// BDS keeps a lying item's `Age` (research/captures/bds-t2ekeep-dump.txt, bds-f14nkeep-dump.txt).
#[test]
fn a_lying_item_is_kept_with_its_age_and_comes_back_as_old() {
    let item = |name: &str| Value::Compound(vec![("Count".into(), Value::Byte(1)), ("Damage".into(), Value::Short(0)), ("Name".into(), Value::String(name.into())), ("WasPickedUp".into(), Value::Byte(0))]);
    let motion = ("Motion", Value::List(List { tag: 5, items: vec![Value::Float(0.0); 3] }));
    let old = actor_with(-901, "minecraft:item", [3.5, 64.0, 3.5], vec![("Item", item("minecraft:dirt")), ("Age", Value::Short(5990)), motion.clone()]);
    let young = actor_with(-902, "minecraft:item", [9.5, 64.0, 9.5], vec![("Item", item("minecraft:stone")), ("Age", Value::Short(100)), motion]);
    let storage = floor_with([old, young].concat());
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    let sent: Vec<RawPacket> = (0..30).flat_map(|_| h.tick(1)).collect();
    assert_eq!(all::<RemoveEntity>(&sent).len(), 1, "the item 10 ticks from its 6000 goes; the young one stays");
    h.leave(1);
    (0..150).for_each(|_| h.shard.tick());
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let ages: Vec<i16> = stored_actors(&column.actors).unwrap().iter().filter(|(actor, ..)| actor.as_ref().is_some_and(|actor| actor.identifier == "minecraft:item")).map(|(.., compound)| actor_age(compound)).collect();
    assert!(matches!(ages[..], [age] if (130..=290).contains(&age)), "the young one saved older: {ages:?}");
}
