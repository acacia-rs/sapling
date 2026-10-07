//! `BiomeSurfaceSystem::buildChunkSurface` (14580d280): per column, x outer and z inner, the
//! surface builder of the biome at the column top runs from the top down to the preliminary
//! surface minus 8, drawing from the chunk MT the bedrock floor left off.

mod biomes;
mod column;
mod noise;
mod overworld;
mod spec;

pub use noise::SurfaceNoises;

use crate::biome::ChunkBiomes;
use crate::block::Block;
use crate::density::Density;
use crate::random::Mt19937;
use crate::terrain::{ChunkShape, MIN_Y};
use column::Column;
use spec::{Builder, Materials, Steep, SurfaceSpec};

/// The y of the column position the adjustment noise samples (the chunk's minimum y).
const ADJUSTMENT_Y: i32 = MIN_Y;
/// The block buffer ends at y 255.
const TOP_Y: i32 = 255;

/// The highest non-air block of every column, `MIN_Y - 1` for an empty one.
fn column_tops(shape: &ChunkShape) -> [[i32; 16]; 16] {
    std::array::from_fn(|x| std::array::from_fn(|z| (MIN_Y..=TOP_Y).rev().find(|&y| shape.block(x, y, z) != Block::Air).unwrap_or(MIN_Y - 1)))
}

/// Whether the column faces one of the steep directions: a neighbour (clamped to the chunk) at
/// least 4 blocks higher on the opposite side (145809ef0).
fn is_steep(steep: &Steep, tops: &[[i32; 16]; 16], x: usize, z: usize) -> bool {
    let (north, south) = (tops[x][z.saturating_sub(1)], tops[x][(z + 1).min(15)]);
    if (steep.south && north - south > 3) || (steep.north && north - south < -3) {
        return true;
    }
    let (west, east) = (tops[x.saturating_sub(1)][z], tops[(x + 1).min(15)][z]);
    (steep.east && west - east > 3) || (steep.west && west - east < -3)
}

impl SurfaceNoises {
    pub fn build_chunk_surface(&self, shape: &mut ChunkShape, biomes: &ChunkBiomes, density: &Density, chunk: (i32, i32), random: &mut Mt19937) {
        let tops = column_tops(shape);
        for x in 0..16 {
            for z in 0..16 {
                let (wx, wz) = (chunk.0 * 16 + x as i32, chunk.1 * 16 + z as i32);
                let top = tops[x][z];
                let biome = biomes.biome(x, (top + 1).min(MIN_Y + crate::HEIGHT as i32 - 1), z);
                let Some(spec) = biomes::surface_spec(biome) else { continue };
                let lower = (density.preliminary_surface(wx >> 2, wz >> 2) - 8).clamp(MIN_Y, TOP_Y);
                let mut column = Column { shape: &mut *shape, x, z, world: (wx, wz), start: (top + 1).min(TOP_Y), lower };
                self.build_column(&mut column, &spec, &tops, random);
            }
        }
    }

    /// The biome's materials with every adjustment whose noise is in range applied, in order.
    fn adjusted_materials(&self, spec: &SurfaceSpec, (x, z): (i32, i32)) -> Materials {
        let mut materials = spec.materials;
        for adjustment in spec.adjustments {
            let value = self.adjustment([x, ADJUSTMENT_Y, z], adjustment.noise_scale);
            if adjustment.range[0] <= value && value < adjustment.range[1] {
                materials.top = adjustment.top_material.unwrap_or(materials.top);
                materials.mid = adjustment.mid_material.unwrap_or(materials.mid);
                materials.sea_floor = adjustment.sea_floor_material.unwrap_or(materials.sea_floor);
                materials.foundation = adjustment.foundation_material.unwrap_or(materials.foundation);
            }
        }
        materials
    }

    fn build_column(&self, column: &mut Column, spec: &SurfaceSpec, tops: &[[i32; 16]; 16], random: &mut Mt19937) {
        let materials = self.adjusted_materials(spec, column.world);
        let depth_noise = self.depth(column.world.0, column.world.1);
        let depth = overworld::surface_depth(random, depth_noise);
        let steep = spec.steep.filter(|s| is_steep(s, tops, column.x, column.z)).map(|s| s.material);
        match spec.builder {
            Builder::Overworld | Builder::FrozenOcean | Builder::Swamp { .. } | Builder::Mesa(_) => {
                overworld::build(column, materials, steep, spec.sea_floor_depth, spec.temperature, depth, random)
            }
        }
    }
}
