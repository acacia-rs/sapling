//! Java `NormalNoise` as BDS computes it (`NormalNoise::getValue` 145e26660).

use super::PerlinNoise;
use crate::random::{Mt19937, PositionalFactory};

const INPUT_FACTOR: f32 = 1.018_126_8;

#[derive(Debug, Clone)]
pub struct NormalNoise {
    first: PerlinNoise,
    second: PerlinNoise,
    value_factor: f32,
}

impl NormalNoise {
    /// Both halves fork from the same named random, first then second.
    pub fn new(factory: &PositionalFactory, name: &str, first_octave: i32, amplitudes: &[f32]) -> Self {
        let mut random = factory.from_hash_of(name);
        let positional = random.fork_positional();
        let first = PerlinNoise::new(&positional, first_octave, amplitudes);
        let positional = random.fork_positional();
        let second = PerlinNoise::new(&positional, first_octave, amplitudes);
        Self { first, second, value_factor: value_factor(amplitudes) }
    }

    /// Both halves in legacy mode from one MT, first then second.
    pub fn from_mt(random: &mut Mt19937, first_octave: i32, amplitudes: &[f32]) -> Self {
        let first = PerlinNoise::from_mt(random, first_octave, amplitudes);
        let second = PerlinNoise::from_mt(random, first_octave, amplitudes);
        Self { first, second, value_factor: value_factor(amplitudes) }
    }

    pub fn sample(&self, x: f32, y: f32, z: f32) -> f32 {
        let a = self.first.sample(x, y, z);
        let b = self.second.sample(x * INPUT_FACTOR, y * INPUT_FACTOR, z * INPUT_FACTOR);
        (a + b) * self.value_factor
    }
}

/// `(1/6) / (0.1 * (1 + 1/(max-min+1)))` over the non-zero amplitude indices, in f32 (1481d9b10).
fn value_factor(amplitudes: &[f32]) -> f32 {
    let used: Vec<usize> = amplitudes.iter().enumerate().filter(|(_, a)| **a != 0.0).map(|(i, _)| i).collect();
    let span = (used[used.len() - 1] - used[0]) as f32;
    0.166_666_67 / ((1.0 / (span + 1.0) + 1.0) * 0.1)
}
