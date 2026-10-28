//! A `SculkSpreader`'s settings (ctor 148b47ba0, fields +0..+0x10) and what a live world hears of
//! its work, which a feature's view does not: BDS passes the level only outside world generation.

use crate::block::Block;
use crate::feature::spreader::Cells;
use crate::feature::view::View;

pub struct Spreader {
    /// Cursors are never merged, and one that strays too far from the origin loses its charge.
    pub worldgen: bool,
    pub growth_spawn_cost: i32,
    pub no_growth_radius: i32,
    pub charge_decay_rate: i32,
    pub additional_decay_rate: i32,
}

impl Spreader {
    /// `SculkPatchFeature`'s.
    pub const WORLDGEN: Spreader = Spreader { worldgen: true, growth_spawn_cost: 50, no_growth_radius: 1, charge_decay_rate: 5, additional_decay_rate: 10 };
    /// A sculk catalyst's (`SculkCatalystBlockActor` ctor 148802230 → 148b47480, constants at 14a94acc0).
    pub const LEVEL: Spreader = Spreader { worldgen: false, growth_spawn_cost: 10, no_growth_radius: 4, charge_decay_rate: 10, additional_decay_rate: 5 };

    /// What a vein turns to sculk under it: `sculk_replaceable_world_gen` (148b47d30), or the level's
    /// `sculk_replaceable` (148b47600), the same less the six deepslate brick, tile and polished blocks.
    pub fn replaceable(&self, block: Block) -> bool {
        let name = block.name();
        if !self.worldgen && name.ends_with("deepslate_bricks") | name.ends_with("deepslate_tiles") | matches!(name, "cobbled_deepslate" | "polished_deepslate") {
            return false;
        }
        super::replaceable(block)
    }
}

/// A sound a spreader plays at a cursor (LevelSoundEvent).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    /// 379 `block.sculk.spread`: veins spread, or a vein turned the block behind it to sculk.
    Spread,
    /// 381 `block.sculk_sensor.place`.
    SensorPlaced,
    /// 382 `block.sculk_shrieker.place`.
    ShriekerPlaced,
}

/// The world a spreader works in. A live one is told what BDS tells the level; the defaults are
/// world generation's, where nothing is told and every chunk ticks.
pub trait SculkWorld: Cells {
    fn sound(&mut self, _at: [i32; 3], _sound: Sound) {}
    /// LevelEvent 2037 `SculkCharge` for the cursors on one cell: their charge and the face bits of one.
    fn charge(&mut self, _at: [i32; 3], _charge: i32, _facings: i32) {}
    /// LevelEvent 2038 `SculkChargePop`: a cursor ran out of charge.
    fn charge_pop(&mut self, _at: [i32; 3]) {}
    /// Whether the cursor at `at` moves this tick (its chunk ticks).
    fn ticking(&self, _at: [i32; 3]) -> bool {
        true
    }
}

impl SculkWorld for View<'_> {}
