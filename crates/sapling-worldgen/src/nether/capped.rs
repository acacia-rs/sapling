//! `minecraft:capped` (`VanillaSurfaceBuilders::CappedSurfaceBuilder::buildSurfaceAt` 145809370, noises built by
//! 1458090b0 and 1458091f0): not Java's capped builder. Walking a column up, every solid-to-open step lays a floor
//! downward and every open-to-solid step a ceiling upward, then stone becomes the foundation and water the sea.

use super::surface_spec::Capped;
use crate::block::Block;
use crate::chunk::ChunkShape;
use crate::noise::LegacyPerlin;
use crate::random::Mt19937;
use crate::surface::overworld::surface_depth;

/// A floor's cells within this of the sea level, at and below it, take the beach material.
const BEACH_DEPTH: i32 = 5;

/// One single-octave noise per floor material, then per ceiling material, seeded `seed + i` in that order; the beach
/// noise is the first floor material's.
pub struct CappedNoises {
    floor: Vec<LegacyPerlin>,
    ceiling: Vec<LegacyPerlin>,
}

impl CappedNoises {
    pub fn new(seed: u32, capped: &Capped) -> Self {
        let noise = |i: usize| LegacyPerlin::new(&mut Mt19937::new(seed.wrapping_add(i as u32)), 1);
        let floor = (0..capped.floor.len()).map(noise).collect();
        let ceiling = (capped.floor.len()..capped.floor.len() + capped.ceiling.len()).map(noise).collect();
        Self { floor, ceiling }
    }
}

/// The material whose noise is highest at the column (the first on ties, and without sampling when alone).
fn pick(materials: &[Block], noises: &[LegacyPerlin], at: [f32; 3]) -> Block {
    if materials.len() == 1 {
        return materials[0];
    }
    let mut best = (materials[0], -1.0f32);
    for (&material, noise) in materials.iter().zip(noises) {
        let value = noise.value(at) * noise.normalisation;
        if best.1 < value {
            best = (material, value);
        }
    }
    best.0
}

/// Water and lava are open, everything else (bedrock too) solid (145809050).
fn solid(block: Block) -> bool {
    block != Block::Air && !block.is_fluid()
}

#[allow(clippy::too_many_arguments)]
pub fn build(shape: &mut ChunkShape, (x, z): (usize, usize), world: (i32, i32), capped: &Capped, noises: &CappedNoises, depth_noise: f32, sea: i32, random: &mut Mt19937) {
    let floor_depth = surface_depth(random, depth_noise);
    let ceiling_depth = surface_depth(random, depth_noise);
    let at = [world.0 as f32 * 0.0625, sea as f32, world.1 as f32 * 0.0625];
    let floor = pick(capped.floor, &noises.floor, at);
    let ceiling = pick(capped.ceiling, &noises.ceiling, at);
    let beach_noise = &noises.floor[0];
    let beach = capped.beach.filter(|_| beach_noise.value([world.0 as f32 * 0.03125, 109.0, world.1 as f32 * 0.03125]) * beach_noise.normalisation > 0.0);
    let height = super::terrain::HEIGHT;
    let mut below = solid(shape.block(x, 0, z));
    for y in 0..height {
        let here = solid(shape.block(x, y, z));
        if here != below {
            if !here {
                fill(shape, (x, z), y - 1, (y - floor_depth - 1).clamp(-1, height), |c| match beach {
                    Some(beach) if sea - BEACH_DEPTH <= c && c <= sea => beach,
                    _ => floor,
                });
            } else {
                fill(shape, (x, z), y, (y + ceiling_depth).clamp(-1, height), |_| ceiling);
            }
            below = here;
        }
        match shape.block(x, y, z) {
            Block::Stone => shape.set_block(x, y, z, capped.foundation),
            Block::Water => shape.set_block(x, y, z, capped.sea),
            _ => {}
        }
    }
}

/// From `from` toward `until` (exclusive), while the cells stay solid.
fn fill(shape: &mut ChunkShape, (x, z): (usize, usize), from: i32, until: i32, material: impl Fn(i32) -> Block) {
    let step = if from <= until { 1 } else { -1 };
    let mut c = from;
    while c != until && solid(shape.block(x, c, z)) {
        shape.set_block(x, c, z, material(c));
        c += step;
    }
}
