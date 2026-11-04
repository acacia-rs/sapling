//! A generated column's waiting block ticks as the `PendingTicks` record a stored column carries, so
//! the shard queues them when the column arrives (sapling-shard blocks/pending.rs): decoration's leftover
//! liquid ticks (`OverworldGenerator::chunk_game_ticks`, in order), then one per bubble column cell. All are
//! due on its first tick, as in BDS (the generator's leftovers.rs).

use acacia_world::BlockRegistry;
use sapling_worldgen::{Block, ChunkShape, HEIGHT, MIN_Y};

use super::convert::BlockIds;
use crate::ChunkPos;

/// The tick's block: a water tick on a magma cell is the magma's own (bubbles above it). A bubble column
/// decoration set keeps its own tick until the game runs it: BDS's tick-free worlds save one per cell
/// (seed 20261003, 321 near the origin), so the column climbs once the chunk ticks.
pub(crate) fn pending_ticks(pos: ChunkPos, shape: &ChunkShape, ticks: &[([i32; 3], bool)], ids: &BlockIds, registry: &BlockRegistry) -> Vec<u8> {
    let magma = |[x, y, z]: [i32; 3]| ChunkPos::of_block(x, z) == pos && shape.block((x & 15) as usize, y, (z & 15) as usize) == Block::Magma;
    let block = |at, lava| match lava {
        true => Block::Lava,
        false if magma(at) => Block::Magma,
        false => Block::Water,
    };
    let mut waiting: Vec<([i32; 3], u32, i64)> = ticks.iter().map(|&(at, lava)| (at, ids.id(block(at, lava)), 1)).collect();
    let column = ids.id(Block::BubbleColumn);
    for x in 0..16 {
        for z in 0..16 {
            for y in (MIN_Y..MIN_Y + HEIGHT as i32).filter(|&y| shape.block(x, y, z) == Block::BubbleColumn) {
                waiting.push(([pos.x * 16 + x as i32, y, pos.z * 16 + z as i32], column, 1));
            }
        }
    }
    if waiting.is_empty() {
        return Vec::new();
    }
    crate::encode_pending_ticks(registry, 0, &waiting)
}
