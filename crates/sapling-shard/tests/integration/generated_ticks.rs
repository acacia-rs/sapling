//! Liquid ticks a generated column's queue still holds run once it comes in (sapling-world generated/ticks.rs).

use std::sync::Arc;

use crate::common;

use common::stored::enter_stored;
use common::{Harness, config};
use sapling_api::PlayerId;
use sapling_world::{ChunkPos, Generated, MemoryStorage, StoredColumn, World, WorldStorage, decode_pending_ticks};

const SEED: i64 = 20261003;
/// Seed 20261003's chunk (5, 4) holds seven water ticks a later neighbour's flow left (sapling-worldgen liquid/leftovers.rs).
const LEFT: ChunkPos = ChunkPos { x: 5, z: 4 };

/// The generated columns as storage, with or without the ticks they came with.
fn stored(generated: &[(ChunkPos, StoredColumn)], keep_ticks: bool) -> Arc<MemoryStorage> {
    let storage = Arc::new(MemoryStorage::default());
    for (pos, column) in generated {
        let pending_ticks = if keep_ticks { column.pending_ticks.clone() } else { Vec::new() };
        storage.store_column(*pos, 0, &StoredColumn { pending_ticks, ..column.clone() }).unwrap();
    }
    storage
}

/// The block names at `cells` once a player has stood over `LEFT` for `ticks` ticks.
fn blocks_after(storage: Arc<MemoryStorage>, ticks: usize, cells: Vec<[i32; 3]>) -> Vec<Option<String>> {
    let mut h = Harness::with(config(), World::with_storage(storage));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| ctx.teleport(PlayerId(1), [LEFT.x as f32 * 16.0 + 8.5, 120.0, LEFT.z as f32 * 16.0 + 8.5]));
    (0..ticks).for_each(|_| _ = h.tick(1));
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(cells.iter().map(|&at| ctx.block(at).map(|block| block.name)).collect::<Vec<_>>()).expect("the test waits"));
    h.tick(1);
    got.recv().expect("ran")
}

#[test]
fn liquid_a_generated_column_was_left_waiting_on_flows_once_it_comes_in() {
    let generator = Generated::with_areas(MemoryStorage::default(), SEED, 1);
    let around = (LEFT.x - 1..=LEFT.x + 1).flat_map(|x| (LEFT.z - 1..=LEFT.z + 1).map(move |z| ChunkPos::new(x, z)));
    let generated: Vec<(ChunkPos, StoredColumn)> = around.map(|pos| (pos, generator.generate_column(pos))).collect();
    let waiting: Vec<[i32; 3]> = generated.iter().flat_map(|(_, column)| decode_pending_ticks(&column.pending_ticks).unwrap().1).map(|tick| tick.pos).collect();
    assert!(!waiting.is_empty(), "the columns round {LEFT:?} were left with liquid ticks");
    let cells: Vec<[i32; 3]> = waiting.iter().flat_map(|&[x, y, z]| [[x - 1, y, z], [x + 1, y, z], [x, y - 1, z], [x, y, z - 1], [x, y, z + 1]]).collect();
    let (ticked, still) = (blocks_after(stored(&generated, true), 40, cells.clone()), blocks_after(stored(&generated, false), 40, cells.clone()));
    let liquid = |name: &Option<String>| name.as_deref().is_some_and(|name| name.ends_with("water") || name.ends_with("lava"));
    let flowed = cells.iter().zip(ticked.iter().zip(&still)).filter(|(_, (ticked, still))| ticked != still && liquid(ticked)).count();
    assert!(flowed > 0, "liquid spread beside the waiting cells {waiting:?} only with their ticks: {ticked:?} against {still:?}");
}
