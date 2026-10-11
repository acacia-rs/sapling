//! Liquid flow at the end of `decorationPostProcessChunk` (1482143a0): the bottom of every water
//! run in the chunk's 60 border columns is reset to a source and ticked, then the ticks queued
//! during decoration (springs, those borders, what they spread to) run as instaticks, one
//! `tickPendingTicks` call per generation, at most 128, over the chunk's 3x3 neighbourhood.
//! The flow rule is docs/research/water.md's (`LiquidBlock::_tick` 148b5d060).

mod detect;
mod detect_table;
mod flow;

use super::ticks::TickQueue;
use super::view::{Routed, View};
use crate::block::Block;

const GENERATIONS: usize = 128;

/// The `View::on_write` hook for decoration: features write with flags 3, so liquids beside a
/// write get a tick. `SAPLING_LIQUID_FEATURE_NOTIFY=0` turns it off.
pub fn write_hook() -> Option<fn(&mut View, [i32; 3])> {
    let on = std::env::var("SAPLING_LIQUID_FEATURE_NOTIFY").map_or(true, |v| v != "0");
    on.then_some(flow::update_neighbors as fn(&mut View, [i32; 3]))
}

/// `inherited`: the ticks earlier sources left on this chunk's queue (docs/research/worldgen-bds-liquids.md, "How they run").
pub fn settle(view: &mut View, inherited: &[Routed]) {
    reset_border_water(view);
    let mut generated = TickQueue::default();
    for (at, lava) in view.own_fluid_ticks() {
        generated.add(at, lava, 1);
    }
    for &(at, lava, delay) in inherited {
        generated.add(at, lava, delay);
    }
    view.ticks.acquire(generated);
    for _ in 0..GENERATIONS {
        if view.ticks.is_empty() {
            break;
        }
        let ticks = view.ticks.take_generation();
        view.ticks.instaticking = true;
        for tick in ticks {
            flow::run(view, tick);
        }
        view.ticks.instaticking = false;
    }
    view.routed = view.ticks.in_game.clone();
    play(view);
}

/// The chunk's queue in game after decoration: the ticks that could not join the block source's
/// queue, then the instaticks left over, run by game tick for `game_ticks()` ticks over the same
/// 3x3 (BDS then has every neighbour; this is the approximation).
fn play(view: &mut View) {
    let budget = game_ticks();
    let mut game = TickQueue::game();
    for (at, lava, delay) in std::mem::take(&mut view.ticks.in_game) {
        game.add(at, lava, delay + 1);
    }
    // Leftover instaticks keep their wrapped times (delay - 1); deferred ones (u64::MAX) never come due.
    for tick in view.ticks.drain() {
        game.add_at(tick.at, tick.lava, tick.time);
    }
    if budget == 0 || game.is_empty() {
        return;
    }
    let decoration = std::mem::replace(&mut view.ticks, game);
    for now in 1..=budget {
        if view.ticks.is_empty() {
            break;
        }
        for tick in view.ticks.take_due(now) {
            flow::run(view, tick);
        }
    }
    view.ticks = decoration;
}

/// Game ticks the oracle's chunks ticked after decoration; `SAPLING_LIQUID_GAME_TICKS` overrides.
pub(super) fn game_ticks() -> u64 {
    static TICKS: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *TICKS.get_or_init(|| std::env::var("SAPLING_LIQUID_GAME_TICKS").ok().and_then(|v| v.parse().ok()).unwrap_or(0))
}

/// The chunk's border columns up to the light heightmap, bottom up: the lowest cell of each run
/// of water (layer 0, any depth) becomes a source, set without updates, and gets a tick in 1.
fn reset_border_water(view: &mut View) {
    let (x0, z0) = (view.chunk().0 * 16, view.chunk().1 * 16);
    let border = (0..16).flat_map(|i| [(0, i), (i, 0), (i, 15), (15, i)]);
    let mut columns: Vec<(i32, i32)> = border.collect();
    // BDS's table (14a923c90): x = 0 column, then x = 1..14 with z = 0 and 15, then x = 15 column.
    columns.sort_unstable();
    columns.dedup();
    for (dx, dz) in columns {
        let (x, z) = (x0 + dx, z0 + dz);
        let top = view.heightmap(x, z);
        let mut below_dry = true;
        for y in crate::terrain::MIN_Y..top {
            let water = view.block(x, y, z).is_fluid() && !view.block(x, y, z).is_lava();
            if water && below_dry {
                if view.block(x, y, z) != Block::Water {
                    view.set_block_quiet(x, y, z, Block::Water);
                }
                view.schedule([x, y, z], false, 1);
            }
            below_dry = !water;
        }
    }
}
