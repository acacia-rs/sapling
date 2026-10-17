//! The Nether's biomes: BDS's legacy layer stack (built at the end of the generator's constructor 1481e6d60).
//! Four noise layers (14824a450 seeds them, 14824a9a0 fills) give each quart column a climate, the multi-noise
//! layer (14824b460) picks the biome whose `minecraft:multinoise_generation_rules` lies nearest, and Java 1.14's
//! voronoi zoom (14824b970, 14824ff80) takes it to blocks. One biome per column (`fillBiomes` 148251120).

use crate::noise::{ImprovedNoise, legacy_octaves};
use crate::random::Mt19937;

pub const HELL: u8 = 8;
pub const SOULSAND_VALLEY: u8 = 178;
pub const CRIMSON_FOREST: u8 = 179;
pub const WARPED_FOREST: u8 = 180;
pub const BASALT_DELTAS: u8 = 181;

/// Candidates in registry (id) order with their temperature, humidity, altitude, weirdness and weight; the
/// first nearest wins, hell is the default.
const RULES: [(u8, [f32; 5]); 5] = [
    (HELL, [0.0, 0.0, 0.0, 0.0, 0.0]),
    (SOULSAND_VALLEY, [0.0, -0.5, 0.0, 0.0, 0.0]),
    (CRIMSON_FOREST, [0.4, 0.0, 0.0, 0.0, 0.0]),
    (WARPED_FOREST, [0.0, 0.5, 0.0, 0.0, 0.375]),
    (BASALT_DELTAS, [-0.5, 0.0, 0.0, 0.0, 0.175]),
];

/// The voronoi layer's salt (its object +0x10 in the constructor).
const ZOOM_SALT: i64 = -0x7945_4928_9d90_b428;

/// One noise layer: Java 1.16's two-octave nether `NormalNoise`, BDS's float `PerlinNoise` pair on an MT.
struct NoiseLayer {
    first: Vec<ImprovedNoise>,
    second: Vec<ImprovedNoise>,
    value_factor: f32,
}

/// `PerlinNoise` (145e27060): its octaves' sum normalised by the inverse of the summed inverse factors.
fn normalisation(octaves: usize) -> f32 {
    let mut sum = 0.0f32;
    let mut factor = 1.0f32;
    for _ in 0..octaves {
        sum += 1.0 / factor;
        factor *= 0.5;
    }
    1.0 / sum
}

/// `PerlinNoise::getValue` (145e27220): octave k at `position * 2^-k`, divided by `2^-k`.
fn perlin(octaves: &[ImprovedNoise], [x, y, z]: [f32; 3]) -> f32 {
    let mut total = 0.0f32;
    let mut factor = 1.0f32;
    for octave in octaves {
        total = total + octave.sample_offset(x * factor + octave.xo, y * factor + octave.yo, z * factor + octave.zo) / factor;
        factor *= 0.5;
    }
    total
}

impl NoiseLayer {
    /// Layer `index` (temperature 0, humidity 1, altitude 2, weirdness 3) seeds its MT with `seed + index`.
    fn new(seed: i64, index: u32) -> Self {
        const OCTAVES: u32 = 2;
        let mut random = Mt19937::new((seed as u32).wrapping_add(index));
        let first = legacy_octaves(&mut random, OCTAVES as usize);
        let second = legacy_octaves(&mut random, OCTAVES as usize);
        let value_factor = 0.166_666_67 / ((1.0 / (OCTAVES + 1) as f32 + 1.0) * 0.1);
        Self { first, second, value_factor }
    }

    fn value(&self, qx: i32, qz: i32) -> f32 {
        let (x, z) = (qx as f32, qz as f32);
        let a = perlin(&self.first, [x * 0.015625, 0.0, z * 0.015625]) * normalisation(self.first.len());
        let k = 1.018_126_8_f32;
        let b = perlin(&self.second, [x * k * 0.015625, 0.0, z * k * 0.015625]) * normalisation(self.second.len());
        (b + a) * self.value_factor
    }
}

pub struct NetherBiomes {
    climate: [NoiseLayer; 4],
    zoom_seed: i64,
}

const M: i64 = 0x5851_f42d_4c95_7f2d;
const A: i64 = 0x1405_7b7e_f767_814f;

/// One step of the layers' LCG (Java's `LinearCongruentialGenerator.next`).
fn mix(state: i64, add: i64) -> i64 {
    state.wrapping_mul(state.wrapping_mul(M).wrapping_add(A)).wrapping_add(add)
}

impl NetherBiomes {
    pub fn new(seed: i64) -> Self {
        let zoom_seed = mix(mix(mix(seed, ZOOM_SALT), ZOOM_SALT), ZOOM_SALT);
        Self { climate: std::array::from_fn(|i| NoiseLayer::new(seed, i as u32)), zoom_seed }
    }

    /// The multi-noise layer at a quart column.
    pub fn quart_biome(&self, qx: i32, qz: i32) -> u8 {
        let climate = self.climate.each_ref().map(|layer| layer.value(qx, qz));
        let mut best = (HELL, f32::MAX);
        for (biome, [t, h, a, w, weight]) in RULES {
            let (dt, dh, da, dw) = (t - climate[0], h - climate[1], a - climate[2], w - climate[3]);
            let distance = dt * dt + dh * dh + da * da + dw * dw + weight * weight;
            if distance < best.1 {
                best = (biome, distance);
            }
        }
        best.0
    }

    /// A corner's jitter: two `nextInt(1024)` from the chunk seed at (x, z) (14824ff80).
    fn jitter(&self, x: i32, z: i32) -> [f32; 2] {
        let (x, z) = (i64::from(x), i64::from(z));
        let mut state = mix(mix(mix(mix(self.zoom_seed, x), z), x), z);
        std::array::from_fn(|_| {
            let draw = ((state >> 24) & 0x3ff) as i32;
            state = mix(state, self.zoom_seed);
            (draw as f32 * 0.000_976_562_5 + -0.5) * 3.6
        })
    }

    /// The biome of a block column: the voronoi corner nearest it of the four quart cells around (x-2, z-2).
    pub fn biome(&self, x: i32, z: i32) -> u8 {
        let (x, z) = (x - 2, z - 2);
        let (qx, qz) = (x >> 2, z >> 2);
        let (k, j) = ((x & 3) as f32, (z & 3) as f32);
        let corners = [(0, 0), (1, 0), (0, 1), (1, 1)];
        let distances = corners.map(|(cx, cz)| {
            let [dx, dz] = self.jitter((qx + cx) * 4, (qz + cz) * 4);
            let (dx, dz) = (dx + 4.0 * cx as f32 - k, dz + 4.0 * cz as f32 - j);
            dx * dx + dz * dz
        });
        let pick = (0..3).find(|&i| (0..4).all(|o| o == i || distances[i] < distances[o])).unwrap_or(3);
        let (cx, cz) = corners[pick];
        self.quart_biome(qx + cx, qz + cz)
    }
}
