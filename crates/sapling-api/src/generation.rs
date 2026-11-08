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
    /// Whether its jigsaw blocks stay, with their block entities, in place of their final states:
    /// BDS's `keepJigsaws`, false unless given true (bds-k1jig.txt).
    pub keep_jigsaws: bool,
    /// `/place structure`'s `ignoreStartHeight`: a data-driven structure's start piece stands at
    /// the position's height, not its own (bds-n13jig.txt).
    pub ignore_start_height: bool,
    /// `liquidSettings` `ignore_waterlogging`: what it places keeps no water in layer 1, where
    /// `apply_waterlogging` (the default) waterlogs what it places in water (bds-n13liq.txt).
    pub ignore_waterlogging: bool,
}

impl Default for PlaceOptions {
    fn default() -> Self {
        Self { include_entities: true, keep_jigsaws: false, ignore_start_height: false, ignore_waterlogging: false }
    }
}

impl PlaceOptions {
    pub fn with_entities(include_entities: bool) -> Self {
        Self { include_entities, ..Self::default() }
    }

    pub fn keeping_jigsaws(self, keep_jigsaws: bool) -> Self {
        Self { keep_jigsaws, ..self }
    }

    pub fn ignoring_start_height(self, ignore_start_height: bool) -> Self {
        Self { ignore_start_height, ..self }
    }

    pub fn ignoring_waterlogging(self, ignore_waterlogging: bool) -> Self {
        Self { ignore_waterlogging, ..self }
    }
}

impl ShardCtx<'_> {
    /// Places the generator's feature `name` (`minecraft:oak_tree_feature`) at `at`.
    pub fn place_feature(&mut self, name: &str, at: BlockPos) -> Result<(), PlaceError> {
        self.backend.place_feature(name, at)
    }

    /// Runs the generator's feature rule `name` in the chunk of `at`, as worldgen would there
    /// whatever its biomes; Failed where it placed nothing.
    pub fn place_feature_rule(&mut self, name: &str, at: BlockPos) -> Result<(), PlaceError> {
        self.backend.place_feature_rule(name, at)
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
    fn place_feature_rule(&mut self, name: &str, at: BlockPos) -> Result<(), PlaceError>;
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
    fn place_feature_rule(&mut self, _: &str, _: BlockPos) -> Result<(), PlaceError> {
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
