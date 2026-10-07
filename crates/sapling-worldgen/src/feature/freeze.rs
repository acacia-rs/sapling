//! The freeze pass that ends `decorationPostProcessChunk` (1482143a0): per column of the decorated
//! chunk, x outer and z inner, water under the rain top turns to ice (`Biome::shouldFreeze`
//! 14119e860) and the rain top gets a snow layer (`Biome::shouldSnow` 14119e740), both by
//! `Biome::getTemperature` (14119c8c0).

use std::sync::OnceLock;

use super::molang::noise as info_noise;
use super::view::View;
use crate::biome::ChunkBiomes;
use crate::block::Block;
use crate::noise::SimplexNoise;
use crate::random::Mt19937;
use crate::surface::biome_temperature;
use crate::terrain::{HEIGHT, MIN_Y};

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

fn temperature_noise() -> &'static [SimplexNoise] {
    static NOISE: OnceLock<Vec<SimplexNoise>> = OnceLock::new();
    NOISE.get_or_init(|| simplex_octaves(1234, 1))
}

fn frozen_noise() -> &'static [SimplexNoise] {
    static NOISE: OnceLock<Vec<SimplexNoise>> = OnceLock::new();
    NOISE.get_or_init(|| simplex_octaves(3456, 3))
}

/// `Biome::getTemperature`; None for biomes without a climate here.
fn temperature(biome: u32, [x, y, z]: [i32; 3]) -> Option<f32> {
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

fn cold(biomes: &ChunkBiomes, at: [i32; 3]) -> bool {
    if !(MIN_Y..MIN_Y + HEIGHT as i32).contains(&at[1]) {
        return false;
    }
    let biome = biomes.biome(at[0].rem_euclid(16) as usize, at[1], at[2].rem_euclid(16) as usize);
    temperature(biome, at).is_some_and(|t| t <= 0.15)
}

/// One above the highest block that stops rain (140a09950): anything but air and plants.
fn rain_top(view: &View, x: i32, z: i32) -> i32 {
    (MIN_Y..MIN_Y + HEIGHT as i32).rev().find(|&y| stops_rain(view.block(x, y, z))).map_or(MIN_Y, |y| y + 1)
}

fn stops_rain(block: Block) -> bool {
    block != Block::Air && (!block.is_plant() || block.is_crop_block())
}

/// `TopSnowBlock` survives on blocks that block motion, except ice (oracle: packed ice holds it).
fn holds_snow(block: Block) -> bool {
    block.blocks_motion() && !matches!(block, Block::Ice | Block::PowderSnow)
}

pub fn freeze_chunk(chunk: (i32, i32), biomes: &ChunkBiomes, view: &mut View) {
    for x in chunk.0 * 16..chunk.0 * 16 + 16 {
        for z in chunk.1 * 16..chunk.1 * 16 + 16 {
            let top = rain_top(view, x, z);
            let below = top - 1;
            if view.block(x, below, z) == Block::Water && cold(biomes, [x, below, z]) {
                view.set_block_quiet(x, below, z, Block::Ice);
            }
            if view.block(x, top, z) == Block::Air && holds_snow(view.block(x, below, z)) && cold(biomes, [x, top, z]) {
                view.set_block_quiet(x, top, z, Block::SnowLayer);
            }
        }
    }
}
