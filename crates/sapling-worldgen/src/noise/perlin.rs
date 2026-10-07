//! Multi-octave Perlin noise (BDS `MultiOctaveNoiseImpl`, ctor 145e24e60, positional mode).

use super::ImprovedNoise;
use crate::random::{Mt19937, PositionalFactory};

#[derive(Debug, Clone)]
struct Octave {
    noise: ImprovedNoise,
    input_factor: f32,
    /// Amplitude times the octave's value factor, one f32 product as BDS stores it.
    weight: f32,
}

#[derive(Debug, Clone)]
pub struct PerlinNoise {
    /// Enabled octaves, lowest frequency first (BDS sums in this order).
    octaves: Vec<Octave>,
}

impl PerlinNoise {
    /// Each octave is seeded by `"octave_" + n` from a fresh positional fork; zero amplitudes
    /// still take their slot but are never sampled.
    pub fn new(factory: &PositionalFactory, first_octave: i32, amplitudes: &[f32]) -> Self {
        let count = amplitudes.len() as u32;
        let denominator = !(-1i32 << count) as f32;
        let mut octaves = Vec::new();
        for (i, &amplitude) in amplitudes.iter().enumerate() {
            if amplitude == 0.0 {
                continue;
            }
            let octave = first_octave + i as i32;
            let mut random = factory.from_hash_of(&format!("octave_{octave}"));
            let noise = ImprovedNoise::new(&mut random);
            let value_factor = (1i32 << (count - 1 - i as u32)) as f32 / denominator;
            octaves.push(Octave { noise, input_factor: 2f64.powi(octave) as f32, weight: value_factor * amplitude });
        }
        Self { octaves }
    }

    /// The legacy mode of 145e24e60 (Java's `PerlinNoise(RandomSource, ...)`) on an MT: octave 0's noise
    /// is drawn first, then from the octave below 0 down, each octave beyond the amplitudes or with
    /// amplitude 0 skipping 262 draws.
    pub fn from_mt(random: &mut Mt19937, first_octave: i32, amplitudes: &[f32]) -> Self {
        let count = amplitudes.len() as i32;
        let denominator = !(-1i32 << count) as f32;
        let mut noises: Vec<Option<ImprovedNoise>> = vec![None; amplitudes.len()];
        let top = -first_octave;
        let first = ImprovedNoise::new(random);
        if (0..count).contains(&top) && amplitudes[top as usize] != 0.0 {
            noises[top as usize] = Some(first);
        }
        for i in (0..top).rev() {
            if i < count && amplitudes[i as usize] != 0.0 {
                noises[i as usize] = Some(ImprovedNoise::new(random));
            } else {
                random.consume_count(262);
            }
        }
        let octaves = noises
            .into_iter()
            .enumerate()
            .filter_map(|(i, noise)| {
                let value_factor = (1i32 << (count - 1 - i as i32)) as f32 / denominator;
                let input_factor = 2f64.powi(first_octave + i as i32) as f32;
                noise.map(|noise| Octave { noise, input_factor, weight: value_factor * amplitudes[i] })
            })
            .collect();
        Self { octaves }
    }

    /// The x/y/z here are already multiplied by any outer input factor.
    pub fn sample(&self, x: f32, y: f32, z: f32) -> f32 {
        let mut sum = 0.0f32;
        for octave in &self.octaves {
            let f = octave.input_factor;
            let n = &octave.noise;
            sum += n.sample_offset(x * f + n.xo, y * f + n.yo, z * f + n.zo) * octave.weight;
        }
        sum
    }
}
