//! Phantoms come to players who have not slept (docs/research/mob-spawning.md "Phantoms").

use crate::common;

use std::sync::Arc;

use common::stored::enter_stored;
use common::{Harness, config};
use sapling_api::{Difficulty, EntityInfo};
use sapling_world::{BlockRegistry, Chunk, ChunkPos, Dimension, MemoryStorage, PlayerData, StoredColumn, World, WorldStorage};

const MIDNIGHT: i64 = 18000;

/// Steve in creative on a stone floor across 8 by 8 columns at midnight, `rested` ticks since it slept.
fn sleepless(rested: Option<i32>) -> Harness {
    let registry = BlockRegistry::vanilla();
    let stone = registry.states_of("minecraft:stone").next().unwrap().0;
    let storage = Arc::new(MemoryStorage::default());
    for (x, z) in (-4..4).flat_map(|x| (-4..4).map(move |z| (x, z))) {
        let mut chunk = Chunk::empty(x, z, Dimension::overworld(registry.air_id()));
        (0..16).for_each(|x| (0..16).for_each(|z| chunk.set(x, 63, z, 0, stone)));
        storage.store_column(ChunkPos::new(x, z), 0, &StoredColumn::of(chunk)).unwrap();
    }
    let steve = PlayerData { rested, position: Some([0.5, 64.0, 0.5]), game_mode: Some(1), ..PlayerData::default() };
    storage.store_player(&acacia_auth::login::offline_identity("Steve"), &steve).unwrap();
    let mut h = Harness::with(config(), World::with_storage(storage));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Normal);
        ctx.set_time(MIDNIGHT);
    });
    h
}

/// The phantoms there once `ticks` have passed, looked for every 100 ticks; the other monsters are
/// taken away each time, so that their cap leaves room.
fn phantoms_within(h: &mut Harness, ticks: usize) -> Vec<EntityInfo> {
    for _ in 0..ticks / 100 {
        (0..100).for_each(|_| drop(h.tick_all()));
        let found = common::asking::ask(h, |ctx| {
            let (phantoms, others): (Vec<EntityInfo>, Vec<EntityInfo>) = ctx.entities().into_iter().partition(|entity| entity.kind == "minecraft:phantom");
            for other in others {
                ctx.remove_entity(other.id);
            }
            phantoms
        });
        if !found.is_empty() {
            return found;
        }
    }
    Vec::new()
}

#[test]
fn a_phantom_comes_20_to_34_over_a_player_awake_three_days() {
    let mut h = sleepless(Some(i32::MAX));
    let found = phantoms_within(&mut h, 3000);
    let phantom = found.first().expect("a phantom");
    let [x, y, z] = phantom.feet;
    assert!((84.0..99.0).contains(&y) && (x - 0.5).abs() <= 11.0 && (z - 0.5).abs() <= 11.0, "{phantom:?}");
}

#[test]
fn none_comes_to_a_player_who_slept() {
    let mut h = sleepless(None);
    assert_eq!(phantoms_within(&mut h, 1500), []);
}
