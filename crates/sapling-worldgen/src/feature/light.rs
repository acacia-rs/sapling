//! The light heightmap `recomputeHeightMap` (1409e29f0) keeps per column.

use crate::block::Block;
use crate::terrain::{ChunkShape, HEIGHT, MIN_Y};

/// What it counts: a block or liquid layer that absorbs light (`Block` +0xa5). Leaves and water do;
/// plants, bamboo, cacti, mangrove roots, snow layers and speleothems do not (the saved heightmaps of
/// the oracle worlds, examples/saved_heightmaps.rs).
pub fn blocks_light(block: Block) -> bool {
    if block == Block::Air {
        return false;
    }
    let clear = matches!(block, Block::Bamboo | Block::BambooSmallLeaves | Block::BambooLargeLeaves | Block::Cactus | Block::MangroveRoots) || block.is_snow_layer();
    let thin = block.is_plant() && !block.is_crop_block() || block.is_speleothem() || clear;
    !thin || block.is_waterlogged()
}

/// One above the column's highest block that `blocks_light`, `MIN_Y` if none.
pub fn column(shape: &ChunkShape, x: usize, z: usize) -> i16 {
    (MIN_Y..MIN_Y + HEIGHT as i32).rev().find(|&y| blocks_light(shape.block(x, y, z))).map_or(MIN_Y, |y| y + 1) as i16
}

/// Every column's, indexed `z * 16 + x`.
pub fn chunk(shape: &ChunkShape) -> [i16; 256] {
    std::array::from_fn(|i| column(shape, i % 16, i / 16))
}
