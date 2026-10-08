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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Ceiling,
    Floor,
    RandomHorizontal,
    RandomVertical,
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
