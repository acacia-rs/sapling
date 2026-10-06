//! Java 1.18.1 `TerrainShaper`: offset, factor and jaggedness as splines over continentalness,
//! erosion, weirdness and ridges.

mod overworld;
mod spline;
#[cfg(test)]
mod tests;

pub use spline::{ShaperPoint, Spline, peaks_and_valleys};

/// BDS keeps both on the shaper object (148068d20 writes them at +0x1f8).
const GLOBAL_OFFSET: f32 = -0.50375;
/// BDS densities are Java's times 128; the factor carries the scale.
pub const DENSITY_SCALE: f32 = 128.0;

#[derive(Debug, Clone)]
pub struct TerrainShaper {
    offset: Spline,
    factor: Spline,
    jaggedness: Spline,
}

impl TerrainShaper {
    pub fn overworld() -> Self {
        Self { offset: overworld::offset_spline(), factor: overworld::factor_spline(), jaggedness: overworld::jaggedness_spline() }
    }

    /// The offset spline without the global offset (the aquifer's deep-dark test reads it raw).
    pub fn raw_offset(&self, point: &ShaperPoint) -> f32 {
        self.offset.apply(point)
    }

    pub fn offset(&self, point: &ShaperPoint) -> f32 {
        self.raw_offset(point) + GLOBAL_OFFSET
    }

    pub fn global_offset() -> f32 {
        GLOBAL_OFFSET
    }

    /// Scaled by 128 (`DENSITY_SCALE`).
    pub fn factor(&self, point: &ShaperPoint) -> f32 {
        self.factor.apply(point) * DENSITY_SCALE
    }

    pub fn jaggedness(&self, point: &ShaperPoint) -> f32 {
        self.jaggedness.apply(point)
    }
}
