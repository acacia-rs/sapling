//! Room templates (Java's `addRoom1x1` ... `addRoom2x2Secret` and the floor room collections, as
//! BDS draws them): the template by the floor and the room's shape, turned and mirrored by where
//! its door and its other cells lie.

use super::dir::{Dir, compose, rel, turn};
use super::placer::Placer;
use super::{Mirror, Pos};
use crate::structure::template::Rotation;

#[derive(Clone, Copy)]
pub enum Collection {
    First,
    /// The second and third floors.
    Upper,
}

impl Collection {
    pub fn of(floor: usize) -> Self {
        if floor == 0 { Collection::First } else { Collection::Upper }
    }
}

impl Placer<'_> {
    fn draw(&mut self, prefix: &str, bound: i32) -> String {
        format!("{prefix}{}", self.random.next_int(bound) + 1)
    }

    fn one_by_one(&mut self, c: Collection) -> String {
        match c {
            Collection::First => self.draw("1x1_a", 5),
            Collection::Upper => self.draw("1x1_b", 5),
        }
    }

    fn side_entrance(&mut self, c: Collection, stairs: bool) -> String {
        match c {
            Collection::First => self.draw("1x2_a", 9),
            Collection::Upper if stairs => "1x2_c_stairs".to_string(),
            Collection::Upper => self.draw("1x2_c", 4),
        }
    }

    fn front_entrance(&mut self, c: Collection, stairs: bool) -> String {
        match c {
            Collection::First => self.draw("1x2_b", 5),
            Collection::Upper if stairs => "1x2_d_stairs".to_string(),
            Collection::Upper => self.draw("1x2_d", 5),
        }
    }

    fn secret_1x2(&mut self, c: Collection) -> String {
        match c {
            Collection::First => self.draw("1x2_s", 2),
            Collection::Upper => self.draw("1x2_se", 1),
        }
    }

    fn two_by_two(&mut self, c: Collection) -> String {
        match c {
            Collection::First => self.draw("2x2_a", 4),
            Collection::Upper => self.draw("2x2_b", 5),
        }
    }

    /// A 1x1 room turned toward its door; with no side door, a secret room.
    pub fn room_1x1(&mut self, p: Pos, rotation: Rotation, door: Option<Dir>, c: Collection) {
        let mut name = self.one_by_one(c);
        let turned = match door {
            Some(Dir::East) => Rotation::None,
            Some(Dir::North) => Rotation::CounterClockwise90,
            Some(Dir::West) => Rotation::Clockwise180,
            Some(Dir::South) => Rotation::Clockwise90,
            None => {
                name = self.draw("1x1_as", 4);
                Rotation::None
            }
        };
        // Java's getZeroPositionWithTransform((1, 0, 0), no mirror, turned, 7, 7), then turned by the house.
        let (zx, zz) = match turned {
            Rotation::None => (1, 0),
            Rotation::Clockwise90 => (7, 0),
            Rotation::Clockwise180 => (7, 6),
            Rotation::CounterClockwise90 => (1, 6),
        };
        let (ox, _, oz) = rotation.apply((zx, 0, zz));
        self.add(name, (p.0 + ox, p.1, p.2 + oz), compose(turned, rotation), Mirror::None);
    }

    /// A 1x2 room by the side its other cell lies on (`side`) and its door (`None`: secret).
    pub fn room_1x2(&mut self, p: Pos, rotation: Rotation, side: Option<Dir>, door: Option<Dir>, c: Collection, stairs: bool) {
        use Dir::*;
        let r = |d: Dir| turn(rotation, d);
        let at = |east: i32, south: i32| rel(rel(p, r(East), east), r(South), south);
        let cw = |n: i32| compose(rotation, Rotation::from_index(n));
        let (kind, pos, rot, mirror) = match (door, side) {
            (Some(East), Some(South)) => (0, at(1, 0), rotation, Mirror::None),
            (Some(East), Some(North)) => (0, at(1, 6), rotation, Mirror::LeftRight),
            (Some(West), Some(North)) => (0, at(7, 6), cw(2), Mirror::None),
            (Some(West), Some(South)) => (0, at(7, 0), rotation, Mirror::FrontBack),
            (Some(South), Some(East)) => (0, at(1, 0), cw(1), Mirror::LeftRight),
            (Some(South), Some(West)) => (0, at(7, 0), cw(1), Mirror::None),
            (Some(North), Some(West)) => (0, at(7, 6), cw(1), Mirror::FrontBack),
            (Some(North), Some(East)) => (0, at(1, 6), cw(3), Mirror::None),
            (Some(South), Some(North)) => (1, at(1, -8), rotation, Mirror::None),
            (Some(North), Some(South)) => (1, at(7, 14), cw(2), Mirror::None),
            (Some(West), Some(East)) => (1, at(15, 0), cw(1), Mirror::None),
            (Some(East), Some(West)) => (1, at(-7, 6), cw(3), Mirror::None),
            (None, Some(East)) => (2, at(15, 0), cw(1), Mirror::None),
            (None, Some(South)) => (2, at(1, 0), rotation, Mirror::None),
            _ => return,
        };
        let name = match kind {
            0 => self.side_entrance(c, stairs),
            1 => self.front_entrance(c, stairs),
            _ => self.secret_1x2(c),
        };
        self.add(name, pos, rot, mirror);
    }

    /// A 2x2 room by the side its other column lies on and its door.
    pub fn room_2x2(&mut self, p: Pos, rotation: Rotation, side: Dir, door: Dir, c: Collection) {
        use Dir::*;
        let cw = |n: i32| compose(rotation, Rotation::from_index(n));
        let (east, south, rot, mirror) = match (door, side) {
            (East, South) => (-7, 0, rotation, Mirror::None),
            (East, North) => (-7, 6, rotation, Mirror::LeftRight),
            (North, East) => (1, 14, cw(3), Mirror::None),
            (North, West) => (7, 14, cw(3), Mirror::LeftRight),
            (South, West) => (7, -8, cw(1), Mirror::None),
            (South, East) => (1, -8, cw(1), Mirror::LeftRight),
            (West, North) => (15, 6, cw(2), Mirror::None),
            (West, South) => (15, 0, rotation, Mirror::FrontBack),
            _ => (0, 0, rotation, Mirror::None),
        };
        let pos = rel(rel(p, turn(rotation, East), east), turn(rotation, South), south);
        let name = self.two_by_two(c);
        self.add(name, pos, rot, mirror);
    }

    pub fn room_2x2_secret(&mut self, p: Pos, rotation: Rotation, _c: Collection) {
        self.add("2x2_s1", rel(p, turn(rotation, Dir::East), 1), rotation, Mirror::None);
    }
}
