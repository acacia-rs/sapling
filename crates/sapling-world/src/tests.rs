use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use acacia_proto::nbt::{self, LittleEndian, Nbt, Value};
use acacia_proto::types::{HeightMapDataType, SubChunkEntryItemResult};
use bytes::BytesMut;
use acacia_proto::xxh64;
use acacia_world::{Chunk, Dimension};

use crate::{ChunkPos, Level, MemoryStorage, StorageError, StoredColumn, World, WorldStorage};

fn platform() -> (World, u32) {
    let mut world = World::void();
    let stone = world.block_id("minecraft:stone").unwrap();
    world.fill([-8, 64, -8], [7, 64, 7], stone);
    (world, stone)
}

/// A storage-backed world whose only stored column, (2, 3), has stone at y 70 and below.
fn stored() -> (World, u32) {
    let storage = MemoryStorage::default();
    let registry = acacia_world::BlockRegistry::vanilla();
    let stone = registry.find("minecraft:stone", "").unwrap();
    let mut chunk = Chunk::empty(2, 3, Dimension::overworld(registry.air_id()));
    (-4..4).for_each(|section| chunk.fill_section(section, stone));
    (64..=70).for_each(|y| (0..16).for_each(|x| (0..16).for_each(|z| chunk.set(x, y, z, 0, stone))));
    storage.insert(0, chunk);
    (World::with_storage(Arc::new(storage)), stone)
}

fn wait_loaded(world: &mut World, pos: ChunkPos) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !world.is_loaded(pos) {
        assert!(Instant::now() < deadline, "{pos:?} never loaded");
        std::thread::sleep(Duration::from_millis(1));
        world.poll();
    }
}

#[test]
fn platform_spans_four_columns_and_void_stays_air() {
    let (world, stone) = platform();
    let air = world.registry().air_id();
    assert_eq!((world.block(-8, 64, -8), world.block(7, 64, 7)), (stone, stone));
    assert_eq!((world.block(8, 64, 0), world.block(0, 65, 0), world.block(0, 400, 0)), (air, air, air));
    assert_eq!(world.written_columns(), 4);
    // y 64 is the ninth section from the bottom (-64).
    assert_eq!(world.level_chunk(ChunkPos::new(0, 0), false).unwrap().highest_subchunk_count, Some(9));
    let void = world.level_chunk(ChunkPos::new(5, 5), false).unwrap().packet(5, 5, 0);
    assert_eq!((void.x, void.z, void.highest_subchunk_count, void.cache_enabled), (5, 5, Some(0), false));
    assert_eq!((world.surface(0, 0), world.surface(40, 40)), (Some(64), None));
}

#[test]
fn sections_decode_back_to_the_blocks_written() {
    let (world, stone) = platform();
    let entry = world.sub_chunk(ChunkPos::new(-1, 0), 4, [-1, 4, 0], false).entry;
    assert_eq!((entry.dx, entry.dy, entry.dz, entry.result), (-1, 4, 0, SubChunkEntryItemResult::Success));
    assert_eq!(entry.heightmap_type, HeightMapDataType::HasData);
    let registry = world.registry();
    let mut decoded = Chunk::empty(-1, 0, Dimension::overworld(registry.air_id()));
    let unhash = |hash| registry.runtime_id_from_hash(hash).expect("a vanilla state hash");
    decoded.set_sub_chunk_mapped(4, &entry.payload.unwrap(), &unhash).unwrap();
    assert_eq!((decoded.block(-1, 64, 3), decoded.block(-1, 65, 3)), (stone, registry.air_id()));

    let above = world.sub_chunk(ChunkPos::new(-1, 0), 5, [0, 0, 0], false).entry;
    assert_eq!((above.result, above.payload, above.heightmap_type), (SubChunkEntryItemResult::SuccessAllAir, None, HeightMapDataType::TooLow));
    let result = |pos, y| world.sub_chunk(pos, y, [0, 0, 0], false).entry.result;
    assert_eq!(result(ChunkPos::new(9, 9), 4), SubChunkEntryItemResult::SuccessAllAir);
    assert_eq!(result(ChunkPos::new(0, 0), 24), SubChunkEntryItemResult::YIndexOutOfBounds);
    assert_eq!(result(ChunkPos::new(0, 0), -5), SubChunkEntryItemResult::YIndexOutOfBounds);
}

#[test]
fn a_write_drops_only_its_section_from_the_cache() {
    let (mut world, stone) = platform();
    let pos = ChunkPos::new(0, 0);
    let before = world.sub_chunk(pos, 4, [0, 0, 0], false).entry;
    world.sub_chunk(pos, 0, [0, 0, 0], false);
    assert!(world.is_section_cached(pos, 4) && world.is_section_cached(pos, 0));

    world.set_block(3, 90, 3, stone);
    assert!(world.is_section_cached(pos, 4) && world.is_section_cached(pos, 0));
    assert_eq!(world.level_chunk(pos, false).unwrap().highest_subchunk_count, Some(10));
    // The heightmap is not cached with the section: the new block above raises it.
    let after = world.sub_chunk(pos, 4, [0, 0, 0], false).entry;
    assert_eq!(after.payload, before.payload);
    assert_ne!(after.heightmap, before.heightmap);

    world.set_block(3, 64, 3, world.registry().air_id());
    assert!(!world.is_section_cached(pos, 4));
    assert_ne!(world.sub_chunk(pos, 4, [0, 0, 0], false).entry.payload, before.payload);
}

#[test]
fn a_blob_cache_client_gets_hashes_and_the_same_bytes_as_blobs() {
    let (world, _) = platform();
    let pos = ChunkPos::new(0, 0);
    let plain = world.sub_chunk(pos, 4, [0, 0, 0], false);
    let cached = world.sub_chunk(pos, 4, [0, 0, 0], true);
    let blob = cached.blob.expect("a section blob");
    assert_eq!(plain.blob, None);
    assert_eq!((plain.entry.payload.as_ref(), plain.entry.blob_id), (Some(&blob.payload), None));
    assert_eq!((cached.entry.payload.as_deref(), cached.entry.blob_id), (Some(&[][..]), Some(xxh64(&blob.payload))));
    assert_eq!((cached.entry.result, cached.entry.heightmap), (plain.entry.result, plain.entry.heightmap));

    let air = world.sub_chunk(pos, 5, [0, 0, 0], true);
    assert_eq!((air.entry.result, air.entry.blob_id, air.blob), (SubChunkEntryItemResult::SuccessAllAir, None, None));

    let (plain, cached) = (world.level_chunk(pos, false).unwrap(), world.level_chunk(pos, true).unwrap());
    let packet = cached.packet(0, 0, 0);
    assert_eq!((packet.cache_enabled, packet.highest_subchunk_count, &packet.payload[..]), (true, Some(9), &[0][..]));
    assert_eq!(packet.blobs, [xxh64(&cached.blobs[0].payload)]);
    // The plain payload is the biomes followed by the same one-byte tail.
    assert_eq!(plain.payload, [&cached.blobs[0].payload[..], &[0]].concat());
}

#[test]
fn stored_columns_arrive_from_the_io_threads() {
    let (mut world, stone) = stored();
    let (pos, absent) = (ChunkPos::new(2, 3), ChunkPos::new(-7, 0));
    let air = world.registry().air_id();
    assert!(!world.is_loaded(pos) && world.level_chunk(pos, false).is_none());
    assert_eq!(world.sub_chunk(pos, 4, [0, 0, 0], false).entry.result, SubChunkEntryItemResult::ChunkNotFound);
    assert_eq!(world.block(33, 70, 49), air, "not loaded reads as air");
    assert!(!world.set_block(33, 70, 49, stone), "and cannot be written");

    assert!(!world.request(pos) && !world.request(pos) && !world.request(absent));
    assert_eq!(world.loaded_columns(), (0, 2));
    wait_loaded(&mut world, pos);
    wait_loaded(&mut world, absent);
    assert_eq!(world.loaded_columns(), (2, 0));
    assert!(world.request(pos));

    assert_eq!((world.block(33, 70, 49), world.block(33, 71, 49), world.surface(33, 49)), (stone, air, Some(70)));
    assert_eq!(world.level_chunk(pos, false).unwrap().highest_subchunk_count, Some(9));
    assert!(world.is_section_cached(pos, 4), "encoded on the I/O thread");
    // Storage has nothing at `absent`: it is void, not an error.
    assert_eq!(world.level_chunk(absent, false).unwrap().highest_subchunk_count, Some(0));
    assert_eq!(world.sub_chunk(absent, 0, [0, 0, 0], false).entry.result, SubChunkEntryItemResult::SuccessAllAir);
}

#[test]
fn unloading_keeps_what_is_wanted_or_written() {
    let (mut world, stone) = stored();
    let positions = [ChunkPos::new(2, 3), ChunkPos::new(0, 0), ChunkPos::new(1, 1)];
    positions.iter().for_each(|&pos| world.load_now(pos));
    assert!(world.set_block(0, 10, 0, stone));
    world.unload(|pos| pos == ChunkPos::new(1, 1));
    assert_eq!(positions.map(|pos| world.is_loaded(pos)), [false, true, true]);
    // Loading again reads storage again.
    world.load_now(positions[0]);
    assert_eq!(world.block(33, 70, 49), stone);
}

#[test]
fn written_columns_reach_storage_and_only_then_unload() {
    let storage = Arc::new(MemoryStorage::default());
    let mut world = World::with_storage(storage.clone());
    let stone = world.block_id("minecraft:stone").unwrap();
    let (pos, other) = (ChunkPos::new(0, 0), ChunkPos::new(4, 4));
    let stored = |pos: ChunkPos, [x, y, z]: [i32; 3]| storage.load_column(pos, 0).unwrap().map(|column| column.chunk.block(x, y, z));
    world.load_now(pos);
    world.load_now(other);
    assert!(world.set_block(1, 10, 1, stone));
    world.unload(|_| false);
    assert_eq!((world.is_loaded(pos), world.is_loaded(other), stored(pos, [1, 10, 1])), (true, false, None), "unsaved columns stay");

    assert_eq!((world.save_modified(), world.save_modified()), (1, 0));
    // Written again while its save is on its way, or after it: the second save waits for the first.
    assert!(world.set_block(2, 10, 2, stone));
    world.unload(|_| false);
    assert!(world.is_loaded(pos));
    assert_eq!(world.save_and_wait(), 0);
    assert_eq!((stored(pos, [1, 10, 1]), stored(pos, [2, 10, 2]), stored(other, [0, 0, 0])), (Some(stone), Some(stone), None));

    world.unload(|_| false);
    assert!(!world.is_loaded(pos), "a saved column unloads");
    world.load_now(pos);
    assert_eq!((world.block(1, 10, 1), world.block(2, 10, 2)), (stone, stone));
}

#[test]
fn a_refused_save_keeps_the_column_unsaved() {
    /// Refuses saves; panics instead for the column at x 1, and cannot read the one at x 2.
    struct Broken;
    impl WorldStorage for Broken {
        fn load_column(&self, pos: ChunkPos, _: i32) -> Result<Option<StoredColumn>, StorageError> {
            if pos.x == 2 { Err("unreadable".into()) } else { Ok(None) }
        }
        fn store_column(&self, pos: ChunkPos, _: i32, _: &StoredColumn) -> Result<(), StorageError> {
            assert!(pos.x != 1, "storage panics");
            Err("read-only".into())
        }
    }
    let mut world = World::with_storage(Arc::new(Broken));
    let stone = world.block_id("minecraft:stone").unwrap();
    (0..3).for_each(|x| world.load_now(ChunkPos::new(x, 0)));
    assert!(world.set_block(1, 10, 1, stone) && world.set_block(17, 10, 1, stone));
    assert!(!world.set_block(33, 10, 1, stone), "a column storage could not read is not written over");
    assert_eq!(world.save_and_wait(), 2, "the panic costs its save, not the wait");
    world.unload(|_| false);
    assert_eq!((world.block(1, 10, 1), world.block(17, 10, 1)), (stone, stone), "what storage refused is still in memory");
}

#[test]
fn a_block_entity_leaves_with_its_block() {
    let entity = |[x, y, z]: [i32; 3]| {
        let fields = [("id", Value::String("Chest".into())), ("x", Value::Int(x)), ("y", Value::Int(y)), ("z", Value::Int(z))];
        let mut out = BytesMut::new();
        nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: Value::Compound(fields.map(|(key, value)| (key.to_string(), value)).into()) });
        out.to_vec()
    };
    let storage = Arc::new(MemoryStorage::default());
    let registry = acacia_world::BlockRegistry::vanilla();
    let (pos, chunk) = (ChunkPos::new(0, 0), Chunk::empty(0, 0, Dimension::overworld(registry.air_id())));
    let (kept, replaced) = (entity([3, 10, 3]), entity([4, 10, 4]));
    storage.store_column(pos, 0, &StoredColumn { block_entities: [&kept[..], &replaced].concat(), ..StoredColumn::of(chunk) }).unwrap();

    let mut world = World::with_storage(storage.clone());
    world.load_now(pos);
    let (air, stone) = (world.registry().air_id(), world.block_id("minecraft:stone").unwrap());
    assert!(world.set_block(3, 10, 3, air), "the same kind of block: its entity stays");
    assert!(world.set_block(4, 10, 4, stone));
    assert_eq!(world.save_and_wait(), 0);
    assert_eq!(storage.load_column(pos, 0).unwrap().unwrap().block_entities, kept);
}

#[test]
fn actors_and_the_level_go_back_to_storage_with_the_world() {
    #[derive(Default)]
    struct Recording {
        columns: MemoryStorage,
        flushed: Mutex<Option<Level>>,
    }
    impl WorldStorage for Recording {
        fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError> {
            self.columns.load_column(pos, dimension)
        }
        fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), StorageError> {
            self.columns.store_column(pos, dimension, column)
        }
        fn flush(&self, level: &Level) -> Result<(), StorageError> {
            *self.flushed.lock().unwrap() = Some(*level);
            Ok(())
        }
    }
    let storage = Arc::new(Recording::default());
    let (pos, chunk) = (ChunkPos::new(0, 0), Chunk::empty(0, 0, Dimension::overworld(acacia_world::BlockRegistry::vanilla().air_id())));
    storage.store_column(pos, 0, &StoredColumn { actors: b"two cows".to_vec(), ..StoredColumn::of(chunk) }).unwrap();

    let mut world = World::with_storage(storage.clone());
    assert_eq!((*world.level(), world.take_arrived()), (Level::default(), Vec::new()));
    world.load_now(pos);
    world.request(ChunkPos::new(5, 5));
    wait_loaded(&mut world, ChunkPos::new(5, 5));
    assert_eq!((world.take_arrived(), world.take_arrived()), (vec![pos, ChunkPos::new(5, 5)], Vec::new()), "each column arrives once");
    assert_eq!((world.actors(pos), world.actors(ChunkPos::new(5, 5)), world.actors(ChunkPos::new(9, 9))), (&b"two cows"[..], &[][..], &[][..]));

    let stone = world.block_id("minecraft:stone").unwrap();
    assert!(world.set_block(1, 10, 1, stone));
    *world.level_mut() = Level { spawn: Some([4, 70, -2]), time: 18_000, daylight_cycle: false };
    assert_eq!(world.save_and_wait(), 0);
    assert_eq!(storage.load_column(pos, 0).unwrap().unwrap().actors, b"two cows");
    assert_eq!(*storage.flushed.lock().unwrap(), Some(Level { spawn: Some([4, 70, -2]), time: 18_000, daylight_cycle: false }));
}

#[test]
fn spawn_lands_on_the_surface_unless_the_save_names_a_height() {
    let (mut world, _) = stored();
    assert_eq!(world.spawn_at([33, 32767, 49]), [33.5, 71.0, 49.5]);
    assert_eq!(world.spawn_at([33, 90, 49]), [33.5, 90.0, 49.5]);
    assert_eq!(world.spawn_at([-100, 32767, 5]), [-99.5, 64.0, 5.5], "void under the spawn");
}
