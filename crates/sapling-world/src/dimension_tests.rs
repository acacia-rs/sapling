//! Worlds of the Nether beside the overworld on one storage.

use std::sync::{Arc, Mutex};

use acacia_world::{BlockRegistry, Chunk, Dimension};

use crate::{ChunkPos, END, Level, MemoryStorage, NETHER, OVERWORLD, StorageError, StoredColumn, World, WorldStorage};

/// Memory storage that counts what closing a world did to it.
#[derive(Default)]
struct Counted {
    inner: MemoryStorage,
    flushed: Mutex<Vec<Level>>,
    synced: Mutex<usize>,
}

impl WorldStorage for Counted {
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError> {
        self.inner.load_column(pos, dimension)
    }
    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), StorageError> {
        self.inner.store_column(pos, dimension, column)
    }
    fn columns(&self, dimension: i32) -> Result<Vec<ChunkPos>, StorageError> {
        self.inner.columns(dimension)
    }
    fn flush(&self, level: &Level) -> Result<(), StorageError> {
        self.flushed.lock().unwrap().push(level.clone());
        Ok(())
    }
    fn sync(&self) -> Result<(), StorageError> {
        *self.synced.lock().unwrap() += 1;
        Ok(())
    }
}

#[test]
fn a_void_nether_is_128_high_and_announces_its_own_dimension() {
    let mut nether = World::void_in(NETHER);
    let quartz = nether.block_id("minecraft:quartz_ore").unwrap();
    assert!(nether.set_block(3, 127, 3, quartz));
    assert!(nether.set_block(3, 200, 3, quartz), "out of height: ignored");
    assert_eq!((nether.dimension(), nether.heights(), nether.in_height(128), nether.in_height(0)), (NETHER, 0..128, false, true));
    assert_eq!((nether.block(3, 127, 3), nether.block(3, 200, 3)), (quartz, nether.registry().air_id()));
    let column = nether.level_chunk(ChunkPos::new(0, 0), false).unwrap();
    assert_eq!(column.highest_subchunk_count, Some(8));
    assert_eq!(column.packet(0, 0, NETHER).dimension, NETHER);
    assert!(nether.sub_chunk(ChunkPos::new(0, 0), 8, [0, 8, 0], false).entry.payload.is_none(), "no section above y 127");
    assert_eq!(crate::empty_column(NETHER, false).highest_subchunk_count, Some(0));
}

#[test]
fn each_dimension_saves_under_its_own_keys_and_only_the_overworld_writes_the_level() {
    let storage = Arc::new(Counted::default());
    let (mut overworld, mut nether) = (World::with_storage(storage.clone()), World::with_storage_in(storage.clone(), NETHER));
    let pos = ChunkPos::new(1, -1);
    overworld.load_now(pos);
    nether.load_now(pos);
    let (stone, netherrack) = (overworld.block_id("minecraft:stone").unwrap(), nether.block_id("minecraft:netherrack").unwrap());
    assert!(overworld.set_block(16, -60, -16, stone));
    assert!(nether.set_block(16, 100, -16, netherrack));
    nether.level_mut().time = 999;
    assert_eq!(nether.save_and_wait(), 0);
    assert_eq!((storage.flushed.lock().unwrap().len(), *storage.synced.lock().unwrap()), (0, 1), "the Nether leaves the level alone");
    overworld.level_mut().time = 1234;
    assert_eq!(overworld.save_and_wait(), 0);
    assert_eq!(storage.flushed.lock().unwrap().iter().map(|level| level.time).collect::<Vec<_>>(), [1234]);

    assert_eq!((storage.columns(OVERWORLD).unwrap(), storage.columns(NETHER).unwrap()), (vec![pos], vec![pos]));
    let registry = BlockRegistry::vanilla();
    let saved = |dimension| storage.load_column(pos, dimension).unwrap().unwrap().chunk;
    assert_eq!((saved(OVERWORLD).block(16, -60, -16), saved(NETHER).block(16, 100, -16)), (stone, netherrack));
    assert_eq!(saved(NETHER).dimension(), Dimension::nether(registry.air_id()));
    let reopened = {
        let mut world = World::with_storage_in(storage.clone(), NETHER);
        world.load_now(pos);
        world.block(16, 100, -16)
    };
    assert_eq!(reopened, netherrack);
}

#[test]
fn a_nether_column_saved_without_biomes_is_hell() {
    let storage = MemoryStorage::default();
    storage.insert(NETHER, Chunk::empty(0, 0, Dimension::nether(BlockRegistry::vanilla().air_id())));
    let mut nether = World::with_storage_in(Arc::new(storage), NETHER);
    nether.load_now(ChunkPos::new(0, 0));
    let column = nether.column(ChunkPos::new(0, 0)).unwrap();
    assert_eq!(column.stored().chunk.biome(5, 70, 5), Some(8));
}

#[test]
fn the_end_is_256_high_saves_under_dimension_two_and_is_the_end_where_unpainted() {
    let storage = Arc::new(MemoryStorage::default());
    storage.insert(END, Chunk::empty(1, 0, Dimension::end(BlockRegistry::vanilla().air_id())));
    let mut end = World::with_storage_in(storage.clone(), END);
    let pos = ChunkPos::new(0, 0);
    end.load_now(pos);
    let obsidian = end.block_id("minecraft:obsidian").unwrap();
    assert!(end.set_block(3, 255, 3, obsidian) && end.set_block(3, 256, 3, obsidian), "the top cell, and one over it ignored");
    assert_eq!(end.block(3, 256, 3), end.registry().air_id());
    assert_eq!((end.dimension(), end.heights()), (END, 0..256));
    assert_eq!(end.save_and_wait(), 0);
    assert!(storage.columns(END).unwrap().contains(&pos) && storage.columns(NETHER).unwrap().is_empty() && storage.columns(OVERWORLD).unwrap().is_empty());
    assert_eq!(storage.load_column(pos, END).unwrap().unwrap().chunk.block(3, 255, 3), obsidian);
    end.load_now(ChunkPos::new(1, 0));
    assert_eq!(end.column(ChunkPos::new(1, 0)).unwrap().stored().chunk.biome(5, 70, 5), Some(9));
    assert_eq!(crate::empty_column(END, false).highest_subchunk_count, Some(0));
}
