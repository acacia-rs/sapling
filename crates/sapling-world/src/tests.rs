use std::sync::Arc;
use std::time::{Duration, Instant};

use acacia_proto::types::{HeightMapDataType, SubChunkEntryItemResult};
use acacia_proto::xxh64;
use acacia_world::{Chunk, Dimension};

use crate::{ChunkPos, MemoryStorage, World};

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
fn spawn_lands_on_the_surface_unless_the_save_names_a_height() {
    let (mut world, _) = stored();
    assert_eq!(world.spawn_at([33, 32767, 49]), [33.5, 71.0, 49.5]);
    assert_eq!(world.spawn_at([33, 90, 49]), [33.5, 90.0, 49.5]);
    assert_eq!(world.spawn_at([-100, 32767, 5]), [-99.5, 64.0, 5.5], "void under the spawn");
}
