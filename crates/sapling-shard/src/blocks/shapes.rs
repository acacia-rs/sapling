//! Blocks whose state follows their neighbours: fences, panes and bars join what is beside them,
//! walls also rise where something stands on them, stairs turn corners, and a gate sinks into a
//! wall (docs/research/standing-and-shapes.md, part B: walls are BDS's rule, the rest Java's).
// TODO: who joins whom is Java's lists (BDS's default connection rules were not found); a
// block's outline is taken to be its collision boxes.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{BlockRegistry, World as Level};

use super::{level, send};

/// North, east, south, west: their names in states and their steps.
const SIDES: [(&str, [i32; 2]); 4] = [
    ("north", [0, -1]),
    ("east", [1, 0]),
    ("south", [0, 1]),
    ("west", [-1, 0]),
];
/// Blocks with a full side that nothing joins (Java `Block.isExceptionForConnection`).
const ALOOF: &[&str] = &["leaves", "barrier", "pumpkin", "melon_block", "shulker_box"];
/// A wall's arm towards each side, and its middle, in its cell (x and z, from and to).
const ARMS: [[f32; 4]; 4] = [
    [0.3125, 0.6875, 0.0, 0.25],
    [0.75, 1.0, 0.3125, 0.6875],
    [0.3125, 0.6875, 0.75, 1.0],
    [0.0, 0.25, 0.3125, 0.6875],
];
const POST: [f32; 4] = [0.25, 0.75, 0.25, 0.75];
/// A stair's `weirdo_direction` as a step: 0 east, 1 west, 2 south, 3 north.
const STAIR_STEPS: [[i32; 2]; 4] = [[1, 0], [-1, 0], [0, 1], [0, -1]];

#[derive(Clone, Copy, PartialEq)]
enum Family {
    WoodenFence,
    NetherFence,
    Pane,
    Wall,
}

struct Cell {
    name: &'static str,
    properties: &'static str,
    boxes: &'static [sapling_world::Aabb],
    is_full: bool,
}

fn cell(level: &Level, [x, y, z]: BlockPos) -> Option<Cell> {
    let state = level
        .in_height(y)
        .then(|| level.registry().get(level.block(x, y, z)))
        .flatten()?;
    Some(Cell {
        name: state.name,
        properties: state.properties,
        boxes: state.boxes,
        is_full: state.is_full_cube(),
    })
}

fn get<'a>(properties: &'a str, key: &str) -> Option<&'a str> {
    properties
        .split(',')
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| *name == key)
        .map(|(_, value)| value)
}

fn family(cell: &Cell) -> Option<Family> {
    Some(match cell.name {
        _ if get(cell.properties, "wall_post_bit").is_some() => Family::Wall,
        _ if get(cell.properties, "minecraft:connection_north").is_none() => return None,
        "minecraft:nether_brick_fence" => Family::NetherFence,
        _ if cell.name.ends_with("_fence") => Family::WoodenFence,
        _ if cell.name.ends_with("glass_pane") || cell.name == "minecraft:iron_bars" => {
            Family::Pane
        }
        _ => return None,
    })
}

/// Whether a gate's line runs along the step `[dx, dz]`: one that faces north or south lies
/// east to west.
fn gate_lies_along(cell: &Cell, [dx, _]: [i32; 2]) -> bool {
    cell.name.ends_with("fence_gate")
        && get(cell.properties, "minecraft:cardinal_direction")
            .is_some_and(|facing| (facing == "north" || facing == "south") == (dx != 0))
}

/// Whether a block of `family` joins the block `beside` it, one step `step` away.
fn joins(family: Family, beside: &Cell, step: [i32; 2]) -> bool {
    let theirs = self::family(beside);
    let kin = match family {
        Family::Pane => matches!(theirs, Some(Family::Pane | Family::Wall)),
        Family::Wall => matches!(theirs, Some(Family::Pane | Family::Wall)),
        fence => theirs == Some(fence),
    };
    let gate = family != Family::Pane && gate_lies_along(beside, step);
    kin || gate || beside.is_full && !ALOOF.iter().any(|name| beside.name.contains(name))
}

/// Whether one of `boxes`, of the block above, reaches down into the square `[x0, x1, z0, z1]`.
fn rests_on(boxes: &[sapling_world::Aabb], [x0, x1, z0, z1]: [f32; 4]) -> bool {
    boxes.iter().any(|shape| {
        shape.min[1] < 0.01
            && shape.min[0] < x1
            && shape.max[0] > x0
            && shape.min[2] < z1
            && shape.max[2] > z0
    })
}

/// A wall's sides and post (BDS `WallBlock::tryFixWallStates`).
fn wall(level: &Level, pos: BlockPos) -> Vec<(String, &'static str)> {
    let above = cell(level, [pos[0], pos[1] + 1, pos[2]]);
    let boxes = above.as_ref().map_or(&[][..], |above| above.boxes);
    let arms: Vec<&str> = SIDES
        .iter()
        .zip(ARMS)
        .map(
            |((_, step), arm)| match cell(level, [pos[0] + step[0], pos[1], pos[2] + step[1]]) {
                Some(beside) if joins(Family::Wall, &beside, *step) => {
                    if rests_on(boxes, arm) {
                        "tall"
                    } else {
                        "short"
                    }
                }
                _ => "none",
            },
        )
        .collect();
    let [n, e, s, w] = [arms[0], arms[1], arms[2], arms[3]];
    let (along_z, along_x) = (
        n != "none" && n == s && e == "none" && w == "none",
        e != "none" && e == w && n == "none" && s == "none",
    );
    let wall_with_post_above = above
        .as_ref()
        .is_some_and(|above| get(above.properties, "wall_post_bit") == Some("1"));
    let post = match () {
        _ if wall_with_post_above => true,
        _ if rests_on(boxes, POST) => {
            if along_z {
                n == "short"
            } else if along_x {
                e == "short"
            } else {
                true
            }
        }
        _ => !(along_z || along_x),
    };
    let mut set: Vec<(String, &str)> = SIDES
        .iter()
        .zip(arms)
        .map(|((side, _), arm)| (format!("wall_connection_type_{side}"), arm))
        .collect();
    set.push(("wall_post_bit".to_owned(), if post { "1" } else { "0" }));
    set
}

/// A stair's step and half, when `cell` is a stair.
fn stair(cell: &Cell) -> Option<([i32; 2], &'static str)> {
    let facing = get(cell.properties, "weirdo_direction")?
        .parse::<usize>()
        .ok()?;
    Some((
        *STAIR_STEPS.get(facing)?,
        get(cell.properties, "upside_down_bit")?,
    ))
}

/// The corner a stair at `pos` turns (Java `StairBlock.getStairsShape`).
fn corner(level: &Level, pos: BlockPos, facing: [i32; 2], half: &str) -> &'static str {
    let at = |[dx, dz]: [i32; 2]| {
        cell(level, [pos[0] + dx, pos[1], pos[2] + dz])
            .as_ref()
            .and_then(stair)
            .map(|(facing, half)| (facing, half.to_owned()))
    };
    let across = |other: [i32; 2]| (other[0] != 0) != (facing[0] != 0);
    let ccw = [facing[1], -facing[0]];
    let takes = |step: [i32; 2]| {
        at(step).is_none_or(|(theirs, their_half)| theirs != facing || their_half != half)
    };
    if let Some((front, _)) =
        at(facing).filter(|(front, front_half)| front_half == half && across(*front))
    {
        if takes([-front[0], -front[1]]) {
            return if front == ccw {
                "outer_left"
            } else {
                "outer_right"
            };
        }
    }
    if let Some((back, _)) =
        at([-facing[0], -facing[1]]).filter(|(back, back_half)| back_half == half && across(*back))
    {
        if takes(back) {
            return if back == ccw {
                "inner_left"
            } else {
                "inner_right"
            };
        }
    }
    "none"
}

/// The properties the neighbours of the block at `pos` decide for it; none for a block whose
/// state does not follow them.
fn decided(level: &Level, pos: BlockPos) -> Vec<(String, &'static str)> {
    let Some(this) = cell(level, pos) else {
        return Vec::new();
    };
    let beside = |step: [i32; 2]| cell(level, [pos[0] + step[0], pos[1], pos[2] + step[1]]);
    if let Some(family) = family(&this) {
        if family == Family::Wall {
            return wall(level, pos);
        }
        let joined = |step: [i32; 2]| beside(step).is_some_and(|cell| joins(family, &cell, step));
        return SIDES
            .iter()
            .map(|(side, step)| {
                (
                    format!("minecraft:connection_{side}"),
                    if joined(*step) { "1" } else { "0" },
                )
            })
            .collect();
    }
    if let Some((facing, half)) =
        stair(&this).filter(|_| get(this.properties, "minecraft:corner").is_some())
    {
        return vec![(
            "minecraft:corner".to_owned(),
            corner(level, pos, facing, half),
        )];
    }
    if this.name.ends_with("fence_gate") {
        let line = if get(this.properties, "minecraft:cardinal_direction")
            .is_some_and(|facing| facing == "north" || facing == "south")
        {
            [[1, 0], [-1, 0]]
        } else {
            [[0, 1], [0, -1]]
        };
        let in_wall = line
            .iter()
            .any(|step| beside(*step).as_ref().and_then(family) == Some(Family::Wall));
        return vec![("in_wall_bit".to_owned(), if in_wall { "1" } else { "0" })];
    }
    Vec::new()
}

/// The state `id` with each of `set` changed.
fn with_all(registry: &BlockRegistry, id: u32, set: &[(String, &str)]) -> Option<u32> {
    let state = registry.get(id)?;
    let pair = |pair: &str| match pair
        .split_once('=')
        .and_then(|(key, _)| set.iter().find(|(name, _)| name == key))
    {
        Some((key, value)) => format!("{key}={value}"),
        None => pair.to_owned(),
    };
    registry.find(
        state.name,
        &state
            .properties
            .split(',')
            .map(pair)
            .collect::<Vec<_>>()
            .join(","),
    )
}

/// Walls a change can run down: a post raises one under it, so a column of walls is a chain.
const MAX_WALL_RUN: usize = 64;

/// Gives the blocks a change at `pos` can reshape (it, the four beside it, and the walls under
/// any of them that changed) the state their neighbours now decide, for everyone who sees them.
pub(super) fn reshape_around(world: &mut World, layer: Entity, pos: BlockPos) {
    let [x, y, z] = pos;
    let mut cells = vec![
        pos,
        [x, y, z - 1],
        [x + 1, y, z],
        [x, y, z + 1],
        [x - 1, y, z],
        [x, y - 1, z],
    ];
    let mut runs = 0;
    while let Some(cell) = cells.pop() {
        let Some(level) = level(world, layer)
            .filter(|level| level.in_height(cell[1]) && level.is_loaded(send::column(cell)))
        else {
            continue;
        };
        let id = level.block(cell[0], cell[1], cell[2]);
        let set = decided(level, cell);
        let Some(new) = (!set.is_empty())
            .then(|| with_all(level.registry(), id, &set))
            .flatten()
            .filter(|new| *new != id)
        else {
            continue;
        };
        let is_wall = set.iter().any(|(key, _)| key == "wall_post_bit");
        if send::set(world, layer, cell, new) && is_wall && runs < MAX_WALL_RUN {
            runs += 1;
            cells.push([cell[0], cell[1] - 1, cell[2]]);
        }
    }
}
