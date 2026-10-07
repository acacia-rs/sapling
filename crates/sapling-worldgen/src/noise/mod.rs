//! BDS's noise primitives. All arithmetic is f32 in BDS's operation order; Java's `wrap()` is
//! absent, as in BDS.

mod blended;
mod improved;
mod normal;
mod perlin;
mod simplex;
#[cfg(test)]
mod tests;

pub use blended::{BlendedNoise, CELL_COLUMNS, CELL_COUNT, CELL_ROWS, CellGrid};
pub use improved::ImprovedNoise;
pub use normal::NormalNoise;
pub use perlin::PerlinNoise;
pub use simplex::SimplexNoise;

use crate::random::{Mt19937, Xoroshiro};

/// `IRandom` over BDS's MT (vtable 14a7125a0): `nextDouble` is one raw draw times 2^-32 (1408741f0).
impl NoiseRandom for Mt19937 {
    fn next_double(&mut self) -> f64 {
        self.next_u32() as f64 * (1.0 / 4_294_967_296.0)
    }

    fn next_float(&mut self) -> f32 {
        Mt19937::next_float(self)
    }

    fn next_int_bounded(&mut self, bound: u32) -> i32 {
        self.next_int(bound as i32)
    }
}

/// The draws an octave's constructor takes; lets tests feed Java's legacy LCG.
pub trait NoiseRandom {
    fn next_double(&mut self) -> f64;
    fn next_float(&mut self) -> f32;
    fn next_int_bounded(&mut self, bound: u32) -> i32;
}

impl NoiseRandom for Xoroshiro {
    fn next_double(&mut self) -> f64 {
        Xoroshiro::next_double(self)
    }

    fn next_float(&mut self) -> f32 {
        Xoroshiro::next_float(self)
    }

    fn next_int_bounded(&mut self, bound: u32) -> i32 {
        Xoroshiro::next_int_bounded(self, bound)
    }
}
