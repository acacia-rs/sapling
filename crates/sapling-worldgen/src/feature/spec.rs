//! The shapes of transcribed feature rules and features (data.rs, scripts/features_to_rust.py).

pub use super::cave_spec::{GrowingPlant, SnapToSurface, VegetationPatch};
pub use super::lush::Lush;
use crate::block::Block;

/// The decoration passes in BDS's registry order; `pregeneration_pass` (carvers) is not one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pass {
    First,
    BeforeUnderground,
    Underground,
    AfterUnderground,
    BeforeSurface,
    Surface,
    AfterSurface,
    BeforeSky,
    Sky,
    AfterSky,
    Final,
}

impl Pass {
    pub const ALL: [Pass; 11] = [
        Pass::First,
        Pass::BeforeUnderground,
        Pass::Underground,
        Pass::AfterUnderground,
        Pass::BeforeSurface,
        Pass::Surface,
        Pass::AfterSurface,
        Pass::BeforeSky,
        Pass::Sky,
        Pass::AfterSky,
        Pass::Final,
    ];
}

/// The biome ids a rule's `minecraft:biome_filter` accepts, evaluated at transcription.
#[derive(Debug, Clone, Copy)]
pub struct BiomeSet(pub [u64; 4]);

impl BiomeSet {
    pub fn contains(&self, biome: u32) -> bool {
        biome < 256 && self.0[(biome >> 6) as usize] & 1 << (biome & 63) != 0
    }
}

/// A Molang-typed field: a constant, or an expression evaluated per use.
#[derive(Debug, Clone, Copy)]
pub enum Expr {
    Const(f32),
    Molang(&'static str),
    /// An expression of `query.has_biome_tag` only, evaluated per biome at transcription: the
    /// value at the scope's biome, 0 for biomes not listed.
    ByBiome(&'static [(u32, f32)]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// One axis of a distribution: a plain value or `{distribution, extent}`.
#[derive(Debug, Clone, Copy)]
pub enum Coordinate {
    Fixed(Expr),
    Uniform([Expr; 2]),
    Gaussian([Expr; 2]),
    InverseGaussian([Expr; 2]),
    Triangle([Expr; 2]),
    FixedGrid { extent: [Expr; 2], step: i32, offset: i32 },
    JitteredGrid { extent: [Expr; 2], step: i32, offset: i32 },
}

#[derive(Debug, Clone, Copy)]
pub enum Chance {
    Percent(Expr),
    Fraction(i32, i32),
}

#[derive(Debug, Clone, Copy)]
pub struct Distribution {
    pub iterations: Expr,
    pub chance: Chance,
    pub order: [Axis; 3],
    pub x: Coordinate,
    pub y: Coordinate,
    pub z: Coordinate,
}

#[derive(Debug)]
pub struct Rule {
    pub name: &'static str,
    pub pass: Pass,
    pub biomes: BiomeSet,
    /// Index into `FEATURES`.
    pub feature: usize,
    pub distribution: Distribution,
}

/// Feature references are indices into `FEATURES`.
#[derive(Debug)]
pub enum Feature {
    Ore(Ore),
    Disk(Disk),
    Scatter(Scatter),
    /// `aggregate_feature`: every feature at the same position.
    Aggregate(&'static [usize], EarlyOut),
    /// `sequence_feature`: each feature at the position the previous one placed at.
    #[allow(dead_code, reason = "no ported vanilla rule reaches one yet")]
    Sequence(&'static [usize], EarlyOut),
    /// `weighted_random_feature`: (feature, weight).
    WeightedRandom(&'static [(usize, f32)]),
    SingleBlock(SingleBlock),
    /// `KelpFeature`: `count` tries at one column each, ±7 around the position.
    Kelp { count: i32 },
    /// `SeagrassFeature`: `count` tries, double with chance `tall`.
    Seagrass { count: i32, tall: f32 },
    /// The legacy land plants BDS builds in code (land.rs, flowers.rs).
    Legacy(Legacy),
    Geode(Geode),
    /// `minecraft:tree_feature`.
    Tree(super::tree::spec::Tree),
    /// `minecraft:search_feature`: the feature at each cell of a box until enough succeed.
    Search(Search),
    /// The legacy tree, foliage and rock features BDS builds in code (legacy_tree.rs).
    LegacyTree(LegacyTree),
    SnapToSurface(SnapToSurface),
    VegetationPatch(VegetationPatch),
    GrowingPlant(GrowingPlant),
    /// The lush caves features BDS builds in code (lush.rs).
    Lush(Lush),
}

/// `minecraft:search_feature` with its axis loops in order (outer, middle, inner), each an axis,
/// a direction and the box's extent on it.
#[derive(Debug)]
pub struct Search {
    pub feature: usize,
    pub loops: [(Axis, i32, [i32; 2]); 3],
    pub required: i32,
}

#[derive(Debug)]
pub enum LegacyTree {
    /// `LegacyTreeFeature`: a count drawn from `count`, then per tree a feature `choice` picks
    /// from `features`, at a random column of the chunk's heightmap.
    Trees { count: f32, choice: TreeChoice, features: &'static [usize], grass: usize },
    /// `LegacyForestFoliageFeature`: double plants (lilac, rose bush, peony) at random heights.
    ForestFoliage { flower_forest: bool },
    /// `LegacySwampFoliageFeature`: seagrass tries, then swamp oaks or a huge mushroom.
    SwampFoliage { seagrass: usize, tree: usize, huge_mushroom: usize, grass: usize },
    /// `LegacyForestRockFeature`: up to two mossy cobblestone blobs.
    ForestRock,
    /// `LeafLitterFeature` (also pink petals): one flower bed of a random size and facing.
    FlowerBed { petals: bool },
    /// `HugeMushroomFeature` of a random type (huge_mushroom.rs).
    HugeMushroom,
}

/// The per-type tree pickers of `LegacyTreeFeature` (its constructor's lambdas), each drawing
/// its own sequence; the doc names the index each outcome picks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeChoice {
    /// Always 0.
    First,
    /// `nextInt(3) == 0` → 0, else 1.
    OneInThree,
    /// `nextInt(5) > 0` → 0, else 1.
    FourInFive,
    /// `nextInt(5) == 0` → 0, else `nextBoolean` → 1 or 0.
    BirchMutated,
    /// `nextInt(3) > 0` → 0, else `nextInt(10) == 0` → 1, else 2.
    ExtremeHills,
    /// `nextInt(5) == 0` → 0, else `nextInt(10) == 0` → 1, else 2.
    Forest,
    /// `nextBoolean` → 0, else 1.
    Meadow,
    /// `nextInt(10) == 0` → 0, else `nextInt(2) == 0` → 1, else `nextInt(3) == 0` → 2, else 3.
    Jungle,
    /// `nextInt(10) == 0` → 0, else `nextInt(2) == 0` → 1, else 2.
    JungleEdge,
    /// `nextInt(20) == 0` → 0, else `nextInt(20) < 3` → 1, else `nextInt(10) < 7` → 2, else 3.
    BambooJungle,
    /// `nextInt(3) == 0` → (`nextInt(13) == 0` → 0, else 1), else `nextInt(3) == 0` → 2, else 3.
    MegaTaiga,
    /// `nextInt(3) == 0` → 0, else `nextInt(3) == 0` → 1, else 2.
    RedwoodMutated,
}

/// `minecraft:geode_feature` (geode.rs); ranges are `[min, max]` as the JSON gives them.
#[derive(Debug)]
pub struct Geode {
    pub filler: Block,
    pub inner: Block,
    pub alternate_inner: Block,
    pub middle: Block,
    pub outer: Block,
    /// Down-facing buds (`Block::faced`).
    pub placements: &'static [Block],
    pub wall: [i32; 2],
    pub points: [i32; 2],
    pub point_offset: [i32; 2],
    pub max_radius: i32,
    pub crack_point_offset: f32,
    pub noise_multiplier: f32,
    pub potential_placements_chance: f32,
    pub alternate_chance: f32,
    pub placements_require_alternate: bool,
    pub invalid_blocks_threshold: i32,
}

#[derive(Debug, Clone, Copy)]
pub enum Legacy {
    DeadBush,
    Reeds,
    Cactus,
    Melon,
    Waterlily,
    /// `BlockPileFeature` with a pumpkin, `count` tries.
    Pumpkin { count: i32 },
    /// `BlockPileFeature` with a sweet berry bush, `count` tries.
    SweetBerries { count: i32 },
    Mushroom { block: Block, count: i32 },
    Flower { kind: FlowerKind, count: i32 },
    /// `legacy:ice_feature`: an iceberg in 1 of 8 chunks (iceberg.rs).
    Iceberg,
}

/// The flower table of a `LegacyFlowerFeature` (1438ef3b0's switch; the forest and flower forest
/// tables are not ported).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowerKind {
    Overworld,
    Plains,
    Swamp,
}

/// `minecraft:scatter_feature`: the distribution drawn from the placing generator.
#[derive(Debug)]
pub struct Scatter {
    pub distribution: Distribution,
    pub project_to_floor: bool,
    pub feature: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarlyOut {
    #[allow(dead_code, reason = "every ported aggregate has an early out")]
    None,
    FirstFailure,
    FirstSuccess,
}

/// `minecraft:single_block_feature` (`SingleBlockFeature::place` 143badfc0).
#[derive(Debug)]
pub struct SingleBlock {
    /// (block, weight); one is drawn even when there is a single entry.
    pub blocks: &'static [(Block, i32)],
    pub may_replace: &'static [Block],
    pub placement: bool,
    pub survival: bool,
    /// `may_not_attach_to.bottom`.
    pub not_on: &'static [Block],
    pub attach: Option<&'static Attach>,
}

/// `may_attach_to`: top, bottom and diagonals must match their lists; of the four sides
/// (north, east, south, west) `min_sides` must match theirs (`sides`, `all` count for each).
/// An empty list matches anything.
#[derive(Debug)]
pub struct Attach {
    pub top: &'static [Block],
    pub bottom: &'static [Block],
    pub diagonal: &'static [Block],
    /// north, east, south, west.
    pub side: [&'static [Block]; 4],
    pub sides: &'static [Block],
    pub all: &'static [Block],
    pub min_sides: i32,
    /// Turn the block toward the last matching side.
    pub auto_rotate: bool,
}

/// A hardcoded disk: placed only where the block at its position is one of `needs`.
#[derive(Debug)]
pub struct Disk {
    pub block: Block,
    pub radius: i32,
    pub half_height: i32,
    pub needs: &'static [Block],
    pub replaces: &'static [Block],
}

#[derive(Debug)]
pub struct Ore {
    pub count: i32,
    pub discard_on_air: f32,
    pub rules: &'static [ReplaceRule],
}

/// `may_replace` matches block types; states are ignored.
#[derive(Debug)]
pub struct ReplaceRule {
    pub places: Block,
    pub may_replace: &'static [Block],
}
