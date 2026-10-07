//! The bedrock floor (`OverworldGenerator::loadChunk` 148211300): 2 to 5 layers per column from
//! an MT seeded by the chunk position alone, x outer and z inner as the block buffer is laid out.

use crate::block::Block;
use crate::random::Mt19937;
use crate::terrain::{ChunkShape, MIN_Y};

pub fn place_bedrock_floor(shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32) {
    let seed = (chunk_x as u32).wrapping_mul(0x1460_9048).wrapping_add((chunk_z as u32).wrapping_mul(0x07EB_E2D5));
    let mut random = Mt19937::new(seed);
    for x in 0..16 {
        for z in 0..16 {
            let layers = random.next_int(4) + 2;
            for y in MIN_Y..MIN_Y + layers {
                shape.set_block(x, y, z, Block::Bedrock);
            }
        }
    }
}
