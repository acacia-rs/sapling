//! The Nether's bedrock (`NetherGenerator::loadChunk` 1481ec000): every column's floor, 2 to 5 layers, then every
//! column's ceiling, 1 to 5 layers down from y 127, x outer and z inner, off an MT seeded by the chunk alone.

use super::terrain::HEIGHT;
use crate::block::Block;
use crate::chunk::ChunkShape;
use crate::random::Mt19937;

/// The chunk MT; the surface builders continue from where the bedrock left it.
pub fn chunk_random(chunk_x: i32, chunk_z: i32) -> Mt19937 {
    Mt19937::new((chunk_x as u32).wrapping_mul(0x9939_f508).wrapping_add((chunk_z as u32).wrapping_mul(0xf156_5bd5)))
}

pub fn place_bedrock(shape: &mut ChunkShape, random: &mut Mt19937) {
    for x in 0..16 {
        for z in 0..16 {
            for y in 0..random.next_int(4) + 2 {
                shape.set_block(x, y, z, Block::Bedrock);
            }
        }
    }
    for x in 0..16 {
        for z in 0..16 {
            for y in HEIGHT - 1 - random.next_int(5)..HEIGHT {
                shape.set_block(x, y, z, Block::Bedrock);
            }
        }
    }
}
