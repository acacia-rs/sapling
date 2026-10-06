//! The seven climate noises (`OverworldNoises3d::make` 1481d9b10) and what BDS samples from them
//! per quart column: shifted coordinates, climate, and the terrain shaper's outputs
//! (1481df0e0 + 1481def00, cached per chunk by 14822cb50).

use crate::noise::NormalNoise;
use crate::random::PositionalFactory;
use crate::shaper::{ShaperPoint, TerrainShaper};

#[derive(Debug, Clone)]
pub struct ClimateNoises {
    pub temperature: NormalNoise,
    pub vegetation: NormalNoise,
    pub continentalness: NormalNoise,
    pub erosion: NormalNoise,
    pub ridge: NormalNoise,
    pub offset: NormalNoise,
    pub jagged: NormalNoise,
    pub shaper: TerrainShaper,
}

/// One quart column's sample, BDS's `ChunkLocalNoiseCache::CacheEntry`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuartClimate {
    pub shifted_x: f32,
    pub shifted_z: f32,
    pub continentalness: f32,
    pub weirdness: f32,
    pub erosion: f32,
    /// Includes the global offset.
    pub offset: f32,
    /// Times 128.
    pub factor: f32,
    /// The jagged noise already weighted by the jaggedness spline.
    pub jagged: f32,
    pub temperature: f32,
    pub humidity: f32,
}

impl ClimateNoises {
    pub fn new(factory: &PositionalFactory) -> Self {
        let noise = |name: &str, first: i32, amplitudes: &[f32]| NormalNoise::new(factory, &format!("minecraft:{name}"), first, amplitudes);
        Self {
            temperature: noise("temperature", -10, &[1.5, 0.0, 1.0, 0.0, 0.0, 0.0]),
            vegetation: noise("vegetation", -8, &[1.0, 1.0, 0.0, 0.0, 0.0, 0.0]),
            continentalness: noise("continentalness", -9, &[1.0, 1.0, 2.0, 2.0, 2.0, 1.0, 1.0, 1.0, 1.0]),
            erosion: noise("erosion", -9, &[1.0, 1.0, 0.0, 1.0, 1.0]),
            ridge: noise("ridge", -7, &[1.0, 2.0, 1.0, 0.0, 0.0, 0.0]),
            offset: noise("offset", -3, &[1.0, 1.0, 1.0, 0.0]),
            jagged: noise("jagged", -16, &[1.0; 16]),
            shaper: TerrainShaper::overworld(),
        }
    }

    /// Java's `shift_x`/`shift_z`: the offset noise times 4 added to quart coordinates.
    pub fn shift(&self, qx: i32, qz: i32) -> (f32, f32) {
        let (x, z) = (qx as f32, qz as f32);
        (x + self.offset.sample(x, 0.0, z) * 4.0, self.offset.sample(z, x, 0.0) * 4.0 + z)
    }

    /// Continentalness, erosion and weirdness at shifted coordinates, as a shaper point.
    pub fn shaper_point(&self, shifted_x: f32, shifted_z: f32) -> ShaperPoint {
        let continentalness = self.continentalness.sample(shifted_x, 0.0, shifted_z);
        let erosion = self.erosion.sample(shifted_x, 0.0, shifted_z);
        let weirdness = self.ridge.sample(shifted_x, 0.0, shifted_z);
        ShaperPoint::new(continentalness, erosion, weirdness)
    }

    pub fn sample(&self, qx: i32, qz: i32) -> QuartClimate {
        let (shifted_x, shifted_z) = self.shift(qx, qz);
        let point = self.shaper_point(shifted_x, shifted_z);
        let jaggedness = self.shaper.jaggedness(&point);
        let jagged = if jaggedness > 0.0 {
            let noise = self.jagged.sample(qx as f32 * 4.0 * 1500.0, 0.0, qz as f32 * 4.0 * 1500.0);
            (if noise > 0.0 { jaggedness } else { jaggedness * 0.5 }) * noise
        } else {
            0.0
        };
        QuartClimate {
            shifted_x,
            shifted_z,
            continentalness: point.continents,
            weirdness: point.weirdness,
            erosion: point.erosion,
            offset: self.shaper.offset(&point),
            factor: self.shaper.factor(&point),
            jagged,
            temperature: self.temperature.sample(shifted_x, 0.0, shifted_z),
            humidity: self.vegetation.sample(shifted_x, 0.0, shifted_z),
        }
    }
}
