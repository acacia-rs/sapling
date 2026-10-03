//! The fixture world (tests/fixtures/README.md) read back: what a BDS 1.26.52 save holds, checked
//! against what is known about the terrain it generated.

use std::path::{Path, PathBuf};

use acacia_world::BlockRegistry;
use sapling_leveldb::{ChunkKey, LevelDbWorld, PLACEHOLDER, SubChunk, Tag};

/// The spawn block column, where the bot that generated the world stood on grass at y 72.
const SPAWN: (i32, i32) = (21, 205);
const GROUND: i32 = 71;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/world")
}

fn world() -> LevelDbWorld {
    LevelDbWorld::open_in_memory(&fixture()).expect("the fixture opens")
}

fn name(registry: &BlockRegistry, id: u32) -> &'static str {
    registry.get(id).map_or("?", |state| state.name)
}

#[test]
fn level_dat_names_the_world_and_its_spawn() {
    let world = world();
    let level = world.level();
    assert_eq!((level.storage_version, level.name(), level.seed()), (10, Some("sapling-hub"), Some(20261003)));
    assert_eq!(level.spawn(), Some([SPAWN.0, 32767, SPAWN.1]), "y is left to the surface");
}

#[test]
fn the_columns_around_spawn_are_saved_in_the_current_format() {
    let world = world();
    let columns = world.columns().unwrap();
    assert_eq!(columns.len(), 25);
    assert!(columns.iter().all(|c| c.dimension == 0 && (-1..=3).contains(&c.x) && (10..=14).contains(&c.z)), "{columns:?}");
    for key in columns {
        let column = world.load_column(key).unwrap().expect("a listed column");
        assert_eq!(column.version, 42);
        // Version 9 records carry the section index the key has.
        for y in -4..20 {
            let Some(record) = world.db().get(&key.sub_chunk(y)).unwrap() else { continue };
            assert_eq!((record[0], record[2] as i8), (9, y), "{key:?}");
            assert!(record[1] <= 2);
        }
    }
    assert_eq!(world.load_column(ChunkKey::new(40, 40, 0)).unwrap(), None, "outside the fixture");
    assert_eq!(world.load_column(ChunkKey::new(1, 12, 1)).unwrap(), None, "no nether was saved");
}

#[test]
fn blocks_match_the_generated_terrain() {
    let world = world();
    let registry = world.registry().clone();
    let placeholder = registry.states_of(PLACEHOLDER).next().unwrap().0;
    for key in world.columns().unwrap() {
        let chunk = world.load_column(key).unwrap().unwrap().chunk;
        assert!(chunk.palette_ids().all(|id| id != placeholder), "every saved state resolves");
        for (x, z) in [(0, 0), (15, 0), (7, 9), (15, 15)] {
            assert_eq!(name(&registry, chunk.block(x, -64, z)), "minecraft:bedrock", "{key:?}");
            assert_eq!(chunk.block(x, 319, z), registry.air_id());
        }
        // Bedrock thins out over the bottom five layers; deepslate and what cuts through it follow.
        let deep: Vec<&str> = (-59..-50).map(|y| name(&registry, chunk.block(3, y, 3))).collect();
        assert!(deep.iter().all(|n| n.contains("deepslate") || ["minecraft:tuff", "minecraft:air", "minecraft:lava", "minecraft:gravel"].contains(n)), "{deep:?}");
    }
    let spawn = world.load_column(ChunkKey::new(SPAWN.0 >> 4, SPAWN.1 >> 4, 0)).unwrap().unwrap().chunk;
    let column: Vec<&str> = (GROUND - 1..=GROUND + 1).map(|y| name(&registry, spawn.block(SPAWN.0, y, SPAWN.1))).collect();
    assert_eq!(column, ["minecraft:dirt", "minecraft:grass_block", "minecraft:air"]);
    assert!(spawn.biome(SPAWN.0, GROUND, SPAWN.1).is_some());
}

#[test]
fn the_saved_heightmap_is_the_top_light_dimming_block_of_every_column() {
    let world = world();
    let registry = world.registry().clone();
    for key in world.columns().unwrap() {
        let column = world.load_column(key).unwrap().unwrap();
        let data = column.data3d.expect("a Data3D record");
        for (x, z) in (0..16).flat_map(|x| (0..16).map(move |z| (x, z))) {
            // Plants do not count, water does: the blocks that dim light.
            let dims_light = |y: &i32| registry.get(column.chunk.block(x, *y, z)).is_some_and(|state| state.light_filter > 0);
            let top = (-64..320).rev().find(dims_light);
            assert_eq!(data.top(x, z, -64), top, "{key:?} ({x}, {z}): {:?}", top.map(|y| name(&registry, column.chunk.block(x, y, z))));
        }
    }
    let spawn = world.load_column(ChunkKey::new(SPAWN.0 >> 4, SPAWN.1 >> 4, 0)).unwrap().unwrap();
    assert_eq!(spawn.data3d.unwrap().top(SPAWN.0, SPAWN.1, -64), Some(GROUND));
}

#[test]
fn opening_in_place_reads_the_same_and_in_memory_leaves_the_folder_alone() {
    let listing = |dir: &Path| {
        let mut files: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap()).map(|e| (e.file_name(), e.metadata().unwrap().len())).collect();
        files.sort();
        files
    };
    let before = listing(&fixture().join("db"));
    let key = ChunkKey::new(1, 12, 0);
    let in_memory = world().load_column(key).unwrap().unwrap();
    assert_eq!(listing(&fixture().join("db")), before);

    let copy = std::env::temp_dir().join(format!("sapling-leveldb-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&copy);
    std::fs::create_dir_all(copy.join("db")).unwrap();
    for (file, _) in &before {
        std::fs::copy(fixture().join("db").join(file), copy.join("db").join(file)).unwrap();
    }
    std::fs::copy(fixture().join("level.dat"), copy.join("level.dat")).unwrap();
    let on_disk = LevelDbWorld::open(&copy).unwrap();
    assert_eq!(on_disk.load_column(key).unwrap().unwrap(), in_memory);
    assert!(on_disk.db().get(&key.tag(Tag::Version)).unwrap().is_some());
    let section = SubChunk::decode(&on_disk.db().get(&key.sub_chunk(4)).unwrap().unwrap(), &sapling_leveldb::BlockStates::new(registry())).unwrap();
    assert_eq!(section.y, Some(4));
    drop(on_disk);
    std::fs::remove_dir_all(&copy).unwrap();
}

fn registry() -> std::sync::Arc<BlockRegistry> {
    BlockRegistry::vanilla_arc()
}
