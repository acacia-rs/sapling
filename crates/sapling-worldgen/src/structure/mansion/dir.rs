//! Horizontal directions and the position and rotation arithmetic the mansion code shares.

use super::Pos;
use crate::structure::template::Rotation;

/// Java's horizontal directions in `Direction.Plane.HORIZONTAL` order (BDS iterates the same).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    North,
    East,
    South,
    West,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::North, Dir::East, Dir::South, Dir::West];

    pub fn step(self) -> (i32, i32) {
        match self {
            Dir::North => (0, -1),
            Dir::East => (1, 0),
            Dir::South => (0, 1),
            Dir::West => (-1, 0),
        }
    }

    /// `n` quarter turns clockwise.
    pub fn turn(self, n: i32) -> Dir {
        Dir::ALL[(self as i32 + n).rem_euclid(4) as usize]
    }

    pub fn cw(self) -> Dir {
        self.turn(1)
    }

    pub fn ccw(self) -> Dir {
        self.turn(-1)
    }

    pub fn opposite(self) -> Dir {
        self.turn(2)
    }

    /// BDS's `Direction::from2DDataValue` table (14a7fe150): north, south, west, east.
    pub fn from_2d(index: i32) -> Dir {
        [Dir::North, Dir::South, Dir::West, Dir::East][index as usize]
    }
}

/// `d` as a piece turned by `rotation` sees it.
pub fn turn(rotation: Rotation, d: Dir) -> Dir {
    d.turn(rotation as i32)
}

pub fn compose(a: Rotation, b: Rotation) -> Rotation {
    Rotation::from_index(a as i32 + b as i32)
}

pub fn rel((x, y, z): Pos, d: Dir, n: i32) -> Pos {
    let (dx, dz) = d.step();
    (x + dx * n, y, z + dz * n)
}

pub fn up((x, y, z): Pos, n: i32) -> Pos {
    (x, y + n, z)
}
