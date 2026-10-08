//! A biome's `minecraft:surface_builder`, `surface_material_adjustments` and steep-slope
//! `mountain_parameters`, as the vanilla behaviour pack defines them (biomes.rs).

use crate::block::Block;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Materials {
    pub top: Block,
    pub mid: Block,
    pub sea_floor: Block,
    pub foundation: Block,
    pub sea: Block,
}

/// Replaces materials where the adjustment noise at `(x, z) * noise_scale` is in `range`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Adjustment {
    pub noise_scale: f32,
    pub range: [f32; 2],
    pub top_material: Option<Block>,
    pub mid_material: Option<Block>,
    pub sea_floor_material: Option<Block>,
    pub foundation_material: Option<Block>,
}

/// The material for slopes facing the flagged directions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Steep {
    pub material: Block,
    pub north: bool,
    pub south: bool,
    pub east: bool,
    pub west: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mesa {
    pub bryce_pillars: bool,
    pub has_forest: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Builder {
    Overworld,
    FrozenOcean,
    Swamp { max_puddle_depth: i32 },
    Mesa(Mesa),
}

/// `minecraft:noise_gradient` (gradient.rs): per block, the block of the band whose `(min, max]`
/// holds the named NormalNoise; blocks in `keep` stay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gradient {
    pub noise: &'static str,
    pub first_octave: i32,
    pub amplitudes: &'static [f32],
    pub keep: &'static [Block],
    pub bands: &'static [(f32, f32, Block)],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceSpec {
    pub builder: Builder,
    pub sea_floor_depth: i32,
    pub temperature: f32,
    pub materials: Materials,
    pub adjustments: &'static [Adjustment],
    pub steep: Option<Steep>,
}
