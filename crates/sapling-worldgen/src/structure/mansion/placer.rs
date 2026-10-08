//! Java's `MansionPiecePlacer` as BDS runs it (createMansion 1480abea0, outer walls 1480b3800,
//! wall piece 1480b3cc0, roofs 1480b3eb0): the entrance, two floors of outer walls traced around
//! the base grid and one around the third, the roofs, then per floor the corridors with their
//! carpets and every room with its walls, doors and a room template from the floor's collection.

use super::dir::{Dir, compose, rel, turn, up};
use super::grid::{CARPET, DOOR, Grid, MansionGrid, ORIGIN, ROOM_1X1, ROOM_1X2, ROOM_2X2, ROOM_ID, ROOM_TYPE, STAIRS};
use super::{Mirror, Piece, Pos};
use crate::random::Mt19937;
use crate::structure::template::Rotation;

struct Walls {
    pos: Pos,
    rotation: Rotation,
    wall: &'static str,
}

pub struct Placer<'a> {
    pub random: &'a mut Mt19937,
    pub grid: &'a MansionGrid,
    pub pieces: Vec<Piece>,
    start: (i32, i32),
}

impl<'a> Placer<'a> {
    pub fn new(random: &'a mut Mt19937, grid: &'a MansionGrid) -> Self {
        Self { random, grid, pieces: Vec::new(), start: (grid.entrance.0 + 1, grid.entrance.1 + 1) }
    }

    pub fn add(&mut self, name: impl Into<String>, pos: Pos, rotation: Rotation, mirror: Mirror) {
        self.pieces.push(Piece::new(name.into(), pos, rotation, mirror));
    }

    /// The cell (x, y) of a floor's grid, `origin` being that floor's corner.
    pub fn cell(&self, origin: Pos, rotation: Rotation, x: i32, y: i32, east: i32) -> Pos {
        let p = rel(origin, turn(rotation, Dir::South), 8 + (y - self.start.1) * 8);
        rel(p, turn(rotation, Dir::East), east + (x - self.start.0) * 8)
    }

    pub fn create(&mut self, origin: Pos, rotation: Rotation) {
        let mut first = Walls { pos: origin, rotation, wall: "wall_flat" };
        self.add("entrance", rel(first.pos, turn(rotation, Dir::West), 9), rotation, Mirror::None);
        first.pos = rel(first.pos, turn(rotation, Dir::South), 16);
        let mut second = Walls { pos: up(first.pos, 8), rotation, wall: "wall_window" };
        let (base, third) = (&self.grid.base, &self.grid.third);
        let (sx, sy) = self.start;
        let end = (self.grid.entrance.0 + 1, self.grid.entrance.1);
        self.outer_walls(&mut first, base, Dir::South, (sx, sy), end);
        self.outer_walls(&mut second, base, Dir::South, (sx, sy), end);
        let mut top = Walls { pos: up(first.pos, 19), rotation, wall: "wall_window" };
        'found: for y in 0..third.height {
            for x in (0..third.width).rev() {
                if third.is_house(x, y) {
                    top.pos = rel(top.pos, turn(rotation, Dir::South), 8 + (y - sy) * 8);
                    top.pos = rel(top.pos, turn(rotation, Dir::East), (x - sx) * 8);
                    self.wall_piece(&mut top);
                    self.outer_walls(&mut top, third, Dir::South, (x, y), (x, y));
                    break 'found;
                }
            }
        }
        self.roof(up(origin, 16), rotation, base, Some(third));
        self.roof(up(origin, 27), rotation, third, None);
        for floor in 0..3 {
            self.floor(origin, rotation, floor);
        }
    }

    fn outer_walls(&mut self, walls: &mut Walls, grid: &Grid, dir: Dir, start: (i32, i32), end: (i32, i32)) {
        let (mut x, mut y, mut d) = (start.0, start.1, dir);
        loop {
            let (dx, dy) = d.step();
            if !grid.is_house(x + dx, y + dy) {
                self.turn_piece(walls);
                d = d.cw();
                if (x, y) != end || d != dir {
                    self.wall_piece(walls);
                }
            } else if grid.is_house(x + dx + d.ccw().step().0, y + dy + d.ccw().step().1) {
                walls.pos = rel(walls.pos, turn(walls.rotation, Dir::South), 6);
                walls.pos = rel(walls.pos, turn(walls.rotation, Dir::East), 8);
                walls.rotation = compose(walls.rotation, Rotation::CounterClockwise90);
                (x, y) = (x + dx, y + dy);
                d = d.ccw();
            } else {
                (x, y) = (x + dx, y + dy);
                if (x, y) != end || d != dir {
                    self.wall_piece(walls);
                }
            }
            if (x, y) == end && d == dir {
                break;
            }
        }
    }

    fn turn_piece(&mut self, walls: &mut Walls) {
        walls.pos = rel(walls.pos, turn(walls.rotation, Dir::South), -1);
        self.add("wall_corner", walls.pos, walls.rotation, Mirror::None);
        walls.pos = rel(walls.pos, turn(walls.rotation, Dir::South), -7);
        walls.pos = rel(walls.pos, turn(walls.rotation, Dir::West), -6);
        walls.rotation = compose(walls.rotation, Rotation::Clockwise90);
    }

    fn wall_piece(&mut self, walls: &mut Walls) {
        self.add(walls.wall, rel(walls.pos, turn(walls.rotation, Dir::East), 7), walls.rotation, Mirror::None);
        walls.pos = rel(walls.pos, turn(walls.rotation, Dir::South), 8);
    }

    /// One floor's corridors (with carpets toward corridors and carpeted rooms) and rooms.
    fn floor(&mut self, origin: Pos, rotation: Rotation, floor: usize) {
        let pos = up(origin, 8 * floor as i32 + if floor == 2 { 3 } else { 0 });
        let rooms = &self.grid.rooms[floor];
        let grid = if floor == 2 { &self.grid.third } else { &self.grid.base };
        let second = if floor == 0 { "" } else { "_2" };
        let r = |d: Dir| turn(rotation, d);
        for y in 0..grid.height {
            for x in 0..grid.width {
                if grid.get(x, y) != 1 {
                    continue;
                }
                let p = self.cell(pos, rotation, x, y, 0);
                self.add(format!("corridor_floor{second}"), p, rotation, Mirror::None);
                let open = |a: i32, b: i32| grid.get(a, b) == 1 || rooms.get(a, b) & CARPET == CARPET;
                if open(x, y - 1) {
                    self.add("carpet_north", up(rel(p, r(Dir::East), 1), 1), rotation, Mirror::None);
                }
                if open(x + 1, y) {
                    self.add("carpet_east", up(rel(rel(p, r(Dir::South), 1), r(Dir::East), 5), 1), rotation, Mirror::None);
                }
                if open(x, y + 1) {
                    self.add(format!("carpet_south{second}"), rel(rel(p, r(Dir::South), 5), r(Dir::West), 1), rotation, Mirror::None);
                }
                if open(x - 1, y) {
                    self.add(format!("carpet_west{second}"), rel(rel(p, r(Dir::West), 1), r(Dir::North), 1), rotation, Mirror::None);
                }
            }
        }
        for y in 0..grid.height {
            for x in 0..grid.width {
                let stairs = floor == 2 && grid.get(x, y) == 3;
                if grid.get(x, y) != 2 && !stairs {
                    continue;
                }
                let v = rooms.get(x, y);
                self.room(pos, rotation, floor, (x, y), v, stairs && v & CARPET == CARPET);
            }
        }
    }

    fn room(&mut self, pos: Pos, rotation: Rotation, floor: usize, (x, y): (i32, i32), v: i32, stairs: bool) {
        let grid = if floor == 2 { &self.grid.third } else { &self.grid.base };
        let (kind, id) = (v & ROOM_TYPE, v & ROOM_ID);
        let open: Vec<Dir> = if v & DOOR == DOOR { Dir::ALL.into_iter().filter(|d| grid.get(x + d.step().0, y + d.step().1) == 1).collect() } else { Vec::new() };
        let door = if !open.is_empty() {
            Some(Some(open[self.random.next_int(open.len() as i32) as usize]))
        } else if v & ORIGIN == ORIGIN {
            Some(None)
        } else {
            None
        };
        let p = self.cell(pos, rotation, x, y, -1);
        let (wall, door_name) = if floor == 0 { ("indoors_wall", "indoors_door") } else { ("indoors_wall_2", "indoors_door_2") };
        let pick = |d: Dir| if door == Some(Some(d)) { door_name } else { wall };
        let r = |d: Dir| turn(rotation, d);
        if grid.is_house(x - 1, y) && !self.grid.is_room(x - 1, y, floor, id) {
            self.add(pick(Dir::West), p, rotation, Mirror::None);
        }
        if grid.get(x + 1, y) == 1 && !stairs {
            self.add(pick(Dir::East), rel(p, r(Dir::East), 8), rotation, Mirror::None);
        }
        if grid.is_house(x, y + 1) && !self.grid.is_room(x, y + 1, floor, id) {
            let at = rel(rel(p, r(Dir::South), 7), r(Dir::East), 7);
            self.add(pick(Dir::South), at, compose(rotation, Rotation::Clockwise90), Mirror::None);
        }
        if grid.get(x, y - 1) == 1 && !stairs {
            let at = rel(rel(p, r(Dir::North), 1), r(Dir::East), 7);
            self.add(pick(Dir::North), at, compose(rotation, Rotation::Clockwise90), Mirror::None);
        }
        let collection = super::rooms::Collection::of(floor);
        match (kind, door) {
            (ROOM_1X1, door) => self.room_1x1(p, rotation, door.flatten(), collection),
            (ROOM_1X2, Some(door)) => {
                let side = self.grid.room_1x2_dir(x, y, floor, id);
                self.room_1x2(p, rotation, side, door, collection, v & STAIRS == STAIRS);
            }
            (ROOM_2X2, Some(Some(door))) => {
                let mut side = door.cw();
                if !self.grid.is_room(x + side.step().0, y + side.step().1, floor, id) {
                    side = side.opposite();
                }
                self.room_2x2(p, rotation, side, door, collection);
            }
            (ROOM_2X2, Some(None)) => self.room_2x2_secret(p, rotation, collection),
            _ => {}
        }
    }
}
