//! Liquid flow at the end of `decorationPostProcessChunk` (1482143a0): the bottom of every water
//! run in the chunk's 60 border columns is reset to a source and ticked, then the ticks queued
//! during decoration (springs, those borders, what they spread to) run as instaticks, one
//! `tickPendingTicks` call per generation, at most 128, over the chunk's 3x3 neighbourhood.
//! The flow rule is docs/research/water.md's (`LiquidBlock::_tick` 148b5d060).

mod flow;

use super::ticks::TickQueue;
use super::view::View;
use crate::block::Block;

const GENERATIONS: usize = 128;

/// The `View::on_write` hook for decoration: features write with flags 3, so liquids beside a
/// write get a tick. `SAPLING_LIQUID_FEATURE_NOTIFY=0` turns it off.
pub fn write_hook() -> Option<fn(&mut View, [i32; 3])> {
    let on = std::env::var("SAPLING_LIQUID_FEATURE_NOTIFY").map_or(true, |v| v != "0");
    on.then_some(flow::update_neighbors as fn(&mut View, [i32; 3]))
}

pub fn settle(view: &mut View) {
    reset_border_water(view);
    let mut generated = TickQueue::default();
    for (at, lava) in view.own_fluid_ticks() {
        generated.add(at, lava, 1);
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
                view.ticks.add([x, y, z], false, 1);
            }
            below_dry = !water;
        }
    }
}
