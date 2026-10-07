//! The `minecraft:mesa` surface builder (build 1487dcb80, init 1487dd550): Java 1.17's badlands
//! builders with 192 clay bands from an MT, 1.18's NormalNoise pillars, and BDS's own bounds.

use super::column::Column;
use super::spec::{Materials, Mesa};
use crate::block::Block;
use crate::noise::NormalNoise;
use crate::random::{Mt19937, PositionalFactory, Xoroshiro};

const SEA_LEVEL: i32 = 63;
const BANDS: usize = 192;
/// Band colours: the 16 dye colours, 16 for plain terracotta.
const PLAIN: u8 = 16;

pub struct MesaNoises {
    clay_bands_offset: NormalNoise,
    surface: NormalNoise,
    pillar: NormalNoise,
    pillar_roof: NormalNoise,
    bands: [u8; BANDS],
}

fn terracotta(colour: u8) -> Block {
    match colour {
        0 => Block::WhiteTerracotta,
        1 => Block::OrangeTerracotta,
        4 => Block::YellowTerracotta,
        8 => Block::LightGrayTerracotta,
        12 => Block::BrownTerracotta,
        14 => Block::RedTerracotta,
        _ => Block::Terracotta,
    }
}

/// Java's `generateBands` on BDS's MT and 192 bands; `nextBoolean` is bit 27 of a draw.
fn bands(seed: u32) -> [u8; BANDS] {
    let mut random = Mt19937::new(seed);
    let mut bands = [PLAIN; BANDS];
    let mut i = 0;
    loop {
        let at = random.next_int(5) + i + 1;
        if at >= BANDS as i32 {
            break;
        }
        bands[at as usize] = 1;
        i = at + 1;
        if at == BANDS as i32 - 1 {
            break;
        }
    }
    for (colour, extra) in [(4u8, 0), (12, 1), (14, 0)] {
        let count = random.next_int(10) + 6;
        for _ in 0..count {
            let length = random.next_int(3) + extra;
            let start = random.next_int(BANDS as i32);
            let end = start + length.min(BANDS as i32 - 1 - start);
            bands[start as usize..=end as usize].fill(colour);
        }
    }
    let whites = random.next_int(7) + 9;
    let mut at = 0usize;
    for _ in 0..whites {
        if at >= BANDS {
            break;
        }
        bands[at] = 0;
        if at >= 2 && random.next_u32() >> 27 & 1 != 0 {
            bands[at - 1] = 8;
        }
        if at < BANDS - 1 && random.next_u32() >> 27 & 1 != 0 {
            bands[at + 1] = 8;
        }
        at += random.next_int(16) as usize + 4;
    }
    bands
}

impl MesaNoises {
    /// BDS seeds these from the low 32 bits of the world seed, zero-extended.
    pub fn new(seed: i64) -> Self {
        let factory: PositionalFactory = Xoroshiro::from_seed(seed as u32 as i64).fork_positional();
        let noise = |name: &str, first: i32, amplitudes: &[f32]| NormalNoise::new(&factory, &format!("minecraft:{name}"), first, amplitudes);
        Self {
            clay_bands_offset: noise("clay_bands_offset", -8, &[1.0]),
            surface: noise("badlands_surface", -6, &[1.0; 3]),
            pillar: noise("badlands_pillar", -2, &[1.0; 4]),
            pillar_roof: noise("badlands_pillar_roof", -8, &[1.0]),
            bands: bands(seed as u32),
        }
    }

    /// The top of an eroded-badlands pillar, 0.0 for none.
    fn pillar_top(&self, x: f32, z: f32) -> f32 {
        let pillar = self.pillar.sample(x * 0.2, 0.0, z * 0.2) * 15.0;
        let height = (self.surface.sample(x, 0.0, z) * 8.25).abs().min(pillar);
        if !(height > 0.0) {
            return 0.0;
        }
        let roof = (self.pillar_roof.sample(x * 0.75, 0.0, z * 0.75) * 1.5).abs() * 50.0;
        (height * height * 2.5).min(roof.ceil() + 24.0) + 64.0
    }

    fn band(&self, offset: i32, y: i32) -> Block {
        let colour = self.bands[(y - crate::MIN_Y + offset + BANDS as i32).rem_euclid(BANDS as i32) as usize];
        terracotta(colour)
    }
}

pub fn build(column: &mut Column, materials: Materials, mesa: Mesa, noises: &MesaNoises, depth_noise: f32, random: &mut Mt19937) {
    let (x, z) = (column.world.0 as f32, column.world.1 as f32);
    let pillar_top = if mesa.bryce_pillars { noises.pillar_top(x, z).floor() as i32 } else { 0 };
    let noise = depth_noise / 3.0;
    let depth = (random.next_float() * 0.25 + noise + 3.0) as i32;
    let cos = (noise * std::f32::consts::PI).cos();
    let top = (column.lower..=TOP_Y).rev().find(|&y| column.get(y) != Block::Air);
    if top.is_some_and(|y| column.get(y).is_fluid()) {
        return;
    }
    let offset = (noises.clay_bands_offset.sample(x, 0.0, z) * 4.0).round() as i32;
    let band = |y: i32| -> Block {
        if cos <= 0.0 && y > SEA_LEVEL && y <= TOP_Y { noises.band(offset, y) } else if y > SEA_LEVEL && y <= TOP_Y { Block::Terracotta } else { Block::OrangeTerracotta }
    };
    let (mut under, mut run, mut sandy, mut stopped, mut counted) = (materials.mid, -1, false, false, 0);
    let mut y = TOP_Y;
    while y >= column.lower {
        let mut block = column.get(y);
        if block == Block::Bedrock {
            return;
        }
        let filled = y <= pillar_top && !stopped && block == Block::Air;
        if filled {
            column.set(y, Block::Stone);
            block = Block::Stone;
        } else {
            stopped |= block != Block::Air;
        }
        if counted < 15 {
            if block == Block::Air {
                run = -1;
            } else if block == Block::Stone {
                if run == -1 {
                    let candidate = if SEA_LEVEL + 1 < y { under } else { materials.mid };
                    if SEA_LEVEL - 4 <= y {
                        under = candidate;
                    }
                    if depth < 1 {
                        under = materials.foundation;
                    }
                    run = (y - SEA_LEVEL).max(0) + depth;
                    sandy = false;
                    if y < SEA_LEVEL - 1 {
                        column.set(y, if under == Block::WhiteTerracotta { Block::OrangeTerracotta } else { under });
                    } else if !mesa.has_forest || y <= SEA_LEVEL + 33 + depth * 2 {
                        if y <= SEA_LEVEL + 10 + depth {
                            column.set(y, materials.top);
                            sandy = true;
                        } else {
                            column.set(y, band(y));
                        }
                    } else {
                        column.set(y, if cos > 0.0 { Block::CoarseDirt } else { Block::GrassBlock });
                    }
                } else if run > 0 {
                    run -= 1;
                    column.set(y, if sandy { Block::OrangeTerracotta } else { noises.band(offset, y) });
                }
                counted += !filled as i32;
            }
        }
        y -= 1;
    }
}

const TOP_Y: i32 = 255;
