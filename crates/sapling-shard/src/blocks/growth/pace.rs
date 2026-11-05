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

/// How many random ticks a cell with `chance` a tick takes this tick, `roll` in [0, 1): BDS's picks are drawn
/// with replacement, so past a rule of about 1600 a cell comes up more than once a tick (Poisson, by its
/// inverse from the top, so that a roll under `1 - e^-chance` ticks it at all).
pub(super) fn count(chance: f32, roll: f32) -> u32 {
    let (chance, roll) = (chance.max(0.0) as f64, roll as f64);
    let (mut one, mut tail, mut ticks) = ((-chance).exp(), -(-chance).exp_m1(), 0);
    while roll < tail && ticks < 64 {
        ticks += 1;
        one *= chance / ticks as f64;
        tail -= one;
    }
    ticks
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

    #[test]
    fn a_cell_takes_its_chance_in_ticks_on_average_more_than_one_a_tick_past_one() {
        let mean = |chance: f32| (0..100_000).map(|i| count(chance, (i as f32 + 0.5) / 100_000.0)).sum::<u32>() as f32 / 100_000.0;
        for chance in [0.001, 0.3, 2.333, 2.5] {
            assert!((mean(chance) - chance).abs() < chance * 0.01, "{chance}: {}", mean(chance));
        }
        assert_eq!((count(2.5, 0.99), count(2.5, 0.9), count(0.0, 0.0)), (0, 1, 0));
    }
}
