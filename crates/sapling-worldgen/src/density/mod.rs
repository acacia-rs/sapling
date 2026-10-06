//! The density stage of `OverworldGeneratorMultinoise::_prepareHeights` (14822eb60): density
//! cells, noise caves, noodles and aquifers. Densities are Java's times 128 throughout.

mod aquifer;
mod cavifier;
mod cells;
mod noises;

pub use aquifer::{AquiferAnswer, AquiferCache, FluidStatus, SEA_LEVEL};
pub use cells::{Cells, MIN_CELL_Y, bottom_slide, cell_index, top_slide};
pub use noises::{AquiferNoises, CaveNoises, NoodleNoises};

use crate::climate::ClimateNoises;
use crate::noise::BlendedNoise;
use crate::random::PositionalFactory;

/// Returned when no cell of the column reaches the preliminary-surface threshold.
pub const NO_SURFACE: i32 = 0x7fff;

pub struct Density {
    pub climate: ClimateNoises,
    blended: BlendedNoise,
    caves: CaveNoises,
    noodles: NoodleNoises,
    aquifer_noises: AquiferNoises,
}

impl Density {
    pub fn new(factory: &PositionalFactory) -> Self {
        Self {
            climate: ClimateNoises::new(factory),
            blended: BlendedNoise::new(&mut factory.from_hash_of("minecraft:terrain")),
            caves: CaveNoises::new(factory),
            noodles: NoodleNoises::new(factory),
            aquifer_noises: AquiferNoises::new(factory),
        }
    }

    /// The highest cell corner whose cave-free, jag-free density clears the threshold
    /// (BDS 14822e160, Java `getPreliminarySurfaceLevel`), at a quart column.
    pub fn preliminary_surface(&self, qx: i32, qz: i32) -> i32 {
        let (shifted_x, shifted_z) = self.climate.shift(qx, qz);
        let point = self.climate.shaper_point(shifted_x, shifted_z);
        let offset = self.climate.shaper.offset(&point);
        let factor = self.climate.shaper.factor(&point);
        for cy in (0..40).rev() {
            let y = (cy + MIN_CELL_Y) * 8;
            let height = (y as f32 * -0.007_812_5 + 1.0 + offset) * factor;
            let density = height * if height > 0.0 { 4.0 } else { 1.0 } + -90.0;
            let density = bottom_slide(top_slide(density, 48 - 8 - cy), cy);
            if density > 50.0 {
                return y;
            }
        }
        NO_SURFACE
    }
}
