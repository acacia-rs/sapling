//! The shapes of transcribed feature rules and features (data.rs, scripts/features_to_rust.py).

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

#[derive(Debug)]
pub enum Feature {
    Ore(Ore),
    Disk(Disk),
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
