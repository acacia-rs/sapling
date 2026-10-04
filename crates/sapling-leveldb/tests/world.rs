//! The fixture world (tests/fixtures/README.md) read back: what a BDS 1.26.52 save holds, checked
//! against what is known about the terrain it generated.

use std::path::{Path, PathBuf};

use acacia_world::BlockRegistry;
use acacia_proto::nbt::{self, List, LittleEndian, Nbt, Value};
use bytes::BytesMut;
use sapling_leveldb::{ChunkKey, LevelDat, LevelDbWorld, PLACEHOLDER, SubChunk, Tag, actors};

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
    // The types these accessors expect are the ones BDS writes.
    assert!(level.time().is_some() && level.long("LastPlayed").is_some() && level.daylight_cycle());
    assert_eq!((level.game_type(), level.difficulty()), (Some(0), Some(0)), "survival, peaceful");
    assert!(matches!(level.root.get("dodaylightcycle"), Some(Value::Byte(1))));
    assert!(matches!(level.root.get("lastOpenedWithVersion"), Some(Value::List(List { tag: 3, items })) if items.len() == 5));
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

    let copy = copy_of_fixture("open");
    let on_disk = LevelDbWorld::open(&copy).unwrap();
    assert_eq!(on_disk.load_column(key).unwrap().unwrap(), in_memory);
    assert!(on_disk.db().get(&key.tag(Tag::Version)).unwrap().is_some());
    let section = SubChunk::decode(&on_disk.db().get(&key.sub_chunk(4)).unwrap().unwrap(), &sapling_leveldb::BlockStates::new(registry())).unwrap();
    assert_eq!(section.y, Some(4));
    drop(on_disk);
    std::fs::remove_dir_all(&copy).unwrap();
}

#[test]
fn stored_columns_read_back_and_carry_what_bds_writes() {
    let world = world();
    let states = sapling_leveldb::BlockStates::new(registry());
    let stone = registry().find("minecraft:stone", "").unwrap();
    for key in world.columns().unwrap() {
        let saved = world.load_column(key).unwrap().unwrap();
        let records = |y: i8| (world.db().get(&key.sub_chunk(y)).unwrap(), world.db().get(&key.tag(Tag::Data3d)).unwrap());
        let before: Vec<_> = (-4..20).map(records).collect();
        world.store_column(key, &saved.chunk, &saved.block_entities, &saved.actors).unwrap();
        let reread = world.load_column(key).unwrap().unwrap();
        assert!((reread.version, &reread.block_entities) == (saved.version, &saved.block_entities), "{key:?}");
        // BDS orders palettes its own way, so blocks and biomes are compared decoded.
        assert!((0..24).all(|index| section(&reread.chunk, index) == section(&saved.chunk, index)), "{key:?}");
        // The heightmap is not kept but recomputed: it must come out as BDS wrote it.
        assert!(reread.data3d.unwrap().heightmap == saved.data3d.unwrap().heightmap, "{key:?}");
        let (ours, theirs) = (world.db().get(&key.tag(Tag::Data3d)).unwrap().unwrap(), before[0].1.clone().unwrap());
        assert_eq!((ours.len() > 512, ours[512] & 1), (true, theirs[512] & 1), "the biome header bit");
        for (y, (section, _)) in (-4..20).zip(&before) {
            let rewritten = world.db().get(&key.sub_chunk(y)).unwrap();
            let decode = |record: &Option<Vec<u8>>| record.as_ref().map(|r| SubChunk::decode(r, &states).unwrap());
            assert_eq!(decode(&rewritten), decode(section), "{key:?} section {y}");
            if let (Some(ours), Some(theirs)) = (&rewritten, section) {
                assert_eq!(ours[..3], theirs[..3], "version, layer count and y");
                assert_eq!(ours[ours.len() - 5..], theirs[theirs.len() - 5..], "the last entry's version");
            }
        }
    }
    // A column storage never held, and a write to one it does.
    let (new, old) = (ChunkKey::new(40, 40, 0), ChunkKey::new(1, 12, 0));
    let mut chunk = acacia_world::Chunk::empty(40, 40, acacia_world::Dimension::overworld(registry().air_id()));
    chunk.set(3, 70, 9, 0, stone);
    world.store_column(new, &chunk, &[], &[]).unwrap();
    let stored = world.load_column(new).unwrap().unwrap();
    assert_eq!((stored.version, stored.chunk.block(3, 70, 9), stored.data3d.unwrap().top(3, 9, -64)), (42, stone, Some(70)));
    assert_eq!(world.db().get(&new.sub_chunk(3)).unwrap(), None, "sections of air are not written");

    let mut edited = world.load_column(old).unwrap().unwrap();
    edited.chunk.set(5, 300, 5, 0, stone);
    world.store_column(old, &edited.chunk, b"kept as given", &[]).unwrap();
    let reread = world.load_column(old).unwrap().unwrap();
    assert_eq!((reread.chunk.block(5, 300, 5), reread.data3d.unwrap().top(5, 5, -64)), (stone, Some(300)));
    assert_eq!(reread.block_entities, b"kept as given");
    world.store_column(old, &edited.chunk, &[], &[]).unwrap();
    assert_eq!(world.db().get(&old.tag(Tag::BlockEntities)).unwrap(), None, "an empty record is not written");

    let level = std::fs::read(fixture().join("level.dat")).unwrap();
    world.flush(world.level()).unwrap();
    assert_eq!(std::fs::read(fixture().join("level.dat")).unwrap(), level, "a world in memory leaves its folder alone");
}

fn actor(id: i64, identifier: &str) -> Vec<u8> {
    let floats = |items: &[f32]| Value::List(List { tag: 5, items: items.iter().map(|&item| Value::Float(item)).collect() });
    let fields = [("UniqueID", Value::Long(id)), ("identifier", Value::String(identifier.into())), ("Pos", floats(&[20.5, 72.0, 200.5])), ("Rotation", floats(&[45.0, 0.0]))];
    let mut out = BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: Value::Compound(fields.map(|(key, value)| (key.to_string(), value)).into()) });
    out.to_vec()
}

#[test]
fn actors_are_stored_by_id_and_listed_by_their_column() {
    let world = world();
    let key = ChunkKey::new(1, 12, 0);
    let column = world.load_column(key).unwrap().unwrap();
    assert!(column.actors.is_empty(), "the fixture holds terrain only");
    let (cow, pig) = (actor(-4_294_967_295, "minecraft:cow"), actor(7, "minecraft:pig"));
    let store = |actors: &[u8]| world.store_column(key, &column.chunk, &column.block_entities, actors).unwrap();
    let stored = |id: i64| world.db().get(&[&b"actorprefix"[..], &id.to_le_bytes()].concat()).unwrap();

    store(&[&cow[..], &pig].concat());
    assert_eq!(world.db().get(&key.actor_digest()).unwrap().unwrap(), [(-4_294_967_295i64).to_le_bytes(), 7i64.to_le_bytes()].concat());
    assert_eq!((stored(-4_294_967_295), stored(7)), (Some(cow.clone()), Some(pig.clone())));
    let found = actors(&world.load_column(key).unwrap().unwrap().actors).unwrap();
    assert_eq!(found.iter().map(|a| (a.unique_id, a.identifier.as_str(), a.position, a.yaw)).collect::<Vec<_>>(), [
        (-4_294_967_295, "minecraft:cow", [20.5, 72.0, 200.5], 45.0),
        (7, "minecraft:pig", [20.5, 72.0, 200.5], 45.0),
    ]);

    store(&pig);
    assert_eq!((stored(-4_294_967_295), stored(7)), (None, Some(pig.clone())), "an actor that left the column is deleted");
    store(&[]);
    assert_eq!((stored(7), world.load_column(key).unwrap().unwrap().actors), (None, Vec::new()));

    // Before 1.18.30 a column's actors were one record under its own tag.
    let old = ChunkKey::new(2, 12, 0);
    world.db().put(&old.tag(Tag::Entity), &cow).unwrap();
    let legacy = world.load_column(old).unwrap().unwrap();
    assert_eq!(legacy.actors, cow);
    world.store_column(old, &legacy.chunk, &legacy.block_entities, &[&cow[..], &pig].concat()).unwrap();
    assert_eq!((world.db().get(&old.tag(Tag::Entity)).unwrap(), stored(7)), (None, Some(pig)), "rewritten the current way");
}

#[test]
fn what_is_stored_and_flushed_is_there_when_the_folder_opens_again() {
    let copy = copy_of_fixture("flush");
    let (key, stone) = (ChunkKey::new(1, 12, 0), registry().find("minecraft:stone", "").unwrap());
    let world = LevelDbWorld::open(&copy).unwrap();
    let mut column = world.load_column(key).unwrap().unwrap();
    column.chunk.set(5, 300, 5, 0, stone);
    world.store_column(key, &column.chunk, &column.block_entities, &column.actors).unwrap();
    let mut level = world.level().clone();
    level.set_last_played(1_800_000_000);
    level.set_spawn([3, 70, -4]);
    level.set_time(30_000);
    level.set_last_opened_with([1, 26, 52, 0, 0]);
    world.flush(&level).unwrap();
    drop(world);

    let reopened = LevelDbWorld::open(&copy).unwrap();
    assert_eq!(reopened.load_column(key).unwrap().unwrap().chunk.block(5, 300, 5), stone);
    assert_eq!(reopened.level(), &level);
    assert_eq!((reopened.level().name(), reopened.level().spawn(), reopened.level().time()), (Some("sapling-hub"), Some([3, 70, -4]), Some(30_000)));
    assert_eq!(reopened.level().long("LastPlayed"), Some(1_800_000_000));
    let old = LevelDat::read(&copy.join("level.dat_old")).unwrap();
    assert_eq!((old.spawn(), old.long("LastPlayed") == Some(1_800_000_000)), (Some([SPAWN.0, 32767, SPAWN.1]), false), "the file it replaced");
    drop(reopened);
    std::fs::remove_dir_all(&copy).unwrap();
}

/// A copy of the fixture that may be written to; the caller removes it.
fn copy_of_fixture(name: &str) -> PathBuf {
    let copy = std::env::temp_dir().join(format!("sapling-leveldb-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&copy);
    std::fs::create_dir_all(copy.join("db")).unwrap();
    for entry in std::fs::read_dir(fixture().join("db")).unwrap() {
        let file = entry.unwrap().file_name();
        std::fs::copy(fixture().join("db").join(&file), copy.join("db").join(&file)).unwrap();
    }
    std::fs::copy(fixture().join("level.dat"), copy.join("level.dat")).unwrap();
    copy
}

/// Both layers and the biomes of section `index`; a section never stored reads as the air it holds.
fn section(chunk: &acacia_world::Chunk, index: usize) -> Vec<u32> {
    let air = registry().air_id();
    let (mut blocks, mut liquid, mut biomes) = ([air; acacia_world::SECTION_VOLUME], [air; acacia_world::SECTION_VOLUME], [0; acacia_world::SECTION_VOLUME]);
    chunk.copy_section(index, &mut blocks, &mut liquid);
    assert!(chunk.copy_biomes(index, &mut biomes));
    [blocks, liquid, biomes].concat()
}

fn registry() -> std::sync::Arc<BlockRegistry> {
    BlockRegistry::vanilla_arc()
}
