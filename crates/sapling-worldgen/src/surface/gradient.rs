//! The subsurface pass of `loadChunk` (after `buildChunkSurface`, 148211300): each column from the
//! bottom to its preliminary surface minus 8, split into runs of one biome; the runs of biomes with
//! a `minecraft:subsurface_builder` go to it. Only `minecraft:noise_gradient` exists
//! (`NoiseGradientSurfaceBuilder` 1487e14c0, seeded in 1487e0c20).

use super::biomes::subsurface_spec;
use super::spec::Gradient;
use crate::biome::ChunkBiomes;
use crate::block::Block;
use crate::density::Density;
use crate::noise::NormalNoise;
use crate::random::PositionalFactory;
use crate::terrain::{ChunkShape, HEIGHT, MIN_Y};

/// The gradient builders' noises by name, from the world's positional factory.
pub struct GradientNoises(Vec<(&'static str, NormalNoise)>);

impl GradientNoises {
    pub fn new(factory: &PositionalFactory) -> Self {
        let mut noises: Vec<(&'static str, NormalNoise)> = Vec::new();
        for spec in (0..256).filter_map(subsurface_spec) {
            if !noises.iter().any(|(name, _)| *name == spec.noise) {
                noises.push((spec.noise, NormalNoise::new(factory, spec.noise, spec.first_octave, spec.amplitudes)));
            }
        }
        Self(noises)
    }

    fn get(&self, name: &str) -> &NormalNoise {
        &self.0.iter().find(|(n, _)| *n == name).expect("built for every spec").1
    }

    pub fn build_subsurface(&self, shape: &mut ChunkShape, biomes: &ChunkBiomes, density: &Density, chunk: (i32, i32)) {
        for x in 0..16 {
            for z in 0..16 {
                let (wx, wz) = (chunk.0 * 16 + x as i32, chunk.1 * 16 + z as i32);
                let top = (density.preliminary_surface(wx >> 2, wz >> 2) - 8).clamp(MIN_Y, MIN_Y + HEIGHT as i32 - 1);
                let mut start = MIN_Y;
                let mut current = biomes.biome(x, MIN_Y, z);
                for y in MIN_Y..=top {
                    let biome = biomes.biome(x, y, z);
                    let end = if biome == current { y } else { y - 1 };
                    if y == top || biome != current {
                        if let Some(spec) = subsurface_spec(current) {
                            self.build_run(shape, &spec, ((x, z), (wx, wz)), start, end);
                        }
                        current = biome;
                        start = y;
                    }
                }
            }
        }
    }

    /// BDS starts one below the run's top (`end - 1`), down to its start.
    fn build_run(&self, shape: &mut ChunkShape, spec: &Gradient, ((x, z), (wx, wz)): ((usize, usize), (i32, i32)), start: i32, end: i32) {
        let noise = self.get(spec.noise);
        for y in (start..end).rev() {
            let block = shape.block(x, y, z);
            if block == Block::Air || block.is_fluid() || spec.keep.contains(&block) {
                continue;
            }
            let value = noise.sample(wx as f32, y as f32, wz as f32).clamp(-1.0, 1.0);
            if let Some(&(_, _, band)) = spec.bands.iter().find(|&&(min, max, _)| min < value && value <= max) {
                shape.set_block(x, y, z, band);
            }
        }
    }
}
