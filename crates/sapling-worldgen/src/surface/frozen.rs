//! The `minecraft:frozen_ocean` surface builder (build 1487e4350, init 1487e18e0): Java 1.17's
//! iceberg pass on 1.18's NormalNoises and a positional Xoroshiro, then the overworld builder.

use super::column::Column;
use crate::block::Block;
use crate::noise::NormalNoise;
use crate::random::{PositionalFactory, Xoroshiro};

pub const SEA_LEVEL: i32 = 63;

pub struct IcebergNoises {
    pillar: NormalNoise,
    surface: NormalNoise,
    pillar_roof: NormalNoise,
    positional: PositionalFactory,
}

impl IcebergNoises {
    /// BDS seeds these from the low 32 bits of the world seed, sign-extended.
    pub fn new(seed: i64) -> Self {
        let positional = Xoroshiro::from_seed(seed as i32 as i64).fork_positional();
        let noise = |name: &str, first: i32, amplitudes: &[f32]| NormalNoise::new(&positional, &format!("minecraft:{name}"), first, amplitudes);
        Self {
            pillar: noise("iceberg_pillar", -6, &[1.0; 4]),
            surface: noise("iceberg_surface", -6, &[1.0; 3]),
            pillar_roof: noise("iceberg_pillar_roof", -3, &[1.0]),
            positional,
        }
    }
}

/// Places the iceberg of a column. Only where the iceberg noise clears 1.8 does BDS go on to the
/// overworld builder; elsewhere the column keeps bare stone (false).
pub fn build_iceberg(column: &mut Column, noises: &IcebergNoises, temperature: f32) -> bool {
    let (x, z) = column.world;
    let (fx, fz) = (x as f32, z as f32);
    let pillar = noises.pillar.sample(fx * 1.28, 0.0, fz * 1.28) * 15.0;
    let height = (noises.surface.sample(fx, 0.0, fz) * 8.25).abs().min(pillar);
    if !(height > 1.8) {
        return false;
    }
    let roof = (noises.pillar_roof.sample(fx * 1.17, 0.0, fz * 1.17) * 1.5).abs() * 40.0;
    let mut top = (height * height * 1.2).min(roof.ceil() + 14.0);
    if temperature > 0.1 {
        top += -2.0;
    }
    let floor = if 2.0 < top { (SEA_LEVEL as f32 - top) + -7.0 } else { 0.0 };
    let top = if 2.0 < top { SEA_LEVEL as f32 + top } else { 0.0 };
    let mut random = noises.positional.at_wide(x, 0, z);
    let snow_limit = random.next_int_bounded(4) + 2;
    let snow_floor = SEA_LEVEL + 18 + random.next_int_bounded(10);
    let start = (column.start as f32).max(top + 1.0) as i32;
    let mut snow = 0;
    let mut y = start;
    // Both bounds truncate (1487e470c, 1487e4713): a fractional top draws for one cell fewer.
    let (top_y, floor_y) = (top as i32, floor as i32);
    while y >= column.lower {
        let block = column.get(y);
        let place = match block {
            Block::Air => y < top_y && random.next_float() > 0.01,
            Block::Water => floor_y < y && y < SEA_LEVEL && floor != 0.0 && random.next_float() > 0.15,
            _ => false,
        };
        if place {
            if snow > snow_limit || y <= snow_floor {
                column.set(y, Block::PackedIce);
            } else {
                column.set(y, Block::Snow);
                snow += 1;
            }
        }
        y -= 1;
    }
    true
}
