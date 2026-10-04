use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use acacia_proto::types::SubChunkEntryItemResult;
use acacia_world::{BlockRegistry, Dimension, Heightmap, LevelChunkData, SubChunkData};

use crate::ChunkPos;
use crate::column::{Column, SubChunkAnswer};
use crate::loader::{Finished, Loader, Source};
use crate::storage::{Level, WorldStorage};
use crate::template::Template;

/// Loads in flight at once per world; each is a few milliseconds of disk and encoding.
const IO_THREADS: usize = 2;

/// One dimension's blocks.
///
/// Without storage every column exists: those nobody wrote to are void and encode as all air.
/// With storage a column exists once it is loaded: [`World::request`] asks the I/O threads for
/// it, [`World::save_modified`] hands them what was written and [`World::poll`] takes what they
/// finished. Only opening a folder (folder.rs) and [`World::save_and_wait`] wait on disk.
pub struct World {
    pub(crate) dimension: i32,
    pub(crate) dim: Dimension,
    pub(crate) registry: Arc<BlockRegistry>,
    /// Of an arena: only the columns that were written to.
    pub(crate) columns: HashMap<ChunkPos, Column>,
    /// Stands in for every unwritten column of a world without storage, so they share one encoding.
    pub(crate) void: Column,
    pub(crate) loader: Option<Loader>,
    pub(crate) level: Level,
    /// Columns that came from storage and [`World::take_arrived`] has not handed out.
    pub(crate) arrived: Vec<ChunkPos>,
    /// Of an arena: where every column it has not written to is read from (template.rs).
    pub(crate) base: Option<Arc<Template>>,
    pub(crate) base_actors_taken: bool,
}

impl World {
    /// An empty overworld without storage.
    pub fn void() -> Self {
        let registry = BlockRegistry::vanilla_arc();
        let dim = Dimension::overworld(registry.air_id());
        let (columns, void) = (HashMap::new(), Column::empty(0, 0, dim));
        Self { dimension: 0, dim, registry, columns, void, loader: None, level: Level::default(), arrived: Vec::new(), base: None, base_actors_taken: false }
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

    pub fn level(&self) -> &Level {
        &self.level
    }

    /// The spawn, the time and what else storage saves when the world closes.
    pub fn level_mut(&mut self) -> &mut Level {
        &mut self.level
    }

    /// The columns that came from storage since the last call, and once an arena's template
    /// columns with actors: their actors are new to the caller.
    pub fn take_arrived(&mut self) -> Vec<ChunkPos> {
        let mut arrived = self.take_base_actors();
        arrived.append(&mut self.arrived);
        arrived
    }

    /// The stored actors of a loaded column (`StoredColumn::actors`); empty when it has none.
    pub fn actors(&self, pos: ChunkPos) -> &[u8] {
        self.column(pos).map_or(&[], Column::actors)
    }

    /// The runtime id of a block's default-ordered first state, e.g. `minecraft:stone`.
    pub fn block_id(&self, name: &str) -> Option<u32> {
        self.registry.states_of(name).next().map(|(id, _)| id)
    }

    /// Takes what the I/O threads finished; returns how many columns they loaded. Call once a tick.
    pub fn poll(&mut self) -> usize {
        let Some(loader) = &mut self.loader else { return 0 };
        let finished = loader.finished();
        finished.into_iter().map(|done| self.accept(done)).filter(|&loaded| loaded).count()
    }

    /// Applies one finished job; true for a loaded column. A failed save leaves its column unsaved.
    fn accept(&mut self, done: Finished) -> bool {
        match done {
            Finished::Loaded(pos, column) => {
                self.insert_loaded(pos, column);
                true
            }
            Finished::Saved { pos, ok } => {
                if let (false, Some(column)) = (ok, self.columns.get_mut(&pos)) {
                    column.mark_modified();
                }
                false
            }
        }
    }

    /// Hands every column written since its last save to the I/O threads; returns how many.
    /// A column whose previous save is still on its way waits for the next call.
    pub fn save_modified(&mut self) -> usize {
        self.queue_saves(&HashSet::new())
    }

    fn insert_loaded(&mut self, pos: ChunkPos, column: Column) {
        if let Entry::Vacant(slot) = self.columns.entry(pos) {
            slot.insert(column);
            self.arrived.push(pos);
        }
    }

    /// Saves every written column, waits for the I/O threads and flushes storage with the
    /// world's [`Level`]; returns how many columns could not be saved. Blocks on disk: for a
    /// world that is being closed, never for a tick.
    pub fn save_and_wait(&mut self) -> usize {
        let mut failed = HashSet::new();
        loop {
            self.queue_saves(&failed);
            let Some(done) = self.loader.as_mut().and_then(Loader::wait) else { break };
            if let Finished::Saved { pos, ok: false } = &done {
                failed.insert(*pos);
            }
            self.accept(done);
        }
        if let Some(Err(error)) = self.loader.as_ref().map(|loader| loader.source().storage.flush(&self.level)) {
            tracing::error!(%error, "flushing storage failed");
        }
        failed.len()
    }

    fn queue_saves(&mut self, skip: &HashSet<ChunkPos>) -> usize {
        let Some(loader) = &mut self.loader else { return 0 };
        let mut queued = 0;
        for (pos, column) in &mut self.columns {
            if column.is_modified() && !loader.is_saving(*pos) && !skip.contains(pos) {
                loader.save(*pos, column.stored());
                column.mark_saved();
                queued += 1;
            }
        }
        queued
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

    /// Loads a column on the calling thread: for opening a world (folder.rs), not for a shard.
    pub(crate) fn load_now(&mut self, pos: ChunkPos) {
        if let (false, Some(loader)) = (self.is_loaded(pos), &self.loader) {
            let column = loader.source().load(pos);
            self.insert_loaded(pos, column);
        }
    }

    /// Columns held in memory, and loads on their way.
    pub fn loaded_columns(&self) -> (usize, usize) {
        (self.columns.len(), self.loader.as_ref().map_or(0, Loader::loading))
    }

    /// Drops the loaded columns `keep` rejects, except those storage does not hold yet: unsaved
    /// ones, and ones whose save is on its way (a reload could read the column before the save).
    pub fn unload(&mut self, keep: impl Fn(ChunkPos) -> bool) {
        if let Some(loader) = &self.loader {
            self.columns.retain(|pos, column| column.is_modified() || loader.is_saving(*pos) || keep(*pos));
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

    /// Sets a layer-0 block by runtime id; false when the column is not loaded or storage
    /// could not read it. Positions outside the dimension's height are ignored.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, id: u32) -> bool {
        let registry = self.registry.clone();
        self.column_mut(ChunkPos::of_block(x, z)).map(|column| column.set_block(x, y, z, id, &registry)).is_some()
    }

    /// The column to write to, if it is loaded and storage could read it. An arena's first
    /// write to a column makes it the arena's own (`Column::fork`).
    fn column_mut(&mut self, pos: ChunkPos) -> Option<&mut Column> {
        if !self.is_loaded(pos) {
            return None;
        }
        let (base, dim) = (&self.base, self.dim);
        let fresh = || base.as_ref().and_then(|template| template.column(pos)).map_or_else(|| Column::empty(pos.x, pos.z, dim), Column::fork);
        Some(self.columns.entry(pos).or_insert_with(fresh)).filter(|column| !column.is_damaged())
    }

    /// The layer-1 block at a position: water around a waterlogged block, air otherwise.
    pub fn liquid(&self, x: i32, y: i32, z: i32) -> u32 {
        self.column(ChunkPos::of_block(x, z)).map_or(self.dim.air, |column| column.liquid(x, y, z))
    }

    /// Sets a layer-1 block; false where [`World::set_block`] is.
    pub fn set_liquid(&mut self, x: i32, y: i32, z: i32, id: u32) -> bool {
        let pos = ChunkPos::of_block(x, z);
        // A column nobody wrote to has no block for a liquid to stand in.
        let written = self.columns.contains_key(&pos) || self.base.as_ref().is_some_and(|template| template.column(pos).is_some());
        written && self.column_mut(pos).map(|column| column.set_liquid(x, y, z, id)).is_some()
    }

    /// Whether blocks exist at this height.
    pub fn in_height(&self, y: i32) -> bool {
        (self.dim.min_y..self.dim.min_y + self.dim.height as i32).contains(&y)
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
        let own = self.columns.get(&pos).or_else(|| self.base.as_ref().and_then(|template| template.column(pos)));
        own.or(self.loader.is_none().then_some(&self.void))
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
