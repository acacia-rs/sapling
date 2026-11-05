//! The world generator on demand: placing one of its features, structures or jigsaw pools at a
//! spot, and finding the nearest start of a structure or the nearest place of a biome, as
//! `/place` and `/locate` do. Worlds without a generator (folders, arenas) find and place nothing.

use crate::{BlockPos, ShardCtx};

/// Why a feature, structure or jigsaw was not placed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaceError {
    /// The generator knows nothing of that name.
    Unknown,
    /// The world in scope has no generator.
    NoGenerator,
    /// Part of what it would cover is not loaded.
    Unloaded,
    /// It could not be placed there (no room, nothing to stand on).
    Failed,
}

/// How a structure or jigsaw is placed, as `/place`'s flags say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct PlaceOptions {
    /// Whether the mobs its templates name (a village's iron golem) are made: BDS's
    /// `includeEntities`, true unless given false (research/captures/bds-k1jig.txt).
    pub include_entities: bool,
}

impl Default for PlaceOptions {
    fn default() -> Self {
        Self { include_entities: true }
    }
}

impl PlaceOptions {
    pub fn with_entities(include_entities: bool) -> Self {
        Self { include_entities }
    }
}

impl ShardCtx<'_> {
    /// Places the generator's feature `name` (`minecraft:oak_tree_feature`) at `at`.
    pub fn place_feature(&mut self, name: &str, at: BlockPos) -> Result<(), PlaceError> {
        self.backend.place_feature(name, at)
    }

    /// Places the generator's structure `name` (as `/locate` names them) as though it started in
    /// the chunk of `at`.
    pub fn place_structure(&mut self, name: &str, at: BlockPos, options: PlaceOptions) -> Result<(), PlaceError> {
        self.backend.place_structure(name, at, options)
    }

    /// Assembles the jigsaw pool `pool` from `at`, starting with a piece whose jigsaw is `target`,
    /// `max_depth` pieces deep.
    pub fn place_jigsaw(&mut self, pool: &str, target: &str, max_depth: u32, at: BlockPos, options: PlaceOptions) -> Result<(), PlaceError> {
        self.backend.place_jigsaw(pool, target, max_depth, at, options)
    }

    /// The block column (x, z) of the nearest start of the structure `name` to `from`, within the
    /// generator's search; `None` for an unknown name or none found.
    pub fn locate_structure(&mut self, name: &str, from: BlockPos) -> Option<[i32; 2]> {
        self.backend.locate_structure(name, from)
    }

    /// The nearest place of the biome `name` (`minecraft:desert`) to `from`, at the height of
    /// `from`.
    pub fn locate_biome(&mut self, name: &str, from: BlockPos) -> Option<BlockPos> {
        self.backend.locate_biome(name, from)
    }
}

/// The generation side of [`crate::Backend`].
pub trait GenerationBackend {
    fn place_feature(&mut self, name: &str, at: BlockPos) -> Result<(), PlaceError>;
    fn place_structure(&mut self, name: &str, at: BlockPos, options: PlaceOptions) -> Result<(), PlaceError>;
    fn place_jigsaw(&mut self, pool: &str, target: &str, max_depth: u32, at: BlockPos, options: PlaceOptions) -> Result<(), PlaceError>;
    fn locate_structure(&mut self, name: &str, from: BlockPos) -> Option<[i32; 2]>;
    fn locate_biome(&mut self, name: &str, from: BlockPos) -> Option<BlockPos>;
}

#[cfg(test)]
impl GenerationBackend for crate::testing::Fake {
    fn place_feature(&mut self, _: &str, _: BlockPos) -> Result<(), PlaceError> {
        Err(PlaceError::NoGenerator)
    }
    fn place_structure(&mut self, _: &str, _: BlockPos, _: PlaceOptions) -> Result<(), PlaceError> {
        Err(PlaceError::NoGenerator)
    }
    fn place_jigsaw(&mut self, _: &str, _: &str, _: u32, _: BlockPos, _: PlaceOptions) -> Result<(), PlaceError> {
        Err(PlaceError::NoGenerator)
    }
    fn locate_structure(&mut self, _: &str, _: BlockPos) -> Option<[i32; 2]> {
        None
    }
    fn locate_biome(&mut self, _: &str, _: BlockPos) -> Option<BlockPos> {
        None
    }
}
