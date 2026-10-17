//! BDS's legacy `PerlinNoise` on a sequential random (built by 145e27060, sampled by 145e27220): improved octaves
//! from `LegacyBlendedNoise::addOctave`, octave k at `position * 2^-k` divided by `2^-k`, and a normalisation the
//! callers multiply in themselves.

use super::{ImprovedNoise, NoiseRandom, legacy_octaves};

#[derive(Debug, Clone)]
pub struct LegacyPerlin {
    octaves: Vec<ImprovedNoise>,
    /// The inverse of the summed inverse factors (+0x20).
    pub normalisation: f32,
}

impl LegacyPerlin {
    pub fn new(random: &mut impl NoiseRandom, count: usize) -> Self {
        let mut sum = 0.0f32;
        let mut factor = 1.0f32;
        for _ in 0..count {
            sum += 1.0 / factor;
            factor *= 0.5;
        }
        Self { octaves: legacy_octaves(random, count), normalisation: 1.0 / sum }
    }

    pub fn value(&self, [x, y, z]: [f32; 3]) -> f32 {
        let mut total = 0.0f32;
        let mut factor = 1.0f32;
        for octave in &self.octaves {
            total = total + octave.sample_offset(x * factor + octave.xo, y * factor + octave.yo, z * factor + octave.zo) / factor;
            factor *= 0.5;
        }
        total
    }
}
