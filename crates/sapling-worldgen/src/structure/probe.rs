//! What start checks ask of the world before any chunk exists: biomes at single blocks
//! (`BiomeSource3d::containsOnly` 14805f710), the preliminary surface, and the heights of a
//! chunk's raw terrain (`WorldGenerator::computeChunkHeightMap` 14196d010).

use crate::block::Block;
use crate::density::Density;
use crate::terrain::MIN_Y;

pub struct Probe<'a> {
    pub density: &'a Density,
}

impl Probe<'_> {
    /// The preliminary surface at the chunk's corner quart; 0x7fff where no cell clears it.
    pub fn surface(&self, (x, z): (i32, i32)) -> i32 {
        self.density.preliminary_surface(x * 4, z * 4)
    }

    pub fn biome(&self, x: i32, y: i32, z: i32) -> u32 {
        self.density.climate.biome_at(x, y, z)
    }

    /// Every block on the 4-block lattice within `radius` of the centre (x outer, z, then y
    /// clamped to the world) has an allowed biome.
    pub fn contains_only(&self, (x, y, z): (i32, i32, i32), radius: i32, allowed: &[u32]) -> bool {
        let (y0, y1) = ((y - radius).max(MIN_Y), (y + radius).min(319));
        (x - radius..=x + radius)
            .step_by(4)
            .all(|bx| (z - radius..=z + radius).step_by(4).all(|bz| (y0..=y1).step_by(4).all(|by| allowed.contains(&self.biome(bx, by, bz)))))
    }

    /// The lowest of the chunk's column heights, each one above its highest non-air block of the
    /// density stage (water counts), 0 for an empty column.
    pub fn min_height(&self, (x, z): (i32, i32)) -> i32 {
        let shape = self.density.chunk_shape(x, z);
        let mut lowest = i32::MAX;
        for cz in 0..16 {
            for cx in 0..16 {
                let top = (MIN_Y..256).rev().find(|&y| shape.block(cx, y, cz) != Block::Air).map_or(0, |y| y + 1);
                lowest = lowest.min(top);
            }
        }
        lowest
    }
}
