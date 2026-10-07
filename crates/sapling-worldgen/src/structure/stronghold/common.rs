//! Stronghold block helpers: the stone brick selector box, small doors (`generateSmallDoor`
//! 14809bdf0) and BDS's orientation tables (`getOrientationData` 141f4a1d0) for each block kind.

use super::Door;
use crate::block::Block;
use crate::random::Mt19937;
use crate::structure::BoundingBox;
use crate::structure::canvas::Canvas;
use crate::structure::mineshaft::full_solid;
use crate::structure::piece::{EAST, Frame, NORTH, SOUTH, WEST};

/// Java's `BoundingBox.orientBox`.
pub fn orient_box((x, y, z): (i32, i32, i32), (ox, oy, oz): (i32, i32, i32), (sx, sy, sz): (i32, i32, i32), dir: u8) -> BoundingBox {
    match dir {
        NORTH => BoundingBox::new(x + ox, y + oy, z - sz + 1 + oz, x + sx - 1 + ox, y + sy - 1 + oy, z + oz),
        WEST => BoundingBox::new(x - sz + 1 + oz, y + oy, z + ox, x + oz, y + sy - 1 + oy, z + sx - 1 + ox),
        EAST => BoundingBox::new(x + oz, y + oy, z + ox, x + sz - 1 + oz, y + sy - 1 + oy, z + sx - 1 + ox),
        _ => BoundingBox::new(x + ox, y + oy, z + oz, x + sx - 1 + ox, y + sy - 1 + oy, z + sz - 1 + oz),
    }
}

/// `randomSmallDoor`: `nextInt(5)` 2 wood, 3 grates, 4 iron, else an opening.
pub fn random_door(random: &mut Mt19937) -> Door {
    match random.next_int(5) {
        2 => Door::Wood,
        3 => Door::Grates,
        4 => Door::Iron,
        _ => Door::Opening,
    }
}

/// Java's directions, as the pieces name the blocks they place before orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    East,
    West,
    South,
    North,
}

/// A wall torch by Java's facing: BDS's legacy torch data per orientation, inlined at each call.
pub fn torch(f: &Frame, facing: Dir) -> Block {
    let o = f.orientation.unwrap_or(SOUTH) as usize;
    let data = match facing {
        Dir::East => [1, 3, 1, 3],
        Dir::West => [2, 4, 2, 4],
        Dir::South => [4, 1, 3, 2],
        Dir::North => [3, 2, 4, 1],
    }[o];
    [Block::TorchUnknown, Block::TorchWest, Block::TorchEast, Block::TorchNorth, Block::TorchSouth][data]
}

/// `facing_direction` data 2..5 in the piece's orientation.
pub fn facing(f: &Frame, data: u8) -> u8 {
    let table: [u8; 4] = match f.orientation {
        Some(SOUTH) => [3, 2, 5, 4],
        Some(WEST) => [4, 5, 3, 2],
        Some(EAST) => [5, 4, 2, 3],
        _ => [2, 3, 4, 5],
    };
    table[data as usize - 2]
}

/// Legacy `direction` data 0..3 in the piece's orientation.
pub fn direction(f: &Frame, data: u8) -> u8 {
    let table: [u8; 4] = match f.orientation {
        Some(SOUTH) => [2, 3, 0, 1],
        Some(WEST) => [3, 0, 1, 2],
        Some(EAST) => [1, 2, 3, 0],
        _ => [0, 1, 2, 3],
    };
    table[data as usize]
}

pub fn ladder(f: &Frame, data: u8) -> Block {
    [Block::LadderNorth, Block::LadderSouth, Block::LadderWest, Block::LadderEast][facing(f, data) as usize - 2]
}

fn button(f: &Frame, data: u8) -> Block {
    [Block::StoneButtonNorth, Block::StoneButtonSouth, Block::StoneButtonWest, Block::StoneButtonEast][facing(f, data) as usize - 2]
}

/// The two halves of a door facing legacy direction 3 in the piece's orientation.
fn door(f: &Frame, iron: bool) -> (Block, Block) {
    let first = if iron { Block::IronDoorEast } else { Block::WoodenDoorEast };
    // Legacy door direction 0..3 is south, west, north, east; the table lists east, south, west, north.
    let index = first as usize + [1, 2, 3, 0][direction(f, 3) as usize] * 2;
    (Block::ALL[index], Block::ALL[index + 1])
}

/// The stronghold selector (148099b60): edges cracked < 0.2, mossy < 0.5, infested < 0.55, else
/// stone bricks; the inside air.
fn select(random: &mut Mt19937) -> Block {
    let roll = random.next_float();
    if roll < 0.2 {
        Block::CrackedStoneBricks
    } else if roll < 0.5 {
        Block::MossyStoneBricks
    } else if roll < 0.55 {
        Block::InfestedStoneBricks
    } else {
        Block::StoneBricks
    }
}

/// `generateBox` with the selector (141f5a6c0) as strongholds call it: air cells too (Java skips
/// them), a draw for every edge cell in or out of the chunk.
pub fn selector_box(f: &Frame, canvas: &mut Canvas, random: &mut Mt19937, (x0, y0, z0): (i32, i32, i32), (x1, y1, z1): (i32, i32, i32)) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            for z in z0..=z1 {
                let edge = y == y0 || y == y1 || x == x0 || x == x1 || z == z0 || z == z1;
                let block = if edge { select(random) } else { Block::Air };
                f.place(canvas, block, x, y, z);
            }
        }
    }
}

pub fn air_box(f: &Frame, canvas: &mut Canvas, lo: (i32, i32, i32), hi: (i32, i32, i32)) {
    f.generate_box(canvas, lo, hi, Block::Air, Block::Air, false);
}

pub fn fill(f: &Frame, canvas: &mut Canvas, lo: (i32, i32, i32), hi: (i32, i32, i32), block: Block) {
    f.generate_box(canvas, lo, hi, block, block, false);
}

/// A doorway three wide and three high from (x, y, z).
pub fn small_door(f: &Frame, canvas: &mut Canvas, kind: Door, (x, y, z): (i32, i32, i32)) {
    let rim = [(x, y), (x, y + 1), (x, y + 2), (x + 1, y + 2), (x + 2, y + 2), (x + 2, y + 1), (x + 2, y)];
    match kind {
        Door::Opening => air_box(f, canvas, (x, y, z), (x + 2, y + 2, z)),
        Door::Grates => {
            f.place(canvas, Block::Air, x + 1, y, z);
            f.place(canvas, Block::Air, x + 1, y + 1, z);
            rim.iter().for_each(|&(rx, ry)| f.place(canvas, Block::IronBars0, rx, ry, z));
        }
        Door::Wood | Door::Iron => {
            rim.iter().for_each(|&(rx, ry)| f.place(canvas, Block::StoneBricks, rx, ry, z));
            let (lower, upper) = door(f, kind == Door::Iron);
            f.place(canvas, lower, x + 1, y, z);
            f.place(canvas, upper, x + 1, y + 1, z);
            if kind == Door::Iron {
                f.place(canvas, button(f, 2), x + 2, y + 1, z + 1);
                f.place(canvas, button(f, 3), x + 2, y + 1, z - 1);
            }
        }
    }
}

/// An end portal frame facing legacy direction `data`.
pub fn portal_frame(data: u8, eye: bool) -> Block {
    let first = if eye { Block::EndPortalFrameSouthEye } else { Block::EndPortalFrameSouth };
    Block::ALL[first as usize + data as usize]
}

pub const NORTH_OR_EAST: [u8; 2] = [NORTH, EAST];

pub fn is_bars(block: Block) -> bool {
    (Block::IronBars0..=Block::IronBars15).contains(&block)
}

/// Iron bars joined to the bars and full blocks beside them (east 1, north 2, south 4, west 8).
pub fn bars_at(canvas: &Canvas, (x, y, z): (i32, i32, i32)) -> Block {
    let joins = |b: Block| is_bars(b) || full_solid(b);
    let bits: usize =
        [(1, 0, 1), (0, -1, 2), (0, 1, 4), (-1, 0, 8)].iter().filter(|(dx, dz, _)| joins(canvas.get(x + dx, y, z + dz))).map(|t| t.2).sum();
    Block::ALL[Block::IronBars0 as usize + bits]
}
