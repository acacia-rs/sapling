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

// TODO: Java's ultrawarm numbers, unconfirmed on BDS.
fn delay(view: &View, lava: bool) -> u64 {
    match (lava, view.ultrawarm) {
        (false, _) => 5,
        (true, false) => 30,
        (true, true) => 10,
    }
}

/// `_isLiquidBlocking` (148b5ee50): water asks the block's liquid detection (detect.rs), lava the type.
fn blocking(view: &View, p: [i32; 3], lava: bool) -> bool {
    let b = block(view, p);
    if lava { lava_blocking(b) } else { holds_water(b) }
}

/// Lava asks the block type instead (vtable +0xc8; read per type from the vtables, worldgen-bds-liquids.md,
/// "Lava"): the base (141a28450) says whether the material blocks motion or is solid, 46 types answer
/// yes whatever their material and 48 answer no.
fn lava_blocking(b: Block) -> bool {
    let name = b.name();
    let coral = name.contains("coral") && !name.ends_with("coral_block");
    let yields = b.is_snow_layer()
        || coral
        || name.ends_with("rail")
        || name.starts_with("chorus")
        || matches!(name, "azalea" | "flowering_azalea" | "big_dripleaf" | "small_dripleaf_block" | "waterlily" | "powder_snow" | "shelf_mushroom");
    // Material 24 (neither solid nor blocking motion) on the base rule: lava takes a mineshaft's webs and torches.
    let yields = yields || name.ends_with("torch") || matches!(name, "web" | "lever" | "tripwire_hook" | "trip_wire" | "redstone_wire");
    let holds = b.multiface().is_some()
        || name.contains("candle")
        || matches!(name, "bamboo" | "campfire" | "soul_campfire" | "ladder" | "scaffolding" | "frame" | "glow_frame" | "portal" | "end_portal" | "resin_clump");
    !yields && (holds || b.is_solid())
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
    if block(view, tick.at) == Block::Magma {
        crate::feature::trace::line(|| format!("  tick {} {} {} magma", tick.at[0], tick.at[1], tick.at[2]));
        magma_tick(view, tick.at);
    } else if let Some(d) = depth(view, tick.at, tick.lava) {
        crate::feature::trace::line(|| format!("  tick {} {} {} {} depth {d}", tick.at[0], tick.at[1], tick.at[2], if tick.lava { "lava" } else { "water" }));
        self::tick(view, tick.at, tick.lava);
    }
}

/// A magma block's tick (a blob queues it in 1; it rides the liquid queue as a water tick on a magma cell):
/// one bubble column cell in the water source above. The column's own ticks wait for the game
/// (docs/research/worldgen-bds-liquids.md, "Game time"), so it grows only under a game-tick replay.
fn magma_tick(view: &mut View, [x, mut y, z]: [i32; 3]) {
    let cells = if super::game_ticks() > 0 { usize::MAX } else { 1 };
    for _ in 0..cells {
        y += 1;
        if view.block(x, y, z) != Block::Water {
            break;
        }
        view.set_block(x, y, z, Block::BubbleColumn);
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
        if depth(view, n, lava).is_some() && !view.ticks.has(n, lava, delay(view, lava)) {
            view.schedule(n, lava, delay(view, lava));
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
    let drop = if lava && !view.ultrawarm { 2 } else { 1 };
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
            // Receding lava waits four times as long three times in four, by the thread's generator: both
            // waits end in the next generation, but the longer one sorts later in it.
            let mut wait = delay(view, lava);
            if lava && d < 8 && d < new && new < 8 && view.thread_random.as_mut().is_some_and(|random| random.next_int(4) != 0) {
                wait *= 4;
            }
            if new < 0 {
                set(view, p, Block::Air);
            } else {
                set(view, p, Block::liquid(lava, new as u8));
                update_neighbors(view, p);
                view.schedule(p, lava, wait);
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
    view.schedule(n, lava, delay(view, lava));
}
