//! `SpeleothemClusterFeature::place` (149d61dc0) and `_placeColumn`: Java 1.18's dripstone cluster
//! with a radius of 2..7, two- or three-block base layers, and Bedrock's column scan.

use super::{SpeleothemBlocks, empty_or_water, get, water};
use crate::block::Block;
use crate::feature::place::Ctx;
use crate::feature::view::View;
use crate::random::Mt19937;

/// How far the column scan looks up and down.
const SEARCH_RANGE: i32 = 12;

/// The cave around a cell: the solid floor and ceiling the column scan stopped at, if found.
pub struct Column {
    pub floor: Option<i32>,
    pub ceiling: Option<i32>,
}

/// `ColumnUtils::scan` (1438eb800, each way 1438eb960): from the cell, up then down, while `inside`
/// holds (at most `range - 1` cells past the first), to an `edge` cell. None unless the cell is inside.
pub fn scan(view: &View, at: [i32; 3], range: i32, inside: impl Fn(Block) -> bool, edge: impl Fn(Block) -> bool) -> Option<Column> {
    if !inside(get(view, at)) {
        return None;
    }
    let way = |dy: i32| {
        let mut y = at[1] + dy;
        for _ in 0..range - 1 {
            if !inside(view.block(at[0], y, at[2])) {
                break;
            }
            y += dy;
        }
        edge(view.block(at[0], y, at[2])).then_some(y)
    };
    Some(Column { ceiling: way(1), floor: way(-1) })
}

pub fn place(ctx: &mut Ctx, blocks: &SpeleothemBlocks, height: [i32; 2], wetness: [f32; 4], at: [i32; 3]) -> Option<[i32; 3]> {
    if !empty_or_water(get(ctx.view, at)) {
        return None;
    }
    let height = height[0] + ctx.random.next_int(height[1] - height[0] + 1);
    let [mean, deviation, min, max] = wetness;
    let wetness = (mean + deviation * ctx.random.next_gaussian()).max(min).min(max);
    let density = ctx.random.next_float() * 0.4 + 0.3;
    let rx = ctx.random.next_int(6) + 2;
    let rz = ctx.random.next_int(6) + 2;
    let shape = Shape { height, wetness, density };
    for dx in -rx..=rx {
        for dz in -rz..=rz {
            let edge = (rx - dx.abs()).min(rz - dz.abs()) as f32 / 3.0;
            let chance = if edge < 0.0 {
                0.1f32 as f64
            } else if edge > 1.0 {
                1.0
            } else {
                (edge * 0.9 + 0.1) as f64
            };
            column(ctx, blocks, &shape, [at[0] + dx, at[1], at[2] + dz], dx.abs() + dz.abs(), chance);
        }
    }
    Some(at)
}

struct Shape {
    height: i32,
    wetness: f32,
    density: f32,
}

fn lava_at(view: &View, x: i32, y: i32, z: i32) -> bool {
    view.block(x, y, z).is_lava()
}

/// Base blocks from the surface into the rock (`up` from a ceiling), 2 or 3 deep.
fn layer(view: &mut View, random: &mut Mt19937, blocks: &SpeleothemBlocks, [x, y, z]: [i32; 3], up: bool) {
    let thickness = random.next_int(2) + 2;
    let dy = if up { 1 } else { -1 };
    for i in 0..thickness {
        if !blocks.replace(view, [x, y + i * dy, z]) {
            return;
        }
    }
}

/// Java's `getDripstoneHeight`: 0 unless the density draw passes, else a normal around half the
/// room fading to 0 at 8 blocks from the centre, clamped to the room.
fn spike_height(random: &mut Mt19937, density: f32, distance: i32, room: i32) -> i32 {
    if random.next_float() > density {
        return 0;
    }
    let (room, t) = (room as f32, distance as f32 * 0.125);
    let mean = if t > 1.0 { 0.0 } else { (0.0 - room * 0.5) * t + room * 0.5 };
    let value = random.next_gaussian() * 3.0 + mean;
    (if room < value { room } else { value.max(0.0) }) as i32
}

fn column(ctx: &mut Ctx, blocks: &SpeleothemBlocks, shape: &Shape, at: [i32; 3], distance: i32, chance: f64) {
    let inside = |b: Block| empty_or_water(b);
    let edge = |b: Block| blocks.is_base(b) || b.is_lava();
    let Some(Column { mut floor, ceiling }) = scan(ctx.view, at, SEARCH_RANGE, inside, edge) else { return };
    if floor.is_none() && ceiling.is_none() {
        return;
    }
    let [x, _, z] = at;
    if ctx.random.next_float() < shape.wetness
        && let Some(f) = floor
        && can_pool(ctx.view, blocks, [x, f, z])
    {
        ctx.view.set_block_quiet(x, f, z, Block::Water);
        floor = Some(f - 1);
    }
    let mut stalactite = 0;
    if ctx.random.next_double() < chance
        && let Some(c) = ceiling
        && !lava_at(ctx.view, x, c, z)
    {
        layer(ctx.view, ctx.random, blocks, [x, c, z], true);
        let room = floor.map_or(shape.height, |f| shape.height.min(c - f));
        stalactite = spike_height(ctx.random, shape.density, distance, room);
    }
    let mut stalagmite = 0;
    if ctx.random.next_double() < chance
        && let Some(f) = floor
        && !lava_at(ctx.view, x, f, z)
    {
        layer(ctx.view, ctx.random, blocks, [x, f, z], false);
        stalagmite = if ceiling.is_some() {
            (stalactite + ctx.random.next_int(3) - 1).max(0)
        } else {
            spike_height(ctx.random, shape.density, distance, shape.height)
        };
    }
    if let (Some(c), Some(f)) = (ceiling, floor)
        && c - stalactite <= f + stalagmite
    {
        let low = (c - stalactite).max(f + 1);
        let high = (f + stalagmite).min(c - 1);
        let meet = if low <= high + 1 { low + ctx.random.next_int(high - low + 2) } else { low };
        stalagmite = meet - f - 1;
        stalactite = c - meet;
    }
    let joined = ctx.random.next_boolean();
    let merge = match (ceiling, floor) {
        (Some(c), Some(f)) => joined && stalactite > 0 && stalagmite > 0 && stalactite + stalagmite == c - f - 1,
        _ => false,
    };
    if let Some(c) = ceiling {
        blocks.grow(ctx.view, [x, c - 1, z], false, stalactite, merge);
    }
    if let Some(f) = floor {
        blocks.grow(ctx.view, [x, f + 1, z], true, stalagmite, merge);
    }
}

/// `canPlacePool` (14829e3d0): not on base, spike or water, no water above, and stone, deepslate,
/// tuff or water on the four sides and below.
fn can_pool(view: &View, blocks: &SpeleothemBlocks, [x, y, z]: [i32; 3]) -> bool {
    let block = view.block(x, y, z);
    if block == blocks.base || water(block) || same_type(block, blocks.spike) || water(view.block(x, y + 1, z)) {
        return false;
    }
    let holds = |b: Block| matches!(b, Block::Stone | Block::Deepslate | Block::Tuff) || water(b);
    [(0, 0, -1), (1, 0, 0), (0, 0, 1), (-1, 0, 0), (0, -1, 0)].iter().all(|&(dx, dy, dz)| holds(view.block(x + dx, y + dy, z + dz)))
}

fn same_type(block: Block, spike: Block) -> bool {
    (spike as usize..spike as usize + 10).contains(&(block as usize))
}
