//! The liquid ticks a generated chunk's own queue still holds when the world has generated it, for the
//! server to run (docs/research/worldgen-bds-liquids.md, "How they run" and "Game time"): the instaticks
//! its decoration left, and what later sources routed to it. BDS's `LevelChunk::tick` (1409f98a0) runs that
//! queue up to the level's current tick, so every one is due on the chunk's first tick, in queue order.

use super::super::ticks::{Tick, TickQueue};
use super::super::view::Routed;
use super::super::{Features, Inputs, Rank};
use crate::Dimension;

/// The leftover instaticks that can come due: a deferred one (`u64::MAX`) never does in game.
pub(super) fn due(left: &[Tick]) -> Vec<Routed> {
    left.iter().filter(|tick| tick.time != u64::MAX).map(|tick| (tick.at, tick.lava, tick.time)).collect()
}

impl Features {
    /// `chunk`'s queue once every source around it has decorated, in the order it runs: (cell, lava).
    /// Empty under a `SAPLING_LIQUID_GAME_TICKS` replay, which ran them already.
    pub fn game_ticks(&self, chunk: (i32, i32), inputs: Inputs) -> Vec<([i32; 3], bool)> {
        let Some(own) = self.rank(chunk).filter(|_| super::game_ticks() == 0 && self.dimension != Dimension::End) else { return Vec::new() };
        // The overworld's decoration takes in what sources before it routed here (`acquireAllTicks`); the Nether's never does.
        let waits = |source: (i32, i32)| self.dimension == Dimension::Nether || self.rank(source).is_some_and(|rank| rank >= own);
        let mut queue = TickQueue::chunk();
        for source in self.sources_before(chunk, Rank::MAX) {
            let writes = self.source_writes(source, inputs);
            if source == chunk {
                writes.3.iter().for_each(|&(at, lava, time)| queue.add_at(at, lava, time));
            } else if waits(source) {
                writes.routed_to(chunk).for_each(|(at, lava, delay)| queue.add(at, lava, delay));
            }
        }
        queue.drain().into_iter().map(|tick| (tick.at, tick.lava)).collect()
    }
}

#[cfg(test)]
mod tests {
    /// Seed 20261003's chunk (5, 4): a flow from a later neighbour stopped at its edge (21x21 chunks round
    /// the origin hold 17 such chunks; most chunks hold none).
    #[test]
    fn a_chunk_keeps_the_ticks_later_sources_routed_to_it() {
        let generator = crate::OverworldGenerator::with_areas(20261003, 1);
        let ticks = generator.chunk_game_ticks(5, 4);
        assert_eq!((ticks.len(), &ticks[..3]), (7, &[([88, -13, 79], false), ([88, -13, 78], false), ([88, -13, 77], false)][..]));
        assert!(ticks.iter().all(|&([x, _, z], _)| (x >> 4, z >> 4) == (5, 4)));
        assert!(generator.chunk_game_ticks(0, 0).is_empty());
    }
}
