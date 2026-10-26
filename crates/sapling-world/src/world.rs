mod wire;

use std::collections::HashMap;
use std::sync::Arc;

use acacia_world::{BlockRegistry, Dimension};

use crate::ChunkPos;
use crate::column::Column;
use crate::light::Lights;
use crate::loader::{Loader, Source};
use crate::players::Players;
use crate::storage::{Level, WorldStorage};
use crate::template::Template;

pub const OVERWORLD: i32 = 0;
pub const NETHER: i32 = 1;
pub const END: i32 = 2;

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
    pub(crate) players: Players,
    pub(crate) light: Lights,
}

impl World {
    /// An empty overworld without storage.
    pub fn void() -> Self {
        Self::void_in(OVERWORLD)
    }

    /// An empty world of a dimension (0 overworld, 1 nether, 2 end) without storage.
    pub fn void_in(dimension: i32) -> Self {
        let registry = BlockRegistry::vanilla_arc();
        let dim = Dimension::from_id(dimension, registry.air_id());
        let (columns, void, players, light) = (HashMap::new(), Column::empty(0, 0, dim), Players::default(), Lights::new(&registry));
        Self { dimension, dim, registry, columns, void, loader: None, level: Level::default(), arrived: Vec::new(), base: None, base_actors_taken: false, players, light }
    }

    /// The overworld of `storage`, loaded on [`WorldStorage::load_threads`] threads. Columns it
    /// does not hold are void.
    pub fn with_storage(storage: Arc<dyn WorldStorage>) -> Self {
        Self::with_storage_in(storage, OVERWORLD)
    }

    /// The dimension `dimension` of `storage`, which keeps each dimension's columns under its own
    /// keys. A world of another dimension than the overworld leaves the level (spawn, time,
    /// rules) to the overworld's world: closing it flushes storage without writing its [`Level`].
    pub fn with_storage_in(storage: Arc<dyn WorldStorage>, dimension: i32) -> Self {
        let mut world = Self::void_in(dimension);
        let threads = storage.load_threads();
        let source = Source { storage, registry: world.registry.clone(), dimension: world.dimension, dim: world.dim, light: world.light.props.clone() };
        world.loader = Some(Loader::spawn(source, threads));
        world
    }

    /// The protocol dimension id (0 = overworld).
    pub fn dimension(&self) -> i32 {
        self.dimension
    }

    /// The dimension's height range, `min_y..min_y + height`.
    pub fn heights(&self) -> std::ops::Range<i32> {
        self.dim.min_y..self.dim.min_y + self.dim.height as i32
    }

    pub fn registry(&self) -> &BlockRegistry {
        &self.registry
    }

    pub fn level(&self) -> &Level {
        &self.level
    }

    /// The block an eye of ender thrown from (x, z) in this world flies to
    /// ([`WorldStorage::locate_stronghold`]); `None` without storage that makes strongholds.
    pub fn locate_stronghold(&self, x: i32, z: i32) -> Option<[i32; 3]> {
        self.loader.as_ref()?.storage().locate_stronghold(x, z)
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

    /// Replaces a loaded column's actors, to be saved with it; false where none is loaded.
    pub fn set_actors(&mut self, pos: ChunkPos, actors: Vec<u8>) -> bool {
        self.column_mut(pos).map(|column| column.set_actors(actors)).is_some()
    }

    /// The loaded columns that hold actors.
    pub fn columns_with_actors(&self) -> Vec<ChunkPos> {
        self.columns.iter().filter(|(_, column)| !column.actors().is_empty()).map(|(pos, _)| *pos).collect()
    }

    /// The runtime id of a block's default-ordered first state, e.g. `minecraft:stone`.
    pub fn block_id(&self, name: &str) -> Option<u32> {
        self.registry.states_of(name).next().map(|(id, _)| id)
    }

    pub fn is_loaded(&self, pos: ChunkPos) -> bool {
        self.loader.is_none() || self.columns.contains_key(&pos)
    }

    /// The layer-0 block at a position; air where nothing is loaded.
    pub fn block(&self, x: i32, y: i32, z: i32) -> u32 {
        self.column(ChunkPos::of_block(x, z)).map_or(self.dim.air, |column| column.block(x, y, z))
    }

    /// Where the loaded column at `pos` holds a layer-0 block `wanted` accepts.
    pub fn find_blocks(&self, pos: ChunkPos, wanted: impl Fn(u32) -> bool) -> Vec<[i32; 3]> {
        let found = self.column(pos).map(|column| column.find(wanted)).unwrap_or_default();
        found.into_iter().map(|[x, y, z]| [pos.x * 16 + x, y, pos.z * 16 + z]).collect()
    }

    /// The y of the highest block in block column `(x, z)` that a player can stand on or in
    /// (anything with collision, or a liquid). `None` when there is none or the column is not loaded.
    pub fn surface(&self, x: i32, z: i32) -> Option<i32> {
        let stands = |id| self.registry.get(id).is_some_and(|s| s.is_solid() || s.is_liquid());
        self.column(ChunkPos::of_block(x, z))?.top(x, z, stands)
    }

    /// The y of the highest block in block column `(x, z)` that has collision; `None` where there
    /// is none or the column is not loaded.
    pub fn top_solid(&self, x: i32, z: i32) -> Option<i32> {
        self.column(ChunkPos::of_block(x, z))?.top(x, z, |id| self.registry.get(id).is_some_and(|s| s.is_solid()))
    }

    /// Sets a layer-0 block by runtime id; false when the column is not loaded or storage
    /// could not read it. Positions outside the dimension's height are ignored.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, id: u32) -> bool {
        let (registry, before) = (self.registry.clone(), self.light_props(x, y, z));
        let set = self.column_mut(ChunkPos::of_block(x, z)).map(|column| column.set_block(x, y, z, id, &registry)).is_some();
        if set {
            self.light_written([x, y, z], before);
        }
        set
    }

    /// The column to write to, if it is loaded and storage could read it. An arena's first
    /// write to a column makes it the arena's own (`Column::fork`).
    pub(crate) fn column_mut(&mut self, pos: ChunkPos) -> Option<&mut Column> {
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
        let before = self.light_props(x, y, z);
        let set = written && self.column_mut(pos).map(|column| column.set_liquid(x, y, z, id)).is_some();
        if set {
            self.light_written([x, y, z], before);
        }
        set
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

    pub(crate) fn column(&self, pos: ChunkPos) -> Option<&Column> {
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
