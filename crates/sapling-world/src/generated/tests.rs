use std::sync::Arc;

use acacia_proto::nbt::Value;
use acacia_world::{BlockRegistry, Chunk, Dimension};
use sapling_worldgen::{Block, EndGenerator, MIN_Y, NetherGenerator, OverworldGenerator};

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
fn only_a_generated_overworld_locates_strongholds() {
    let overworld = World::with_storage(Arc::new(Generated::with_areas(MemoryStorage::default(), SEED, 1)));
    assert_eq!(overworld.locate_stronghold(0, 0), Some([-1422, 32, -1950]), "BDS's /locate answer");
    let nether = World::with_storage_in(Arc::new(Generated::nether(MemoryStorage::default(), SEED, 1)), crate::NETHER);
    assert_eq!(nether.locate_stronghold(0, 0), None);
    assert_eq!(World::void().locate_stronghold(0, 0), None);
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
fn the_nether_is_generated_in_its_own_dimension_128_high() {
    let registry = BlockRegistry::vanilla();
    let generated = Generated::nether(MemoryStorage::default(), SEED, 1);
    let pos = ChunkPos::new(2, -1);
    assert_eq!(generated.load_column(pos, 0).unwrap(), None, "the overworld stays void");
    let column = generated.load_column(pos, 1).unwrap().expect("a nether hole is generated");
    let (chunk, shape) = (&column.chunk, NetherGenerator::new(SEED).chunk_blocks(pos.x, pos.z));
    assert_eq!(chunk.dimension(), Dimension::nether(registry.air_id()));
    for (x, z) in [(0, 0), (5, 11), (15, 15)] {
        for y in 0..128 {
            let state = registry.get(chunk.block(x as i32, y, z as i32)).unwrap();
            assert_eq!(state.name.strip_prefix("minecraft:"), Some(shape.block(x, y, z).name()), "at {x} {y} {z}");
        }
    }
    assert_eq!(registry.get(chunk.block(3, 0, 3)).unwrap().name, "minecraft:bedrock");
    let biomes = NetherGenerator::new(SEED).chunk_biomes(pos.x, pos.z);
    assert_eq!((chunk.biome(4, 64, 9), chunk.biome(4, 127, 9)), (Some(biomes.biome(4, 64, 9)), Some(biomes.biome(4, 127, 9))));

    let mut world = World::with_storage_in(Arc::new(generated), 1);
    world.load_now(pos);
    assert_eq!((world.dimension(), world.heights()), (1, 0..128));
    assert_eq!(world.level_chunk(pos, false).unwrap().highest_subchunk_count, Some(8));
}

#[test]
fn the_end_is_generated_in_its_own_dimension_256_high() {
    let registry = BlockRegistry::vanilla();
    let generated = Generated::end(MemoryStorage::default(), SEED, 1);
    let pos = ChunkPos::new(0, 0);
    assert_eq!((generated.load_column(pos, 0).unwrap(), generated.load_column(pos, 1).unwrap()), (None, None));
    let column = generated.load_column(pos, 2).unwrap().expect("an end hole is generated");
    let (chunk, shape) = (&column.chunk, EndGenerator::new(SEED).chunk_blocks(pos.x, pos.z));
    assert_eq!(chunk.dimension(), Dimension::end(registry.air_id()));
    for (x, z) in [(0, 0), (5, 11), (15, 15)] {
        for y in 0..256 {
            let state = registry.get(chunk.block(x as i32, y, z as i32)).unwrap();
            assert_eq!(state.name.strip_prefix("minecraft:"), Some(shape.block(x, y, z).name()), "at {x} {y} {z}");
        }
    }
    assert_eq!(registry.get(chunk.block(0, 50, 0)).unwrap().name, "minecraft:end_stone");
    assert_eq!((chunk.biome(4, 0, 9), chunk.biome(4, 255, 9)), (Some(9), Some(9)));

    let mut world = World::with_storage_in(Arc::new(generated), 2);
    world.load_now(pos);
    assert_eq!((world.dimension(), world.heights()), (2, 0..256));
    // Sections up to the highest one holding a block, as BDS's End columns (research/captures/bds-end-travel.txt).
    let top = (0..256).rev().find(|&y| (0..16).any(|x| (0..16).any(|z| shape.block(x, y, z) != Block::Air))).unwrap();
    assert_eq!(world.level_chunk(pos, false).unwrap().highest_subchunk_count, Some(top / 16 + 1));
}

#[test]
fn a_generated_dungeon_saves_its_block_entities_as_bds() {
    // The dungeon at 123,-20,270, whose spawner and chest BDS saved in the oracle world 20261003-structures.
    let column = Generated::new(MemoryStorage::default(), SEED).generate_column(ChunkPos::new(7, 16));
    let entities = sapling_leveldb::block_entities(&column.block_entities).unwrap();
    let at = |pos: [i32; 3]| &entities.iter().find(|e| e.pos == pos).unwrap_or_else(|| panic!("nothing at {pos:?}: {entities:?}")).nbt;
    let text = |s: &str| Some(Value::String(s.into()));
    let spawner = at([123, -20, 270]);
    assert_eq!((spawner.get("id").cloned(), spawner.get("EntityIdentifier").cloned()), (text("MobSpawner"), text("minecraft:spider")));
    assert_eq!(spawner.get("Delay"), Some(&Value::Short(20)));
    let chest = at([125, -20, 270]);
    assert_eq!((chest.get("id").cloned(), chest.get("LootTable").cloned()), (text("Chest"), text("loot_tables/chests/simple_dungeon.json")));
    assert_eq!(chest.get("LootTableSeed"), Some(&Value::Int(357601146)));
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

/// The world of research/captures/bds-locate.txt.
const LOCATE_SEED: i64 = 12345;

#[test]
fn structures_are_located_where_bds_located_them() {
    let world = World::with_storage(Arc::new(Generated::with_areas(MemoryStorage::default(), LOCATE_SEED, 1)));
    let answers = [
        ("village", [-488, 248]),
        ("minecraft:stronghold", [1458, -830]),
        ("monument", [312, 1784]),
        ("ruins", [-168, -152]),
        ("shipwreck", [-216, -88]),
        ("ruined_portal", [120, 104]),
        ("ancient_city", [-312, -216]),
        ("pillager_outpost", [552, 136]),
        ("mansion", [-8584, 5592]),
        ("temple", [552, 648]),
        ("buried_treasure", [-184, 264]),
    ];
    let found: Vec<(&str, Option<[i32; 2]>)> = answers.iter().map(|(name, _)| (*name, world.locate_structure(name, 0, 0))).collect();
    assert_eq!(found, answers.map(|(name, at)| (name, Some(at))));
    assert_eq!([world.locate_structure("abandoned_camp", 0, 0), world.locate_structure("fortress", 0, 0)], [None, None]);
    assert_eq!(World::void().locate_structure("village", 0, 0), None);
}

#[test]
fn a_biome_is_located_on_a_32_block_grid() {
    let world = World::with_storage(Arc::new(Generated::with_areas(MemoryStorage::default(), LOCATE_SEED, 1)));
    let plains = sapling_data::biome_id("minecraft:plains").unwrap();
    let [x, _, z] = world.locate_biome(plains, [0, 120, 0]).expect("plains near the origin");
    assert_eq!((x % 32, z % 32), (0, 0));
    assert!(x * x + z * z <= 742 * 742, "BDS found plains at -736 -96: ({x}, {z})");
    assert_eq!(World::void().locate_biome(plains, [0, 120, 0]), None);
}

#[test]
fn a_generated_world_without_a_spawn_spawns_on_the_surface_above_the_origin() {
    let mut world = World::with_storage(Arc::new(Generated::new(MemoryStorage::default(), SEED)));
    let shape = OverworldGenerator::new(SEED).chunk_blocks(0, 0);
    let top = (MIN_Y..320).rev().find(|&y| shape.block(0, y, 0) != Block::Air).unwrap();
    assert_eq!(world.load_spawn(), [0.5, top as f32 + 1.0, 0.5]);
}

/// Decoration's leftover liquid ticks go out with their column, due at once (generated/ticks.rs).
#[test]
fn a_generated_column_keeps_the_liquid_ticks_its_queue_holds() {
    let generated = Generated::with_areas(MemoryStorage::default(), SEED, 1);
    let (saved_at, ticks) = crate::decode_pending_ticks(&generated.generate_column(ChunkPos::new(5, 4)).pending_ticks).unwrap();
    assert_eq!((ticks.len(), ticks[0].pos), (7, [88, -13, 79]), "sapling-worldgen's leftovers.rs test");
    assert!(ticks.iter().all(|tick| tick.name == "minecraft:water" && tick.time - saved_at == 1), "{ticks:?}");
    assert!(generated.generate_column(ChunkPos::new(0, 0)).pending_ticks.is_empty());
}
