//! A generated column's leftover liquid ticks (`OverworldGenerator::chunk_game_ticks`) as the `PendingTicks`
//! record a stored column carries, so the shard queues them when the column arrives (sapling-shard
//! blocks/pending.rs). All are due on its first tick, in order, as in BDS (the generator's leftovers.rs).

use acacia_world::BlockRegistry;
use sapling_worldgen::{Block, ChunkShape};

use super::convert::BlockIds;
use crate::ChunkPos;

/// The tick's block: a water tick on a magma cell is the magma's own (bubbles above it).
pub(crate) fn pending_ticks(pos: ChunkPos, shape: &ChunkShape, ticks: &[([i32; 3], bool)], ids: &BlockIds, registry: &BlockRegistry) -> Vec<u8> {
    if ticks.is_empty() {
        return Vec::new();
    }
    let magma = |[x, y, z]: [i32; 3]| ChunkPos::of_block(x, z) == pos && shape.block((x & 15) as usize, y, (z & 15) as usize) == Block::Magma;
    let block = |at, lava| match lava {
        true => Block::Lava,
        false if magma(at) => Block::Magma,
        false => Block::Water,
    };
    let ticks: Vec<([i32; 3], u32, i64)> = ticks.iter().map(|&(at, lava)| (at, ids.id(block(at, lava)), 1)).collect();
    crate::encode_pending_ticks(registry, 0, &ticks)
}
