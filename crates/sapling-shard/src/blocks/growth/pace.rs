//! A cell's chance of a random tick a game tick, BDS's (`LevelChunk::tickImpl` 1409fbe2c): a ticking chunk
//! filled up to its `n`th section picks `floor(2.5 n) x randomTickSpeed` cells a tick, each anywhere in those
//! sections (`n x 4096` cells), so a lone floor of blocks ticks a fifth slower than a chunk with anything
//! above it (docs/research/growth.md "The random tick's pace").

use std::collections::HashMap;

use bevy_ecs::entity::Entity;
use sapling_api::BlockPos;
use sapling_world::ChunkPos;

const CELLS_A_SECTION: f32 = 4096.0;

/// The chance for one cell of a chunk filled up to `sections` at `rule`.
pub(super) fn chance(sections: usize, rule: i32) -> f32 {
    if sections == 0 {
        return 0.0;
    }
    (sections as f32 * 2.5).floor() * rule.max(0) as f32 / (sections as f32 * CELLS_A_SECTION)
}

/// The most [`chance`] gives at `rule`: a chunk of two sections, five picks in 8192 cells.
pub(super) fn most(rule: i32) -> f32 {
    chance(2, rule)
}

/// Each world's columns' filled sections, read once a tick.
#[derive(Default)]
pub(super) struct Heights(HashMap<(Entity, ChunkPos), usize>);

impl Heights {
    /// [`chance`] for the cell at `pos` of `level`, the world `layer`, at `rule`.
    pub(super) fn chance(&mut self, layer: Entity, level: &sapling_world::World, [x, _, z]: BlockPos, rule: i32) -> f32 {
        let column = ChunkPos::of_block(x, z);
        chance(*self.0.entry((layer, column)).or_insert_with(|| level.filled_sections(column)), rule)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chunk_picks_two_and_a_half_cells_a_section_rounded_down() {
        assert_eq!(chance(1, 1), 2.0 / 4096.0);
        assert_eq!(chance(2, 1), 5.0 / 8192.0);
        assert_eq!(chance(3, 20), 7.0 * 20.0 / 12288.0);
        assert_eq!(chance(24, 1), 60.0 / 98304.0);
        assert_eq!(chance(0, 200), 0.0);
        assert!((1..=24).all(|sections| chance(sections, 7) <= most(7)));
    }
}
