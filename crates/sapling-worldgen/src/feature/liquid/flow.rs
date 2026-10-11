//! One liquid tick as decoration's instatick runs it: `LiquidBlock::_tick` (148b5d060), its
//! `_trySpreadTo` (148b5e580), `_getSlopeDistance` (148b5f020) and `neighborChanged` (148b5cd30),
//! as docs/research/water.md transcribes them. Instaticking replaces whatever the liquid flows
//! into, without drops or waterlogging.

use super::detect::holds_water;
use crate::feature::ticks::Tick;
use crate::block::Block;
use crate::feature::view::View;
use crate::terrain::MIN_Y;

/// -x, +x, -z, +z: the order costs are taken and ties spread in; a side's opposite is `i ^ 1`.
const SIDES: [[i32; 3]; 4] = [[-1, 0, 0], [1, 0, 0], [0, 0, -1], [0, 0, 1]];
/// `BlockSource::updateNeighborsAt` (1415f89a0): x-1, x+1, z+1, z-1, y+1, y-1.
const NEIGHBORS: [[i32; 3]; 6] = [[-1, 0, 0], [1, 0, 0], [0, 0, 1], [0, 0, -1], [0, 1, 0], [0, -1, 0]];
/// Where `solidify` looks for water, in `LiquidBlockBase::neighborChanged`'s order.
const SOLIDIFY: [[i32; 3]; 5] = [[0, 0, 1], [0, 0, -1], [1, 0, 0], [-1, 0, 0], [0, 1, 0]];
const NO_DROP: i32 = 1000;

fn add(p: [i32; 3], d: [i32; 3]) -> [i32; 3] {
    [p[0] + d[0], p[1] + d[1], p[2] + d[2]]
}

fn block(view: &View, p: [i32; 3]) -> Block {
    view.block(p[0], p[1], p[2])
}

/// `getLiquidBlock`: a waterlogged plant holds a water source in its second layer.
fn liquid(view: &View, p: [i32; 3]) -> Block {
    let b = block(view, p);
    if b.is_waterlogged() { Block::Water } else { b }
}

fn depth(view: &View, p: [i32; 3], lava: bool) -> Option<i32> {
    let b = liquid(view, p);
    (b.is_fluid() && b.is_lava() == lava).then(|| b.liquid_depth().unwrap_or(0) as i32)
}

fn delay(lava: bool) -> u64 {
    if lava { 30 } else { 5 }
}

/// `_isLiquidBlocking` (148b5ee50): water asks the block's liquid detection (detect.rs), lava the type.
fn blocking(view: &View, p: [i32; 3], lava: bool) -> bool {
    let b = block(view, p);
    if lava { b.is_solid() || lava_blocking(b) } else { holds_water(b) }
}

/// Lava asks the block type instead (vtable +0xc8, base 141a28450: the material blocks motion or is
/// solid): these non-solid types override it to hold lava.
fn lava_blocking(b: Block) -> bool {
    let name = b.name();
    b.is_snow_layer()
        || b.multiface().is_some()
        || matches!(name, "azalea" | "flowering_azalea" | "big_dripleaf" | "small_dripleaf_block" | "ladder" | "waterlily" | "scaffolding")
        || ["bamboo", "rail", "candle", "campfire"].iter().any(|part| name.contains(part))
}

/// Runs a tick if the cell still holds the liquid it was scheduled for; a tick within a block of
/// the 3x3's edge waits for the next generation (its neighbours' chunks are not in the source).
pub fn run(view: &mut View, tick: Tick) {
    // A tick that read a cell in doubt may flow to any side (reach.rs).
    view.reach_begin(true);
    run_tick(view, tick);
    view.reach_end(tick.at, 1);
}

fn run_tick(view: &mut View, tick: Tick) {
    if !view.can_tick(tick.at[0], tick.at[2]) {
        if !view.ticks.game && view.has_block(tick.at[0], MIN_Y, tick.at[2]) {
            view.ticks.add(tick.at, tick.lava, 0);
        }
        return;
    }
    if let Some(d) = depth(view, tick.at, tick.lava) {
        crate::feature::trace::line(|| format!("  tick {} {} {} {} depth {d}", tick.at[0], tick.at[1], tick.at[2], if tick.lava { "lava" } else { "water" }));
        self::tick(view, tick.at, tick.lava);
    }
}

/// `neighborChanged` for the liquid in each neighbour: lava may harden, then a tick unless one is due.
/// Instaticking passes it on too: LiquidBlock's constructor (148b5c4c0) sets the BlockLegacy flag
/// (+0x163 bit 3) that 1415f87d0 asks for then.
pub fn update_neighbors(view: &mut View, at: [i32; 3]) {
    for d in NEIGHBORS {
        let n = add(at, d);
        let l = liquid(view, n);
        if !l.is_fluid() {
            continue;
        }
        let lava = l.is_lava();
        if lava {
            solidify(view, n);
        }
        if depth(view, n, lava).is_some() && !view.ticks.has(n, lava, delay(lava)) {
            view.schedule(n, lava, delay(lava));
        }
    }
}

/// `LiquidBlockBase::solidify` (148b60f90): lava touching water beside or above turns to
/// obsidian (a source) or cobblestone.
fn solidify(view: &mut View, at: [i32; 3]) {
    let b = block(view, at);
    if !b.is_lava() {
        return;
    }
    if SOLIDIFY.iter().any(|&d| depth(view, add(at, d), false).is_some()) {
        let hard = if b == Block::Lava { Block::Obsidian } else { Block::Cobblestone };
        set(view, at, hard);
    }
}

/// `setBlock` with flags 3, which updates no neighbour while decoration ticks (worldgen-bds-liquids.md,
/// "Neighbour updates"); a waterlogged plant keeps its block (its water layer is not modelled).
fn set(view: &mut View, at: [i32; 3], b: Block) {
    if block(view, at).is_waterlogged() {
        return;
    }
    view.set_block(at[0], at[1], at[2], b);
}

fn tick(view: &mut View, p: [i32; 3], lava: bool) {
    let Some(mut d) = depth(view, p, lava) else { return };
    let drop = if lava { 2 } else { 1 };
    if d >= 1 {
        let (mut nearest, mut sources) = (None::<i32>, 0);
        for side in SIDES {
            let Some(nd) = depth(view, add(p, side), lava) else { continue };
            sources += (nd == 0) as i32;
            nearest = Some(if nd >= 8 { 0 } else { nearest.map_or(nd, |h| h.min(nd)) });
        }
        let mut new = nearest.map_or(-1, |h| if h + drop > 7 { -1 } else { h + drop });
        let above = depth(view, add(p, [0, 1, 0]), lava);
        if let Some(a) = above {
            new = if a >= 8 { a } else { a + 8 };
        }
        let below = add(p, [0, -1, 0]);
        if !lava && (sources >= 2 || sources == 1 && above == Some(0)) && (block(view, below).is_solid() || depth(view, below, lava) == Some(0)) {
            new = 0;
        }
        if new != d {
            if new < 0 {
                set(view, p, Block::Air);
            } else {
                set(view, p, Block::liquid(lava, new as u8));
                update_neighbors(view, p);
                view.schedule(p, lava, delay(lava));
            }
            d = new;
        }
    }
    // A cell that just dried (d = -1) still flows down, as depth 7: BDS tests for it only before the sides.
    let below = add(p, [0, -1, 0]);
    if below[1] >= MIN_Y && view.has_block(below[0], below[1], below[2]) {
        let l = liquid(view, below);
        if !(l.is_fluid() && (l.is_lava() == lava || l.is_lava())) && !blocking(view, below, lava) {
            if lava && l.is_fluid() {
                set(view, below, Block::Stone);
            } else {
                spread_to(view, below, if d >= 8 { d } else { d + 8 }, lava);
            }
            return;
        }
    }
    if d < 0 || d != 0 && !blocking(view, below, lava) {
        return;
    }
    let mut costs = [NO_DROP; 4];
    for (i, side) in SIDES.into_iter().enumerate() {
        let n = add(p, side);
        if blocking(view, n, lava) || depth(view, n, lava) == Some(0) {
            continue;
        }
        costs[i] = if !blocking(view, add(n, [0, -1, 0]), lava) {
            0
        } else {
            let own = depth(view, p, lava).unwrap_or(-1);
            slope(view, n, if (5..=7).contains(&own) { own - 3 } else { 1 }, i, lava)
        };
    }
    let next = if d >= 8 { 1 } else { d + drop };
    if next > 7 {
        return;
    }
    let cheapest = *costs.iter().min().expect("four sides");
    for (i, side) in SIDES.into_iter().enumerate() {
        if costs[i] == cheapest {
            spread_to(view, add(p, side), next, lava);
        }
    }
}

/// How far the nearest drop is from `q`, reached by side `from`; `NO_DROP` beyond reach.
fn slope(view: &View, q: [i32; 3], pass: i32, from: usize, lava: bool) -> i32 {
    let limit = if lava { 2 } else { 4 };
    let mut best = NO_DROP;
    for (i, side) in SIDES.into_iter().enumerate() {
        if i == from ^ 1 {
            continue;
        }
        let n = add(q, side);
        if blocking(view, n, lava) || depth(view, n, lava) == Some(0) {
            continue;
        }
        if !blocking(view, add(n, [0, -1, 0]), lava) {
            return pass;
        }
        if pass < limit {
            best = best.min(slope(view, n, pass + 1, i, lava));
        }
    }
    best
}

fn spread_to(view: &mut View, n: [i32; 3], depth: i32, lava: bool) {
    if n[1] < MIN_Y || !view.has_block(n[0], n[1], n[2]) {
        return;
    }
    let l = liquid(view, n);
    if l.is_fluid() && (l.is_lava() == lava || l.is_lava()) || blocking(view, n, lava) {
        return;
    }
    set(view, n, Block::liquid(lava, depth as u8));
    // Lava set beside water stays lava until a neighbour's update reaches it (`update_neighbors`).
    update_neighbors(view, n);
    view.schedule(n, lava, delay(lava));
}
