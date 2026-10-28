//! Liquid flow at the end of `decorationPostProcessChunk` (1482143a0): the bottom of every water
//! run in the chunk's 60 border columns is reset to a source and ticked, then the ticks queued
//! during decoration (springs, those borders, what they spread to) run as instaticks, one
//! `tickPendingTicks` call per generation, at most 128, over the chunk's 3x3 neighbourhood.
//! The flow rule is docs/research/water.md's (`LiquidBlock::_tick` 148b5d060).

mod detect;
mod detect_table;
mod flow;
mod leftovers;

use super::ticks::TickQueue;
use super::view::{Routed, View};
use crate::block::Block;

const GENERATIONS: usize = 128;

/// The magma the structure stage set in the chunk: placing it queued the block's tick in 1 on the chunk's
/// queue, after generation's aquifer ticks (one-worker run: a ruined portal's magma ticked in its chunk's decoration).
pub fn structure_magma(shape: &crate::terrain::ChunkShape, chunk: (i32, i32)) -> Vec<[i32; 3]> {
    let at = |i: usize| [chunk.0 * 16 + (i & 15) as i32, (i / 256) as i32 + crate::terrain::MIN_Y, chunk.1 * 16 + ((i >> 4) & 15) as i32];
    (0..16 * 16 * crate::terrain::HEIGHT).filter(|&i| shape.block_at(i) == Block::Magma).map(at).collect()
}

/// `structure`: the chunk's structure-stage magma (`structure_magma`); `inherited`: the ticks earlier sources
/// left on this chunk's queue (docs/research/worldgen-bds-liquids.md, "How they run").
pub fn settle(view: &mut View, structure: &[[i32; 3]], inherited: &[Routed]) {
    // Trace lines follow what they sum up (examples/feature_trace.rs prints a try's lines before it).
    super::trace::line(|| format!("{:?} liquid: features done, {} inherited ticks", view.chunk(), inherited.len()));
    reset_border_water(view);
    super::trace::line(|| format!("{:?} liquid border reset", view.chunk()));
    let mut generated = TickQueue::chunk();
    for (at, lava) in view.own_fluid_ticks() {
        super::trace::line(|| format!("  own tick {} {} {}", at[0], at[1], at[2]));
        generated.add(at, lava, 1);
    }
    for &at in structure {
        super::trace::line(|| format!("  structure magma tick {} {} {}", at[0], at[1], at[2]));
        generated.add(at, false, 1);
    }
    // What this decoration left on its own chunk's queue (magma ticks) is taken with the rest.
    let chunk = view.chunk();
    let (own, others): (Vec<Routed>, Vec<Routed>) = std::mem::take(&mut view.ticks.in_game).into_iter().partition(|(at, ..)| (at[0] >> 4, at[2] >> 4) == chunk);
    view.ticks.in_game = others;
    for &(at, lava, delay) in inherited.iter().chain(&own) {
        super::trace::line(|| format!("  inherited tick {} {} {} delay {delay}", at[0], at[1], at[2]));
        generated.add(at, lava, delay);
    }
    super::trace::line(|| format!("{:?} liquid queue", view.chunk()));
    view.ticks.acquire(generated);
    instatick(view);
}

/// The instatick generations over what the decoration queued, then the chunk queues' game ticks. The Nether's
/// decoration ends with this alone: no border reset, and the chunk's own queue waits for game time.
pub fn instatick(view: &mut View) {
    for generation in 0..GENERATIONS {
        if view.ticks.is_empty() {
            break;
        }
        let ticks = view.ticks.take_generation();
        let count = ticks.len();
        view.ticks.instaticking = true;
        for tick in ticks {
            flow::run(view, tick);
        }
        view.ticks.instaticking = false;
        super::trace::line(|| format!("{:?} liquid generation {generation}: {count} ticks", view.chunk()));
    }
    view.routed = view.ticks.in_game.clone();
    play(view);
}

/// The chunk's queue in game after decoration: the ticks that could not join the block source's
/// queue, then the instaticks left over, run by game tick for `game_ticks()` ticks over the same
/// 3x3 (BDS then has every neighbour; this is the approximation). Without that replay the
/// leftovers go out with the chunk instead (leftovers.rs).
fn play(view: &mut View) {
    let budget = game_ticks();
    let mut game = TickQueue::game();
    for (at, lava, delay) in std::mem::take(&mut view.ticks.in_game) {
        game.add(at, lava, delay + 1);
    }
    // Leftover instaticks keep their wrapped times (delay - 1); deferred ones (u64::MAX) never come due.
    let left = view.ticks.drain();
    for tick in &left {
        game.add_at(tick.at, tick.lava, tick.time);
    }
    if budget == 0 {
        view.left = leftovers::due(&left);
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
                    view.set_block(x, y, z, Block::Water);
                }
                view.schedule([x, y, z], false, 1);
            }
            below_dry = !water;
        }
    }
}
