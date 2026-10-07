//! The bedrock floor (`OverworldGenerator::loadChunk` 148211300): 2 to 5 layers per column from
//! an MT seeded by the chunk position alone, x outer and z inner as the block buffer is laid out.

use crate::block::Block;
use crate::random::Mt19937;
use crate::terrain::{ChunkShape, MIN_Y};

/// The chunk MT: the floor draws first, the surface builders continue from it.
pub fn chunk_random(chunk_x: i32, chunk_z: i32) -> Mt19937 {
    Mt19937::new((chunk_x as u32).wrapping_mul(0x1460_9048).wrapping_add((chunk_z as u32).wrapping_mul(0x07EB_E2D5)))
}

pub fn place_bedrock_floor(shape: &mut ChunkShape, random: &mut Mt19937) {
    for x in 0..16 {
        for z in 0..16 {
            let layers = random.next_int(4) + 2;
            for y in MIN_Y..MIN_Y + layers {
                shape.set_block(x, y, z, Block::Bedrock);
            }
        }
    }
}
