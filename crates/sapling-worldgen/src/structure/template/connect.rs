//! Neighbour-shaped states of structure blocks, set from their final neighbours as BDS's saved
//! blocks show them: fence, pane, bar and wall connections and stair corners. Placed states
//! arrive with these unset.

use std::collections::HashSet;
use std::sync::OnceLock;

use super::{FULL_CUBE, flags, try_named, wall};
use crate::block::Block;
use crate::structure::canvas::Canvas;
use crate::structure::mineshaft::full_solid;

pub(super) const SIDES: [(&str, i32, i32); 4] = [("east", 1, 0), ("north", 0, -1), ("south", 0, 1), ("west", -1, 0)];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Shape {
    WoodenFence,
    NetherFence,
    Pane,
    Stairs,
}

/// `shape` and whether it is a wall, of every named block by `ALL` index and every template block
/// by palette index: the passes ask them of every cell they scan.
struct Kinds {
    named: Vec<(Option<Shape>, bool)>,
    templates: Vec<(Option<Shape>, bool)>,
}

fn kinds(block: Block) -> (Option<Shape>, bool) {
    static KINDS: OnceLock<Kinds> = OnceLock::new();
    let kinds = KINDS.get_or_init(|| {
        let of = |block: Block| (shape_by_states(block), block.states().contains("wall_post_bit"));
        Kinds { named: Block::ALL.iter().map(|&b| of(b)).collect(), templates: Block::templates().map(of).collect() }
    });
    match block {
        Block::Template(index) => kinds.templates[index as usize],
        named => kinds.named[named.index()],
    }
}

pub(super) fn shape(block: Block) -> Option<Shape> {
    kinds(block).0
}

pub fn is_wall(block: Block) -> bool {
    kinds(block).1
}

fn shape_by_states(block: Block) -> Option<Shape> {
    let (name, states) = (block.name(), block.states());
    if states.contains("minecraft:corner=") {
        Some(Shape::Stairs)
    } else if !states.contains("minecraft:connection_") {
        None
    } else if name == "nether_brick_fence" {
        Some(Shape::NetherFence)
    } else if name.ends_with("_fence") {
        Some(Shape::WoodenFence)
    } else {
        Some(Shape::Pane)
    }
}

/// By the palette's flags where it has the block (chests and stairs are not full), else solid.
pub(super) fn full_cube(block: Block) -> bool {
    match (block, flags(block)) {
        (Block::Template(_), f) => f & FULL_CUBE != 0,
        (_, 0) => full_solid(block),
        (_, f) => f & FULL_CUBE != 0,
    }
}

/// Whether `neighbour`, lying `(dx, dz)` from the connecting block, shows it a full face: any
/// face of a full cube but a gourd's (a taiga farm's wall beside its pumpkin stays apart), the tall
/// back of stairs, a composter's sides.
pub(super) fn sturdy(neighbour: Block, (dx, dz): (i32, i32)) -> bool {
    let gourd = matches!(neighbour.name(), "pumpkin" | "carved_pumpkin" | "lit_pumpkin" | "melon_block");
    full_cube(neighbour) && !gourd || shape(neighbour) == Some(Shape::Stairs) && facing(neighbour) == (-dx, -dz) || neighbour.name() == "composter"
}

pub(super) fn joins(kind: Shape, neighbour: Block, side: (i32, i32)) -> bool {
    let other = shape(neighbour);
    sturdy(neighbour, side)
        || match kind {
            Shape::WoodenFence | Shape::NetherFence => other == Some(kind) || neighbour.name().ends_with("fence_gate"),
            _ => other == Some(Shape::Pane) || neighbour.states().contains("wall_post_bit"),
        }
}

pub(super) fn with_states(block: Block, changes: &[(String, String)]) -> Block {
    let mut pairs: Vec<(&str, &str)> = block.states().split(',').filter_map(|p| p.split_once('=')).collect();
    for (key, value) in changes {
        if let Some(pair) = pairs.iter_mut().find(|(k, _)| k == key) {
            pair.1 = value;
        }
    }
    let states = pairs.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(",");
    let table = Block::with_states(block.name(), &states).filter(|b| b.is_waterlogged() == block.is_waterlogged());
    table.or_else(|| try_named(block.name(), &states, block.is_waterlogged())).unwrap_or(block)
}

pub(super) fn state(block: Block, key: &str) -> Option<&'static str> {
    block.states().split(',').filter_map(|p| p.split_once('=')).find(|(k, _)| *k == key).map(|(_, v)| v)
}

/// Java facing as (dx, dz) from Bedrock's `weirdo_direction` (0 east, 1 west, 2 south, 3 north).
fn facing(stairs: Block) -> (i32, i32) {
    match state(stairs, "weirdo_direction") {
        Some("0") => (1, 0),
        Some("1") => (-1, 0),
        Some("2") => (0, 1),
        _ => (0, -1),
    }
}

/// Java's `StairBlock.getStairsShape`, corners named as Bedrock saves them.
pub(super) fn corner(canvas: &Canvas, stairs: Block, (x, y, z): (i32, i32, i32)) -> &'static str {
    let (fx, fz) = facing(stairs);
    let half = state(stairs, "upside_down_bit");
    let stair_at = |dx: i32, dz: i32| Some(canvas.get(x + dx, y, z + dz)).filter(|&b| shape(b) == Some(Shape::Stairs) && state(b, "upside_down_bit") == half);
    let takes = |dx: i32, dz: i32| stair_at(dx, dz).is_none_or(|n| facing(n) != (fx, fz));
    let ccw = (fz, -fx);
    if let Some(front) = stair_at(fx, fz) {
        let f = facing(front);
        if (f.0 != 0) != (fx != 0) && takes(-f.0, -f.1) {
            return if f == ccw { "outer_left" } else { "outer_right" };
        }
    }
    if let Some(back) = stair_at(-fx, -fz) {
        let b = facing(back);
        if (b.0 != 0) != (fx != 0) && takes(b.0, b.1) {
            return if b == ccw { "inner_left" } else { "inner_right" };
        }
    }
    "none"
}

/// Every neighbour-shaped template block of `canvas`'s chunk between heights `low` and `high`, but
/// those in `settled` (a replayed run's template passes decided them, update.rs).
pub fn rejoin(canvas: &mut Canvas, (low, high): (i32, i32), settled: &HashSet<(i32, i32, i32)>) {
    let b = canvas.bb;
    let (low, high) = (low.max(b.y0), high.min(b.y1));
    // Bottom up: each wall sees the wall above as its template placed it.
    for y in low..=high {
        for z in b.z0..=b.z1 {
            for x in b.x0..=b.x1 {
                let block = canvas.get(x, y, z);
                if wall::is_wall(block) && !settled.contains(&(x, y, z)) {
                    let joined = with_states(block, &wall::changes(canvas, (x, y, z)));
                    canvas.set(x, y, z, joined);
                }
            }
        }
    }
    let mut changed = Vec::new();
    for y in low..=high {
        for z in b.z0..=b.z1 {
            for x in b.x0..=b.x1 {
                let block = canvas.get(x, y, z);
                let Some(kind) = shape(block).filter(|_| !wall::is_wall(block) && !settled.contains(&(x, y, z))) else { continue };
                let changes: Vec<(String, String)> = if kind == Shape::Stairs {
                    vec![("minecraft:corner".into(), corner(canvas, block, (x, y, z)).into())]
                } else {
                    SIDES
                        .iter()
                        .map(|(side, dx, dz)| {
                            let joined = joins(kind, canvas.get(x + dx, y, z + dz), (*dx, *dz));
                            (format!("minecraft:connection_{side}"), u8::from(joined).to_string())
                        })
                        .collect()
                };
                changed.push(((x, y, z), with_states(block, &changes)));
            }
        }
    }
    for ((x, y, z), block) in changed {
        canvas.set(x, y, z, block);
    }
}
