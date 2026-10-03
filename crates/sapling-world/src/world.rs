use std::collections::HashMap;
use std::sync::Arc;

use acacia_proto::types::SubChunkEntryItemResult;
use acacia_world::{BlockRegistry, Dimension, Heightmap, LevelChunkData, SubChunkData};

use crate::ChunkPos;
use crate::column::{Column, SubChunkAnswer};
use crate::loader::{Loader, Source};
use crate::storage::WorldStorage;

/// Loads in flight at once per world; each is a few milliseconds of disk and encoding.
const IO_THREADS: usize = 2;

/// One dimension's blocks.
///
/// Without storage every column exists: those nobody wrote to are void and encode as all air.
/// With storage a column exists once it is loaded: [`World::request`] asks the I/O threads for
/// it and [`World::poll`] takes what they finished, so no call here waits on disk except
/// [`World::load_now`].
pub struct World {
    dimension: i32,
    dim: Dimension,
    registry: Arc<BlockRegistry>,
    columns: HashMap<ChunkPos, Column>,
    /// Stands in for every unwritten column of a world without storage, so they share one encoding.
    void: Column,
    loader: Option<Loader>,
}

impl World {
    /// An empty overworld without storage.
    pub fn void() -> Self {
        let registry = BlockRegistry::vanilla_arc();
        let dim = Dimension::overworld(registry.air_id());
        Self { dimension: 0, dim, registry, columns: HashMap::new(), void: Column::empty(0, 0, dim), loader: None }
    }

    /// The overworld of `storage`. Columns it does not hold are void.
    pub fn with_storage(storage: Arc<dyn WorldStorage>) -> Self {
        let mut world = Self::void();
        let source = Source { storage, registry: world.registry.clone(), dimension: world.dimension, dim: world.dim };
        world.loader = Some(Loader::spawn(source, IO_THREADS));
        world
    }

    /// The protocol dimension id (0 = overworld).
    pub fn dimension(&self) -> i32 {
        self.dimension
    }

    pub fn registry(&self) -> &BlockRegistry {
        &self.registry
    }

    /// The runtime id of a block's default-ordered first state, e.g. `minecraft:stone`.
    pub fn block_id(&self, name: &str) -> Option<u32> {
        self.registry.states_of(name).next().map(|(id, _)| id)
    }

    /// Takes the columns the I/O threads finished; returns how many. Call once a tick.
    pub fn poll(&mut self) -> usize {
        let Some(loader) = &mut self.loader else { return 0 };
        let finished = loader.finished();
        let count = finished.len();
        for (pos, column) in finished {
            self.columns.entry(pos).or_insert(column);
        }
        count
    }

    pub fn is_loaded(&self, pos: ChunkPos) -> bool {
        self.loader.is_none() || self.columns.contains_key(&pos)
    }

    /// Whether the column can be served now; if not, its load is queued (once).
    pub fn request(&mut self, pos: ChunkPos) -> bool {
        let loaded = self.is_loaded(pos);
        if let (false, Some(loader)) = (loaded, &mut self.loader) {
            loader.request(pos);
        }
        loaded
    }

    /// Loads a column on the calling thread. For startup code; a shard uses [`World::request`].
    pub fn load_now(&mut self, pos: ChunkPos) {
        if let (false, Some(loader)) = (self.is_loaded(pos), &self.loader) {
            let column = loader.source().load(pos);
            self.columns.insert(pos, column);
        }
    }

    /// Columns held in memory, and loads on their way.
    pub fn loaded_columns(&self) -> (usize, usize) {
        (self.columns.len(), self.loader.as_ref().map_or(0, Loader::pending))
    }

    /// Drops the loaded columns `keep` rejects. Columns written to stay: nothing saves them yet.
    pub fn unload(&mut self, keep: impl Fn(ChunkPos) -> bool) {
        if self.loader.is_some() {
            self.columns.retain(|pos, column| column.is_modified() || keep(*pos));
        }
    }

    /// The layer-0 block at a position; air where nothing is loaded.
    pub fn block(&self, x: i32, y: i32, z: i32) -> u32 {
        self.column(ChunkPos::of_block(x, z)).map_or(self.dim.air, |column| column.block(x, y, z))
    }

    /// The y of the highest block in block column `(x, z)` that a player can stand on or in
    /// (anything with collision, or a liquid). `None` when there is none or the column is not loaded.
    pub fn surface(&self, x: i32, z: i32) -> Option<i32> {
        let stands = |id| self.registry.get(id).is_some_and(|s| s.is_solid() || s.is_liquid());
        self.column(ChunkPos::of_block(x, z))?.top(x, z, stands)
    }

    /// Sets a layer-0 block by runtime id; false when the column is not loaded. Positions
    /// outside the dimension's height are ignored.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, id: u32) -> bool {
        let pos = ChunkPos::of_block(x, z);
        if !self.is_loaded(pos) {
            return false;
        }
        self.columns.entry(pos).or_insert_with(|| Column::empty(pos.x, pos.z, self.dim)).set_block(x, y, z, id);
        true
    }

    /// Fills the box between two corners, both included.
    pub fn fill(&mut self, from: [i32; 3], to: [i32; 3], id: u32) {
        let range = |axis: usize| from[axis].min(to[axis])..=from[axis].max(to[axis]);
        for x in range(0) {
            for z in range(2) {
                for y in range(1) {
                    self.set_block(x, y, z, id);
                }
            }
        }
    }

    /// What announces a column in sub-chunk request mode: `.packet(x, z, dimension)` is the
    /// `LevelChunk`, and for a blob-cache client (`cached`) `.blobs` holds the biomes it names.
    /// `None` until the column is loaded.
    pub fn level_chunk(&self, pos: ChunkPos, cached: bool) -> Option<&LevelChunkData> {
        Some(self.column(pos)?.level_chunk(cached))
    }

    /// One entry of a `Subchunk` answer: section `section_y` of column `pos`, requested at
    /// `offset` from the request's origin.
    pub fn sub_chunk(&self, pos: ChunkPos, section_y: i32, offset: [i8; 3], cached: bool) -> SubChunkAnswer {
        let failed = |result| {
            let missing = SubChunkData { payload: None, blob: None, heightmap: Heightmap::TooLow };
            SubChunkAnswer { entry: acacia_proto::types::SubChunkEntryItem { result, ..missing.entry(offset[0], offset[1], offset[2]) }, blob: None }
        };
        let Some(column) = self.column(pos) else { return failed(SubChunkEntryItemResult::ChunkNotFound) };
        column.sub_chunk(section_y, &self.registry, offset, cached).unwrap_or_else(|| failed(SubChunkEntryItemResult::YIndexOutOfBounds))
    }

    fn column(&self, pos: ChunkPos) -> Option<&Column> {
        self.columns.get(&pos).or(self.loader.is_none().then_some(&self.void))
    }

    #[cfg(test)]
    pub(crate) fn is_section_cached(&self, pos: ChunkPos, section_y: i32) -> bool {
        self.columns[&pos].is_section_cached(section_y)
    }

    #[cfg(test)]
    pub(crate) fn written_columns(&self) -> usize {
        self.columns.len()
    }
}
