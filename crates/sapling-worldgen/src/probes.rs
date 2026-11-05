//! `OverworldGenerator`'s hooks for the oracle, its replays and the examples: decoration order and traces,
//! thread-local reach, cache sizes. Nothing a world needs to generate.

use crate::feature;
use crate::{Block, OverworldGenerator, ReachCells, ReachTries, structure};

impl OverworldGenerator {
    /// One source chunk's feature writes, decorated alone: world (x, y, z), block placed, block replaced.
    #[doc(hidden)]
    pub fn source_feature_writes(&self, chunk_x: i32, chunk_z: i32) -> Vec<([i32; 3], Block, Block)> {
        let writes = self.with_inputs(|inputs| self.features.source_writes((chunk_x, chunk_z), inputs));
        writes.world_writes((chunk_x, chunk_z))
    }

    /// From now on sources follow what BDS's thread-local generator could reach (feature/reach.rs).
    #[doc(hidden)]
    pub fn track_reach(&mut self) {
        self.features.track_reach = true;
    }

    /// `Features::canonical_shuffles`, for the oracle test's `SAPLING_ORACLE_CANONICAL_SHUFFLES`.
    #[doc(hidden)]
    pub fn shuffle_canonically(&mut self) {
        self.features.canonical_shuffles = true;
    }

    /// Decorates in the order one BDS run did (chunks as it started them) instead of by phase; chunks
    /// the order lacks decorate nothing. For checking a live run (scripts/dbg_order.py).
    #[doc(hidden)]
    /// `thread`: the run had one worker whose generator was seeded so (scripts/bds_seed0.py) and had
    /// drawn that often at each chunk's first `MultifaceFeature` draw (scripts/dbg_loads.py).
    pub fn set_decoration_order(&mut self, order: &[(i32, i32)], thread: Option<(u32, &[Option<u32>])>) {
        self.features.set_order(order, thread);
    }

    /// The chunk's cells a thread-local draw of BDS could have left different from ours, and the
    /// tries the chunk followed as a source; needs `track_reach` before anything is generated.
    #[doc(hidden)]
    pub fn thread_local_reach(&self, chunk_x: i32, chunk_z: i32) -> (Option<ReachCells>, ReachTries) {
        self.with_inputs(|inputs| self.features.reach((chunk_x, chunk_z), inputs))
    }

    /// Process-wide: the dead coral (a `CoralColor`, 0..5) the coral crust templates place; see feature/coral/crust.rs.
    #[doc(hidden)]
    pub fn set_dead_coral(color: usize) {
        feature::set_dead_coral(color);
    }

    /// Molang's `query.noise(x, z)` (examples/query_noise.rs).
    #[doc(hidden)]
    pub fn query_noise(x: f32, z: f32) -> f32 {
        feature::query_noise(x, z)
    }

    /// `source_feature_writes` with the decoration trace (feature/trace.rs) of every source it ran.
    #[doc(hidden)]
    pub fn source_feature_trace(&self, chunk_x: i32, chunk_z: i32) -> Vec<String> {
        feature::trace::capture(|| self.source_feature_writes(chunk_x, chunk_z)).1
    }

    /// Cave segments near a column, for probes (`Carvers::segments_near`).
    #[doc(hidden)]
    pub fn cave_segments_near(&self, x: i32, z: i32, reach: f32, skip_gate: bool) -> Vec<(usize, [f32; 3], f32, f32)> {
        self.carvers.segments_near(x, z, reach, skip_gate)
    }

    /// Each structure start map's entries kept and made.
    #[doc(hidden)]
    pub fn structure_cache_counts(&self) -> Vec<(&'static str, usize, usize)> {
        self.structures.cache_counts()
    }

    /// Keeps only `starts` structure starts of each kind, so that they are dropped and built again
    /// (examples/walk.rs).
    #[doc(hidden)]
    pub fn with_structure_start_limit(mut self, areas: usize, starts: usize) -> Self {
        self.structures = structure::Structures::with_start_limit(self.seed, areas.max(1), starts);
        self
    }
}
