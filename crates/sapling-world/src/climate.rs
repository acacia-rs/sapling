//! Where snow falls rather than rain: the biome a loaded column holds, and worldgen's
//! `Biome::getTemperature`, which cools with height.

use crate::{ChunkPos, World};

/// At or under this temperature water freezes and snow falls (`Biome::shouldFreeze`, `shouldSnow`).
const COLD: f32 = 0.15;

impl World {
    /// The biome id at a cell of a loaded column.
    pub fn biome(&self, x: i32, y: i32, z: i32) -> Option<u32> {
        self.column(ChunkPos::of_block(x, z))?.chunk().biome(x, y, z)
    }

    /// Whether it is cold enough at a cell for water to freeze and snow to fall; never outside
    /// the overworld's sky or a loaded column.
    pub fn is_cold(&self, x: i32, y: i32, z: i32) -> bool {
        self.dim.sky && self.biome(x, y, z).and_then(|biome| sapling_worldgen::temperature(biome, [x, y, z])).is_some_and(|t| t <= COLD)
    }
}
