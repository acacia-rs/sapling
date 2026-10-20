//! The Nether's surface stage (`NetherGenerator::loadChunk` 1481ec000): per column, x outer and z inner, the builder
//! of the column's biome on the chunk MT the bedrock left. The default builder walks y 127 down to 1 on a fixed sea
//! level of 32 with its adjustments chosen per column and applied per y (the whole column's at y 127 first).

use super::capped::{self, CappedNoises};
use super::surface_spec::{Builder, HeightAdjustment, builder};
use super::terrain::{HEIGHT, NetherNoises, SEA_LEVEL};
use crate::biome::ChunkBiomes;
use crate::chunk::ChunkShape;
use crate::random::Mt19937;
use crate::surface::column::Column;
use crate::surface::overworld::{self, Sea};
use crate::surface::spec::{Adjustment, Materials};
use crate::surface::{adjustment_noise, depth_noise};

/// The depth noise's frequency (0.0625 where the overworld's is 0.125).
const DEPTH_FREQUENCY: f32 = 0.0625;

pub struct NetherSurface {
    soulsand_valley: CappedNoises,
    basalt_deltas: CappedNoises,
}

fn apply(materials: Materials, adjustment: &Adjustment) -> Materials {
    Materials {
        top: adjustment.top_material.unwrap_or(materials.top),
        mid: adjustment.mid_material.unwrap_or(materials.mid),
        sea_floor: adjustment.sea_floor_material.unwrap_or(materials.sea_floor),
        foundation: adjustment.foundation_material.unwrap_or(materials.foundation),
        sea: materials.sea,
    }
}

/// `materials` with every kept adjustment whose heights hold `y` applied, in order.
fn adjusted(materials: Materials, kept: &[&HeightAdjustment], y: i32) -> Materials {
    kept.iter().filter(|a| a.heights[0] <= y && y <= a.heights[1]).fold(materials, |m, a| apply(m, &a.adjustment))
}

impl NetherSurface {
    /// The capped builders' noises take the seed's low 32 bits.
    pub fn new(seed: i64) -> Self {
        let capped = |biome: u8| match builder(u32::from(biome)) {
            Some(Builder::Capped(capped)) => CappedNoises::new(seed as u32, &capped),
            _ => unreachable!("a capped biome"),
        };
        Self { soulsand_valley: capped(super::SOULSAND_VALLEY), basalt_deltas: capped(super::BASALT_DELTAS) }
    }

    pub fn build(&self, shape: &mut ChunkShape, biomes: &ChunkBiomes, noises: &NetherNoises, chunk: (i32, i32), random: &mut Mt19937) {
        for x in 0..16 {
            for z in 0..16 {
                let world = (chunk.0 * 16 + x as i32, chunk.1 * 16 + z as i32);
                let biome = biomes.biome(x, 0, z);
                let depth = depth_noise(&noises.depth, world.0, world.1, DEPTH_FREQUENCY);
                match builder(biome) {
                    Some(Builder::Default { sea_floor_depth, materials, adjustments }) => {
                        let kept: Vec<&HeightAdjustment> = adjustments
                            .iter()
                            .filter(|a| {
                                let value = adjustment_noise(&noises.adjustment, [world.0, 0, world.1], a.adjustment.noise_scale);
                                a.adjustment.range[0] <= value && value < a.adjustment.range[1]
                            })
                            .collect();
                        let base = adjusted(materials, &kept, HEIGHT - 1);
                        let mut column = Column { shape: &mut *shape, x, z, world, start: HEIGHT - 1, lower: 0 };
                        let depth = overworld::surface_depth(random, depth);
                        overworld::build(&mut column, |y| adjusted(base, &kept, y), None, sea_floor_depth, 2.0, depth, Sea::Fixed(SEA_LEVEL), random);
                    }
                    Some(Builder::Capped(spec)) => {
                        let noises = if biome as u8 == super::SOULSAND_VALLEY { &self.soulsand_valley } else { &self.basalt_deltas };
                        capped::build(shape, (x, z), world, &spec, noises, depth, SEA_LEVEL, random);
                    }
                    None => {}
                }
            }
        }
    }
}
