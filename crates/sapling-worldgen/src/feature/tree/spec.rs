//! The shapes of a transcribed `minecraft:tree_feature` (scripts/trees_data.py). Block lists are
//! the JSON descriptors and tags resolved to the blocks sapling places; an empty list passes all.

use crate::block::Block;

/// An int range: `int_range` draws `min + nextInt(max - min)` (none when `min >= max - 1`),
/// `int_range_inclusive` `min + nextInt(max - min + 1)` (none when `min >= max`).
#[derive(Debug, Clone, Copy)]
pub struct Range(pub i32, pub i32);

/// `ChanceInformation`: a float percent (one `nextFloat` unless <= 0 or >= 100) or a fraction
/// (one raw draw modulo the denominator unless it is 0 or equals the numerator).
#[derive(Debug, Clone, Copy)]
pub enum Odds {
    Percent(f32),
    Fraction(u32, u32),
}

#[derive(Debug)]
pub struct Tree {
    pub trunk: Trunk,
    pub canopy: Option<&'static Canopy>,
    pub may_grow_on: &'static [Block],
    pub may_replace: &'static [Block],
    pub may_grow_through: &'static [Block],
    /// The first entry is what the trunk writes under itself when the ground is none of these.
    pub base_block: &'static [Block],
    pub roots: Option<&'static MangroveRoots>,
    pub base_cluster: Option<BaseCluster>,
}

#[derive(Debug)]
pub enum Trunk {
    Simple(SimpleTrunk),
    Acacia(AcaciaTrunk),
    Fancy(FancyTrunk),
    Mega(MegaTrunk),
    Mangrove(MangroveTrunk),
    Cherry(CherryTrunk),
    Fallen(FallenTrunk),
}

/// The bare `trunk` key.
#[derive(Debug)]
pub struct SimpleTrunk {
    pub block: Block,
    pub height: Range,
    pub height_modifier: Range,
    /// `can_be_submerged`: how far the base may sink through `may_grow_through` (0: not at all).
    pub max_depth: i32,
    pub decoration: Option<Decoration>,
}

/// `acacia_trunk` (acacias, dark and pale oaks).
#[derive(Debug)]
pub struct AcaciaTrunk {
    pub block: Block,
    pub width: i32,
    pub height_base: i32,
    pub height_intervals: &'static [i32],
    pub min_height_for_canopy: i32,
    pub lean_height: Range,
    pub lean_steps: Range,
    pub lean_length: Range,
    pub allow_diagonal: bool,
    pub decoration: Option<Decoration>,
    pub branches: AcaciaBranches,
}

#[derive(Debug)]
pub struct AcaciaBranches {
    pub length: Range,
    pub position: Range,
    pub chance: Odds,
    pub canopy: Option<&'static Canopy>,
}

/// `fancy_trunk` (fancy oaks).
#[derive(Debug)]
pub struct FancyTrunk {
    pub block: Block,
    pub width: i32,
    pub height_base: i32,
    pub height_variance: i32,
    pub height_scale: f32,
    pub branch_slope: f32,
    pub branch_density: f32,
    pub min_altitude_factor: f32,
    pub foliage_altitude_factor: f32,
    pub width_scale: f32,
}

/// `mega_trunk` (giant spruces and pines, mega jungle trees).
#[derive(Debug)]
pub struct MegaTrunk {
    pub block: Block,
    pub width: i32,
    pub height_base: i32,
    pub height_intervals: &'static [i32],
    pub decoration: Option<Decoration>,
    pub branches: Option<MegaBranches>,
}

#[derive(Debug)]
pub struct MegaBranches {
    pub length: i32,
    pub slope: f32,
    pub interval: Range,
    pub altitude_factor: (f32, f32),
    pub canopy: Option<&'static Canopy>,
}

/// `mangrove_trunk`.
#[derive(Debug)]
pub struct MangroveTrunk {
    pub block: Block,
    pub height_base: i32,
    pub height_rand_a: i32,
    pub height_rand_b: i32,
    pub branch_length: Range,
    pub branch_steps: Range,
    pub decoration: Option<Decoration>,
}

/// `cherry_trunk`; branch logs lie along x or z.
#[derive(Debug)]
pub struct CherryTrunk {
    pub block: Block,
    pub block_x: Block,
    pub block_z: Block,
    pub height_base: i32,
    pub height_intervals: &'static [i32],
    /// one branch, two branches, two branches and the trunk.
    pub weights: [i32; 3],
    pub horizontal_length: Range,
    pub start_offset: Range,
    pub end_offset: Range,
}

/// `fallen_trunk`: a stump and a log lying beside it.
#[derive(Debug)]
pub struct FallenTrunk {
    pub block: Block,
    pub block_x: Block,
    pub block_z: Block,
    pub log_length: Range,
    pub height_modifier: Range,
    pub stump_height: Range,
    /// Index into `FEATURES`, placed on every log of the lying part.
    pub log_decoration: Option<usize>,
    pub decoration: Option<Decoration>,
}

#[derive(Debug)]
pub enum Canopy {
    Simple(SimpleCanopy),
    Acacia { leaf: Block, size: i32, simplify: bool },
    Pine { leaf: Block, height: Range, base_radius: i32 },
    Spruce { leaf: Block, lower_offset: Range, upper_offset: Range, max_radius: Range },
    Fancy { leaf: Block, height: i32, radius: i32 },
    Mega { leaf: Block, height: Range, base_radius: i32, core_width: i32, simplify: bool },
    MegaPine { leaf: Block, height: Range, base_radius: i32, radius_step_modifier: f32, core_width: i32 },
    Roofed { leaf: Block, height: i32, core_width: i32, outer_radius: i32, inner_radius: i32 },
    Mangrove(MangroveCanopy),
    Cherry(CherryCanopy),
    RandomSpread { leaves: &'static [(Block, i32)], height: i32, radius: i32, attempts: i32 },
}

/// The bare `canopy` key: stacked squares with corners cut by `variation_chance`.
#[derive(Debug)]
pub struct SimpleCanopy {
    pub leaf: Block,
    pub offset: (i32, i32),
    pub min_width: i32,
    pub rise: i32,
    pub run: i32,
    pub variation_chance: &'static [Odds],
    pub decoration: Option<CanopyDecoration>,
}

/// `canopy_decoration` of the simple and mangrove canopies: vines hung beside leaves.
#[derive(Debug)]
pub struct CanopyDecoration {
    pub block: Decorative,
    pub chance: Odds,
    pub steps: Range,
}

#[derive(Debug)]
pub struct MangroveCanopy {
    pub height: Range,
    pub radius: Range,
    pub attempts: i32,
    pub leaves: &'static [(Block, i32)],
    pub hanging: Block,
    pub hanging_chance: Odds,
    pub decoration: Option<Decoration>,
}

#[derive(Debug)]
pub struct CherryCanopy {
    pub leaf: Block,
    pub height: Range,
    pub radius: Range,
    pub trunk_width: i32,
    pub wide_bottom_hole: Odds,
    pub corner_hole: Odds,
    pub hanging_leaves: Odds,
    pub hanging_leaves_extension: Odds,
}

/// An attachable decoration (`trunk_decoration`): per enabled side a chance roll, then runs of
/// each entry's block stepping `step` from the cell beside the log.
#[derive(Debug)]
pub struct Decoration {
    pub entries: &'static [(Decorative, Range)],
    pub chance: Odds,
    pub step: Step,
}

/// A decoration block; vines take the side they hang on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decorative {
    Vine,
    Block(Block),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Down,
    Up,
    Outward,
}

/// `mangrove_roots`.
#[derive(Debug)]
pub struct MangroveRoots {
    pub max_width: i32,
    pub max_length: i32,
    pub y_offset: Range,
    pub root: Block,
    pub muddy_root: Block,
    pub mud: Block,
    pub above_root: Option<(Block, Odds)>,
    pub may_grow_through: &'static [Block],
    pub decoration: Option<Decoration>,
}

/// `base_cluster` (giant spruces' podzol): circles of the base block around the trunk.
#[derive(Debug)]
pub struct BaseCluster {
    pub may_replace: &'static [Block],
    pub count: i32,
    pub radius: i32,
}
