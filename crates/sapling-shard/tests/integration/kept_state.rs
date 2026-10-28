//! What a mob keeps beyond its place and health, read and written as a BDS world held it
//! (research/captures/bds-keepmobs.txt): effects, a primed TNT's fuse, a wither's fight.

use crate::common;

use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::AddEntity;
use common::stored::{actor_with, compound, enter_stored, floor_with};
use common::{Harness, all, config};
use sapling_world::{ChunkPos, World, WorldStorage, WitherRecord, actor_effects, actor_fuse, stored_actors, wither_record};

/// One effect as BDS wrote it, less the fade data it reads without.
fn effect(id: i8, amplifier: i8, left: i32, particles: i8) -> Value {
    let durations = ["Duration", "DurationEasy", "DurationHard", "DurationNormal"].map(|key| (key, Value::Int(left)));
    let [a, b, c, d] = durations;
    compound([("Ambient", Value::Byte(0)), ("Amplifier", Value::Byte(amplifier)), a, b, c, d, ("Id", Value::Byte(id)), ("ShowParticles", Value::Byte(particles))])
}

fn kept() -> Vec<u8> {
    let effects = Value::List(List { tag: 10, items: vec![effect(1, 1, 1900, 1), effect(11, 2, 3920, 0)] });
    let mut actors = actor_with(-11, "minecraft:zombie", [4.5, 64.0, 4.5], vec![("ActiveEffects", effects)]);
    actors.extend(actor_with(-12, "minecraft:tnt", [2.5, 64.0, 12.5], vec![("Fuse", Value::Byte(120)), ("IsFuseLit", Value::Byte(1))]));
    let wither = vec![("AirAttack", Value::Byte(0)), ("Invul", Value::Int(0)), ("SpawningFrames", Value::Int(0)), ("Phase", Value::Int(0)), ("maxHealth", Value::Int(300))];
    actors.extend(actor_with(-13, "minecraft:wither", [12.5, 70.0, 12.5], wither));
    actors
}

#[test]
fn effects_a_lit_fuse_and_a_withers_armour_come_back_and_are_saved_as_they_went_on() {
    let storage = floor_with(kept());
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    let sent = enter_stored(&mut h, 1);
    h.remote.run(|ctx| ctx.set_game_mode(sapling_api::PlayerId(1), sapling_api::GameMode::Creative));
    let kinds: Vec<String> = all::<AddEntity>(&sent).into_iter().map(|added| added.entity_type).collect();
    assert!(["minecraft:zombie", "minecraft:tnt", "minecraft:wither"].iter().all(|kind| kinds.iter().any(|shown| shown == kind)), "{kinds:?}");
    (0..20).for_each(|_| drop(h.tick(1)));
    h.shard.save_worlds();
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let saved = stored_actors(&column.actors).unwrap();
    let of = |kind: &str| saved.iter().find(|(actor, ..)| actor.as_ref().is_some_and(|actor| actor.identifier == kind)).map(|(.., compound)| compound.clone()).unwrap_or_else(|| panic!("no {kind} saved"));
    let effects = actor_effects(&of("minecraft:zombie"));
    assert_eq!(effects.iter().map(|[id, amplifier, _, shown]| [*id, *amplifier, *shown]).collect::<Vec<_>>(), [[1, 1, 1], [11, 2, 0]]);
    // Counted down from the column's arrival: the 20 ticks after it, and the join's own.
    assert!(effects.iter().zip([1900, 3920]).all(|([.., left, _], was)| (was - 60..=was - 20).contains(left)), "{effects:?}");
    let fuse = actor_fuse(&of("minecraft:tnt")).expect("a fuse");
    assert!((60..=100).contains(&fuse), "{fuse}");
    assert_eq!(wither_record(&of("minecraft:wither")), Some(WitherRecord { charging: 0, armoured: true, max_health: 300 }));
}

#[test]
fn a_summoned_wither_is_saved_charging_with_its_difficultys_health() {
    let storage = floor_with(Vec::new());
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| {
        ctx.set_game_mode(sapling_api::PlayerId(1), sapling_api::GameMode::Creative);
        ctx.set_difficulty(sapling_api::Difficulty::Easy);
        assert!(ctx.spawn_entity("minecraft:wither", [8.5, 64.0, 8.5]).is_some());
    });
    (0..10).for_each(|_| drop(h.tick(1)));
    h.shard.save_worlds();
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let record = stored_actors(&column.actors).unwrap().iter().find_map(|(.., compound)| wither_record(compound)).expect("a wither");
    assert!((185..=195).contains(&record.charging) && !record.armoured && record.max_health == 300, "{record:?}");
}
