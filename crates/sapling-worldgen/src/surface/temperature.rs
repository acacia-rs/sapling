//! `Biome::getTemperature` (14119c8c0): a biome's climate temperature at a block. Frozen oceans are
//! warmer in patches (Java's frozen temperature modifier) and every biome cools with height. The
//! freeze pass and the frozen ocean's surface builder ask it.

use std::sync::OnceLock;

use super::biome_temperature;
use crate::noise::SimplexNoise;
use crate::random::Mt19937;

/// Frozen ocean and deep frozen ocean: the biomes with `minecraft:partially_frozen`.
const PARTIALLY_FROZEN: [u32; 2] = [46, 47];
/// Temperature falls above sea level (63) plus 17. 14119c8c0 and 1411a3e60 pick +17 and a noise factor
/// of 8 on version flags read here as "legacy"; the oracle fits them (+1 and 4 snow 1.1k cells too low).
const COOLING_Y: i32 = 80;

/// `PerlinSimplexNoise(seed, octaves)` (145e276f0): an MT on the seed, per octave three floats and a shuffle.
fn simplex_octaves(seed: u32, octaves: usize) -> Vec<SimplexNoise> {
    let mut mt = Mt19937::new(seed);
    (0..octaves)
        .map(|_| {
            let [a, b, c] = [mt.next_float(), mt.next_float(), mt.next_float()];
            SimplexNoise::new([c, b, a], |bound| mt.next_int(bound as i32))
        })
        .collect()
}

/// Octave k samples `(x, z) * scale * k` and weighs it `1 / k`, k halving from 1.
fn sum(octaves: &[SimplexNoise], x: i32, z: i32, scale: f32) -> f32 {
    let (x, z) = (x as f32 * scale, z as f32 * scale);
    let (mut total, mut k) = (0.0f32, 1.0f32);
    for octave in octaves {
        total += octave.value(x * k, z * k) / k;
        k *= 0.5;
    }
    total
}

/// The biome info noise, one octave from an MT seeded 2345 whatever the world: also Molang's
/// `query.noise` and the legacy flower features' noise.
pub fn info_noise() -> &'static SimplexNoise {
    static NOISE: OnceLock<Vec<SimplexNoise>> = OnceLock::new();
    &NOISE.get_or_init(|| simplex_octaves(2345, 1))[0]
}

fn temperature_noise() -> &'static [SimplexNoise] {
    static NOISE: OnceLock<Vec<SimplexNoise>> = OnceLock::new();
    NOISE.get_or_init(|| simplex_octaves(1234, 1))
}

fn frozen_noise() -> &'static [SimplexNoise] {
    static NOISE: OnceLock<Vec<SimplexNoise>> = OnceLock::new();
    NOISE.get_or_init(|| simplex_octaves(3456, 3))
}

/// None for biomes without a climate here.
pub fn temperature(biome: u32, [x, y, z]: [i32; 3]) -> Option<f32> {
    let mut t = biome_temperature(biome)?;
    let info = std::slice::from_ref(info_noise());
    if PARTIALLY_FROZEN.contains(&biome) && sum(frozen_noise(), x, z, 0.05) + sum(info, x, z, 0.2) < 0.3 && sum(info, x, z, 0.09) < 0.8 {
        t = 0.2;
    }
    if y > COOLING_Y {
        // The chunk caches the noise per column as a signed byte, 64 per unit (1411a3e60).
        let cached = ((sum(temperature_noise(), x, z, 0.125) * 8.0 * 8.0) as i32 as i8) as f32 * 0.125;
        t += (y as f32 + cached - COOLING_Y as f32) * -0.05 / 40.0;
    }
    Some(t)
}
