//! The shapes of the lush caves' data-driven features (cave.rs, scripts/cave_data.py).

use super::tree::spec::Range;
use crate::block::Block;

/// `minecraft:snap_to_surface_feature` (143bb7720, cave.rs): from the position through open cells
/// of its biome to a surface within `range`, then `feature` beside or in it.
#[derive(Debug)]
pub struct SnapToSurface {
    pub feature: usize,
    pub range: i32,
    pub surface: Surface,
    pub allow_air: bool,
    pub allow_water: bool,
    pub allow_solid: bool,
    /// Place in the surface block, not beside it.
    pub embed: bool,
    /// When not empty, the surface must be one of these (states ignored).
    pub allowed: &'static [Block],
}

/// `minecraft:multiface_feature` (143b83e50, multiface.rs): `block` (its down-facing variant) on a
/// supporting face of the position or, failing that, of a neighbour.
#[derive(Debug)]
pub struct Multiface {
    pub block: Block,
    pub search_range: i32,
    /// FacingIDs in the order the flags push them: floor, ceiling, then walls north, east, south, west.
    pub faces: &'static [usize],
    pub can_place_on: &'static [Block],
    /// The same list by name, for the blocks only templates place.
    pub on_names: &'static [&'static str],
    pub spread_chance: f32,
}

/// `minecraft:sculk_patch_feature` (143ba12e0; sculk/).
#[derive(Debug)]
pub struct SculkPatch {
    /// Empty: any neighbour with BDS's solid flag.
    pub can_place_on: &'static [Block],
    pub cursors: i32,
    pub charge: i32,
    pub attempts: i32,
    pub growth_rounds: i32,
    pub spread_rounds: i32,
    pub catalyst_chance: f32,
    pub extra_growths: Range,
}

/// `minecraft:surface_relative_threshold_feature` (143bc69b0): `feature` only more than
/// `min_depth` blocks under the preliminary surface of the quart column.
#[derive(Debug)]
pub struct SurfaceThreshold {
    pub feature: usize,
    pub min_depth: i32,
}

/// `minecraft:partially_exposed_blob_feature` (blob.rs).
#[derive(Debug)]
pub struct PartiallyExposedBlob {
    pub block: Block,
    pub radius: i32,
    pub probability: f32,
    /// The FacingID that may stay open.
    pub exposed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Ceiling,
    Floor,
    /// A wall (a vertical surface), sides in shuffled order.
    RandomVertical,
    /// The ceiling or the floor (a horizontal surface), by a boolean.
    RandomHorizontal,
}

/// `minecraft:vegetation_patch_feature` (143bc8310): a patch of `ground` on a floor or ceiling,
/// then `vegetation` on its cells with `vegetation_chance`.
#[derive(Debug)]
pub struct VegetationPatch {
    pub replaceable: &'static [Block],
    pub ground: Block,
    pub vegetation: usize,
    pub depth: Range,
    pub extra_deep_chance: f32,
    pub vertical_range: i32,
    pub vegetation_chance: f32,
    pub radius: Range,
    pub extra_edge_chance: f32,
    pub waterlogged: bool,
    pub ceiling: bool,
}

/// `minecraft:growing_plant_feature` (144c15200): a column of body blocks with a head, its height
/// picked by weight from `heights`.
#[derive(Debug)]
pub struct GrowingPlant {
    pub heights: &'static [(Range, f32)],
    pub up: bool,
    /// The head's `growing_plant_age`, when the range is not (0, 0).
    pub age: Range,
    pub body: &'static [(Block, f32)],
    pub head: &'static [(Block, f32)],
    pub allow_water: bool,
}
