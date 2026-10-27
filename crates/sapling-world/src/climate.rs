//! Where snow falls rather than rain: the biome a loaded column holds, and worldgen's
//! `Biome::getTemperature`, which cools with height.

use crate::{ChunkPos, World};

/// At or under this temperature water freezes and snow falls (`Biome::shouldFreeze`, `shouldSnow`).
const COLD: f32 = 0.15;

impl World {
    /// The biome id stored at a cell (the dimension's own where none was); `None` where nothing
    /// is loaded.
    pub fn biome(&self, x: i32, y: i32, z: i32) -> Option<u32> {
        let column = self.column(ChunkPos::of_block(x, z))?;
        Some(column.chunk().biome(x, y, z).unwrap_or_else(|| crate::column::default_biome(self.dim)))
    }

    /// Whether it is cold enough at a cell for water to freeze and snow to fall; never outside
    /// the overworld's sky or a loaded column.
    pub fn is_cold(&self, x: i32, y: i32, z: i32) -> bool {
        self.dim.sky && self.temperature(x, y, z).is_some_and(|t| t <= COLD)
    }

    /// The temperature at a cell, cooled with height; `None` where nothing is loaded or the biome
    /// has no climate here.
    pub fn temperature(&self, x: i32, y: i32, z: i32) -> Option<f32> {
        sapling_worldgen::temperature(self.biome(x, y, z)?, [x, y, z])
    }
}
