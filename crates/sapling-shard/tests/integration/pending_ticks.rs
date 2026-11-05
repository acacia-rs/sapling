//! Block ticks kept with their column, as BDS keeps `PendingTicks` (blocks/pending.rs).

use crate::common;

use common::stored::{enter_stored, floor_holding};
use common::{Harness, config};
use sapling_world::{BlockRegistry, ChunkPos, World, WorldStorage, decode_pending_ticks, encode_pending_ticks};

#[test]
fn water_a_column_kept_waiting_runs_when_it_comes_in_and_is_kept_waiting_again() {
    let registry = BlockRegistry::vanilla();
    let water = |depth: u8| registry.find("minecraft:water", &format!("liquid_depth={depth}")).unwrap();
    let storage = floor_holding(Vec::new(), &[([3, 64, 3], "minecraft:water"), ([12, 64, 12], "minecraft:water")]);
    let mut column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().unwrap();
    // Only the first source was waiting; water set down stands still until something wakes it.
    column.pending_ticks = encode_pending_ticks(registry, 1_000, &[([3, 64, 3], water(0), 1_005)]);
    storage.store_column(ChunkPos::new(0, 0), 0, &column).unwrap();

    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    (0..12).for_each(|_| _ = h.tick(1));
    let (sent, got) = std::sync::mpsc::channel();
    // It runs toward the void beyond the column's low edges, a drop within its reach.
    h.remote.run(move |ctx| sent.send([[2, 64, 3], [3, 64, 2], [14, 64, 12]].map(|at| ctx.block(at).map(|block| block.name))).expect("the test waits"));
    h.tick(1);
    let names = got.recv().expect("ran");
    h.shard.save_worlds();
    let kept = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().unwrap().pending_ticks;
    let (_, ticks) = decode_pending_ticks(&kept).unwrap();
    let water_name = Some("minecraft:water".to_owned());
    assert!(names[0] == water_name && names[1] == water_name, "the kept tick ran: {names:?}, kept {ticks:?}");
    assert_eq!(names[2].as_deref(), Some("minecraft:air"), "the other source was not waiting");
    assert!(!ticks.is_empty() && ticks.iter().all(|tick| tick.name == "minecraft:water" && tick.time > 0), "the running water waits again: {ticks:?}");
}

/// A kept tick is due by the level's clock, not by how far it was from its save: BDS ran a generated spring's
/// ticks (dated 85 to 369 from 0) at once in a world 192 ticks old (docs/research/world-ticks.md "Potent sulfur").
#[test]
fn water_a_kept_tick_is_due_by_the_level_s_clock() {
    let registry = BlockRegistry::vanilla();
    let water = registry.find("minecraft:water", "liquid_depth=0").unwrap();
    let storage = floor_holding(Vec::new(), &[([3, 64, 3], "minecraft:water")]);
    let mut column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().unwrap();
    // Saved at 1000, due at 1300: 300 ticks off when saved, 10 from the level's 1290.
    column.pending_ticks = encode_pending_ticks(registry, 1_000, &[([3, 64, 3], water, 1_300)]);
    storage.store_column(ChunkPos::new(0, 0), 0, &column).unwrap();
    let mut level = World::with_storage(storage.clone());
    level.level_mut().age = 1_290;
    let mut h = Harness::with(config(), level);
    enter_stored(&mut h, 1);
    (0..30).for_each(|_| _ = h.tick(1));
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(ctx.block([2, 64, 3]).map(|block| block.name)).expect("the test waits"));
    h.tick(1);
    assert_eq!(got.recv().expect("ran").as_deref(), Some("minecraft:water"));
}

/// A kept tick that has not come due is saved again at the time it was loaded with, however long the
/// column was in and whether the save comes in a tick or between two.
#[test]
fn water_a_kept_tick_keeps_its_time_across_a_load_and_a_save() {
    let registry = BlockRegistry::vanilla();
    let water = registry.find("minecraft:water", "liquid_depth=0").unwrap();
    let storage = floor_holding(Vec::new(), &[([3, 64, 3], "minecraft:water")]);
    let mut column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().unwrap();
    column.pending_ticks = encode_pending_ticks(registry, 1_000, &[([3, 64, 3], water, 5_000)]);
    storage.store_column(ChunkPos::new(0, 0), 0, &column).unwrap();
    let mut level = World::with_storage(storage.clone());
    level.level_mut().age = 1_000;
    let mut h = Harness::with(config(), level);
    enter_stored(&mut h, 1);
    let kept = |h: &mut Harness| {
        h.shard.save_worlds();
        let (saved_at, ticks) = decode_pending_ticks(&storage.load_column(ChunkPos::new(0, 0), 0).unwrap().unwrap().pending_ticks).unwrap();
        (saved_at, ticks.iter().map(|tick| tick.time).collect::<Vec<_>>())
    };
    (0..25).for_each(|_| _ = h.tick(1));
    let (first_at, first) = kept(&mut h);
    (0..40).for_each(|_| _ = h.tick(1));
    let (then_at, then) = kept(&mut h);
    assert_eq!((first, then), (vec![5_000], vec![5_000]), "saved at {first_at} and {then_at}");
    assert_eq!(then_at - first_at, 40);
}
