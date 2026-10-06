//! A placed rail joins the rails around it and they turn to it: the Java game's `RailState`
//! (docs/research/standing-and-shapes.md, B5). Taking a rail away leaves the others as they are.
// TODO: no power, so a plain rail at a junction never switches and `rail_data_bit` stays 0.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::World as Level;

use super::{doors, level, send};

const RAILS: &[&str] = &["minecraft:rail", "minecraft:golden_rail", "minecraft:detector_rail", "minecraft:activator_rail"];
const NS: u8 = 0;
const EW: u8 = 1;
const RISING_EAST: u8 = 2;
const RISING_WEST: u8 = 3;
const RISING_NORTH: u8 = 4;
const RISING_SOUTH: u8 = 5;
const SOUTH_EAST: u8 = 6;
const SOUTH_WEST: u8 = 7;
const NORTH_WEST: u8 = 8;
const NORTH_EAST: u8 = 9;

fn step([x, y, z]: BlockPos, [dx, dy, dz]: [i32; 3]) -> BlockPos {
    [x + dx, y + dy, z + dz]
}

const N: [i32; 3] = [0, 0, -1];
const S: [i32; 3] = [0, 0, 1];
const W: [i32; 3] = [-1, 0, 0];
const E: [i32; 3] = [1, 0, 0];
const UP: [i32; 3] = [0, 1, 0];

/// Where a rail of `shape` at `pos` leads.
fn ends(pos: BlockPos, shape: u8) -> Vec<BlockPos> {
    let up = |side: [i32; 3]| step(step(pos, side), UP);
    match shape {
        NS => vec![step(pos, N), step(pos, S)],
        EW => vec![step(pos, W), step(pos, E)],
        RISING_EAST => vec![step(pos, W), up(E)],
        RISING_WEST => vec![up(W), step(pos, E)],
        RISING_NORTH => vec![up(N), step(pos, S)],
        RISING_SOUTH => vec![step(pos, N), up(S)],
        SOUTH_EAST => vec![step(pos, E), step(pos, S)],
        SOUTH_WEST => vec![step(pos, W), step(pos, S)],
        NORTH_WEST => vec![step(pos, W), step(pos, N)],
        _ => vec![step(pos, E), step(pos, N)],
    }
}

fn is_rail(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| RAILS.contains(&state.name))
}

/// A rail as the algorithm sees it: where it is, whether it only runs straight, where it leads.
struct Rail {
    pos: BlockPos,
    straight: bool,
    ends: Vec<BlockPos>,
}

impl Rail {
    fn at(level: &Level, pos: BlockPos) -> Option<Rail> {
        let state = level.registry().get(level.block(pos[0], pos[1], pos[2])).filter(|state| RAILS.contains(&state.name))?;
        let shape = state.property("rail_direction")?.parse().ok()?;
        Some(Rail { pos, straight: state.name != RAILS[0], ends: ends(pos, shape) })
    }

    /// The rail at `pos`, or one up or one down from it.
    fn near(level: &Level, pos: BlockPos) -> Option<Rail> {
        [pos, step(pos, UP), step(pos, [0, -1, 0])].into_iter().filter(|pos| level.in_height(pos[1])).find_map(|pos| Rail::at(level, pos))
    }

    fn leads_to(&self, [x, _, z]: BlockPos) -> bool {
        self.ends.iter().any(|end| end[0] == x && end[2] == z)
    }

    /// Forgets the ends that no rail leading back meets (Java `removeSoftConnections`).
    fn forget_loose_ends(&mut self, level: &Level) {
        let pos = self.pos;
        self.ends = self.ends.iter().filter_map(|end| Rail::near(level, *end).filter(|rail| rail.leads_to(pos)).map(|rail| rail.pos)).collect();
    }

    fn can_join(&self, other: &Rail) -> bool {
        self.leads_to(other.pos) || self.ends.len() != 2
    }

    /// Whether the rail beside at `pos` would join this one (Java `hasNeighborRail`).
    fn is_joined_from(&self, level: &Level, pos: BlockPos) -> bool {
        Rail::near(level, pos).is_some_and(|mut rail| {
            rail.forget_loose_ends(level);
            rail.can_join(self)
        })
    }

    /// The shape of this rail turned to `other` too (Java `connectTo`).
    fn turned_to(&mut self, level: &Level, other: BlockPos) -> u8 {
        self.ends.push(other);
        let [n, s, w, e] = [N, S, W, E].map(|side| self.leads_to(step(self.pos, side)));
        let mut shape = None;
        if n || s {
            shape = Some(NS);
        }
        if w || e {
            shape = Some(EW);
        }
        if !self.straight {
            shape = corner(n, s, w, e).or(shape);
        }
        rising(level, self.pos, shape).unwrap_or(NS)
    }
}

/// The curve a rail joined on exactly two sides at a right angle takes.
fn corner(n: bool, s: bool, w: bool, e: bool) -> Option<u8> {
    match (n, s, w, e) {
        (false, true, false, true) => Some(SOUTH_EAST),
        (false, true, true, false) => Some(SOUTH_WEST),
        (true, false, true, false) => Some(NORTH_WEST),
        (true, false, false, true) => Some(NORTH_EAST),
        _ => None,
    }
}

/// A straight shape made to rise towards a rail one up at either end.
fn rising(level: &Level, pos: BlockPos, shape: Option<u8>) -> Option<u8> {
    let up = |side: [i32; 3]| is_rail(level, step(step(pos, side), UP));
    Some(match shape? {
        NS if up(N) => RISING_NORTH,
        NS if up(S) => RISING_SOUTH,
        EW if up(E) => RISING_EAST,
        EW if up(W) => RISING_WEST,
        shape => shape,
    })
}

/// The shape a rail placed at `pos` takes among its neighbours (Java `RailState.place`, unpowered).
fn placed_shape(level: &Level, rail: &Rail, default: u8) -> u8 {
    let [n, s, w, e] = [N, S, W, E].map(|side| rail.is_joined_from(level, step(rail.pos, side)));
    let (along_z, along_x) = (n || s, w || e);
    let mut shape = match (along_z, along_x) {
        (true, false) => Some(NS),
        (false, true) => Some(EW),
        _ => None,
    };
    if !rail.straight {
        shape = corner(n, s, w, e).or(shape);
    }
    if shape.is_none() {
        shape = if along_z && along_x { Some(default) } else { None };
        if !rail.straight {
            // The last that fits wins.
            for (fits, curve) in [(n && w, NORTH_WEST), (n && e, NORTH_EAST), (s && w, SOUTH_WEST), (s && e, SOUTH_EAST)] {
                if fits {
                    shape = Some(curve);
                }
            }
        }
    }
    rising(level, rail.pos, shape).unwrap_or(default)
}

fn set_shape(world: &mut World, layer: Entity, pos: BlockPos, shape: u8) {
    let Some(level) = level(world, layer) else { return };
    let id = level.block(pos[0], pos[1], pos[2]);
    if let Some(new) = doors::with(level.registry(), id, "rail_direction", &shape.to_string()).filter(|new| *new != id) {
        send::set(world, layer, pos, new);
    }
}

/// Shapes the rail just placed at `pos` to the rails around it, and turns those that can to it.
pub(super) fn join_placed(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(rail) = level(world, layer).and_then(|level| Rail::at(level, pos)) else { return };
    let default = level(world, layer).and_then(|level| level.registry().get(level.block(pos[0], pos[1], pos[2]))?.property("rail_direction")?.parse().ok()).unwrap_or(NS);
    let Some(shape) = level(world, layer).map(|level| placed_shape(level, &rail, default)) else { return };
    set_shape(world, layer, pos, shape);
    let rail = Rail { ends: ends(pos, shape), ..rail };
    for end in &rail.ends {
        let Some(level) = level(world, layer) else { return };
        let Some(mut beside) = Rail::near(level, *end) else { continue };
        beside.forget_loose_ends(level);
        if beside.can_join(&rail) {
            let turned = beside.turned_to(level, rail.pos);
            set_shape(world, layer, beside.pos, turned);
        }
    }
}
