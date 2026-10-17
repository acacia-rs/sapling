use std::sync::Arc;

use acacia_world::{BlockRegistry, Chunk, Dimension};
use sapling_worldgen::{Block, MIN_Y, OverworldGenerator};

use super::Generated;
use crate::{ChunkPos, MemoryStorage, StoredColumn, World, WorldStorage};

const SEED: i64 = 20261003;

fn stone_column(pos: ChunkPos, dimension: i32) -> Chunk {
    let registry = BlockRegistry::vanilla();
    let mut chunk = Chunk::empty(pos.x, pos.z, Dimension::from_id(dimension, registry.air_id()));
    chunk.fill_section(4, registry.find("minecraft:stone", "").unwrap());
    chunk
}

#[test]
fn inner_columns_come_first_and_only_overworld_holes_are_generated() {
    let (stored, hole) = (ChunkPos::new(0, 0), ChunkPos::new(1, 0));
    let inner = MemoryStorage::default();
    inner.insert(0, stone_column(stored, 0));
    inner.insert(1, stone_column(hole, 1));
    let generated = Generated::new(inner, SEED);

    assert_eq!(generated.load_column(stored, 0).unwrap(), Some(StoredColumn::of(stone_column(stored, 0))));
    let filled = generated.load_column(hole, 0).unwrap().expect("a hole is generated");
    assert_eq!(filled, generated.generate_column(hole));
    let bedrock = BlockRegistry::vanilla().find("minecraft:bedrock", "infiniburn_bit=0").unwrap();
    assert_eq!(filled.chunk.block(16, MIN_Y, 0), bedrock);
    assert_eq!(generated.load_column(hole, 1).unwrap(), Some(StoredColumn::of(stone_column(hole, 1))));
    assert_eq!(generated.load_column(stored, 1).unwrap(), None, "the nether stays void");
}

#[test]
fn generated_columns_are_saved_only_once_written() {
    let storage = Arc::new(Generated::new(MemoryStorage::default(), SEED));
    let mut world = World::with_storage(storage.clone());
    let (untouched, written) = (ChunkPos::new(0, 0), ChunkPos::new(1, 0));
    world.load_now(untouched);
    world.load_now(written);
    let stone = world.block_id("minecraft:stone").unwrap();
    assert!(world.set_block(16, 300, 0, stone));
    assert_eq!(world.save_and_wait(), 0);
    assert_eq!(storage.inner().columns(0).unwrap(), vec![written]);
}

#[test]
fn an_oak_tree_feature_grows_over_the_loaded_columns() {
    let mut world = World::void();
    let dirt = world.block_id("minecraft:dirt").unwrap();
    world.fill([-8, 63, -8], [8, 63, 8], dirt);
    let writes = world.place_feature("minecraft:oak_tree_feature", [0, 64, 0], 7).expect("an oak");
    let name = |id: u32| world.registry().get(id).map(|state| state.name).unwrap_or_default();
    assert!(writes.iter().any(|(pos, id)| *pos == [0, 64, 0] && name(*id) == "minecraft:oak_log"), "{writes:?}");
    assert!(writes.iter().any(|(pos, id)| pos[1] > 66 && name(*id) == "minecraft:oak_leaves"));
    assert!(world.place_feature("minecraft:no_such_feature", [0, 64, 0], 7).is_none());
}

#[test]
fn a_generated_world_without_a_spawn_spawns_on_the_surface_above_the_origin() {
    let mut world = World::with_storage(Arc::new(Generated::new(MemoryStorage::default(), SEED)));
    let shape = OverworldGenerator::new(SEED).chunk_blocks(0, 0);
    let top = (MIN_Y..320).rev().find(|&y| shape.block(0, y, 0) != Block::Air).unwrap();
    assert_eq!(world.load_spawn(), [0.5, top as f32 + 1.0, 0.5]);
}
