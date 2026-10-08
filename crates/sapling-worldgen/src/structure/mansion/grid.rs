//! The mansion's floor plans (Java's `MansionGrid`, BDS's start 1480a8c20 with corridors 1480b72e0,
//! edge cleaning 1480b7780 and rooms 1480b7a60): an 11x11 base grid grown into corridors from the
//! entrance, rooms found on the first two floors, and a third floor grown from a 1x2 room's
//! stairs. Cells: 0 clear, 1 corridor, 2 room, 3 start room, 5 blocked. Room cells carry a type
//! (`ROOM_1X1` ...), an id, and flags (`DOOR`, `ORIGIN`, `STAIRS`, `CARPET`).

use super::dir::Dir;
use crate::random::Mt19937;

pub const ROOM_1X1: i32 = 0x10000;
pub const ROOM_1X2: i32 = 0x20000;
pub const ROOM_2X2: i32 = 0x40000;
pub const ROOM_TYPE: i32 = 0xF0000;
pub const ROOM_ID: i32 = 0xFFFF;
pub const ORIGIN: i32 = 0x100000;
pub const DOOR: i32 = 0x200000;
pub const STAIRS: i32 = 0x400000;
pub const CARPET: i32 = 0x800000;

#[derive(Clone)]
pub struct Grid {
    pub width: i32,
    pub height: i32,
    cells: Vec<i32>,
    outside: i32,
}

impl Grid {
    fn new(width: i32, height: i32, outside: i32) -> Self {
        Self { width, height, cells: vec![0; (width * height) as usize], outside }
    }

    fn inside(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width && y >= 0 && y < self.height
    }

    pub fn get(&self, x: i32, y: i32) -> i32 {
        if self.inside(x, y) { self.cells[(x * self.height + y) as usize] } else { self.outside }
    }

    pub fn set(&mut self, x: i32, y: i32, value: i32) {
        if self.inside(x, y) {
            self.cells[(x * self.height + y) as usize] = value;
        }
    }

    fn fill(&mut self, (x0, y0): (i32, i32), (x1, y1): (i32, i32), value: i32) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.set(x, y, value);
            }
        }
    }

    fn set_if(&mut self, x: i32, y: i32, expected: i32, value: i32) {
        if self.get(x, y) == expected {
            self.set(x, y, value);
        }
    }

    fn edges_to(&self, x: i32, y: i32, value: i32) -> bool {
        self.get(x - 1, y) == value || self.get(x + 1, y) == value || self.get(x, y + 1) == value || self.get(x, y - 1) == value
    }

    /// Corridors, rooms and the start room count as the house.
    pub fn is_house(&self, x: i32, y: i32) -> bool {
        matches!(self.get(x, y), 1..=4)
    }
}

pub struct MansionGrid {
    pub base: Grid,
    pub third: Grid,
    pub rooms: [Grid; 3],
    pub entrance: (i32, i32),
}

impl MansionGrid {
    pub fn new(random: &mut Mt19937) -> Self {
        let (ex, ey) = (7, 4);
        let mut base = Grid::new(11, 11, 5);
        base.fill((ex, ey), (ex + 1, ey + 1), 3);
        base.fill((ex - 1, ey), (ex - 1, ey + 1), 2);
        base.fill((ex + 2, ey - 2), (ex + 3, ey + 3), 5);
        base.fill((ex + 1, ey - 2), (ex + 1, ey - 1), 1);
        base.fill((ex + 1, ey + 2), (ex + 1, ey + 3), 1);
        base.set(ex - 1, ey - 1, 1);
        base.set(ex - 1, ey + 2, 1);
        base.fill((0, 0), (11, 1), 5);
        base.fill((0, 9), (11, 11), 5);
        corridor(random, &mut base, ex, ey - 2, Dir::West, 6);
        corridor(random, &mut base, ex, ey + 3, Dir::West, 6);
        corridor(random, &mut base, ex - 2, ey - 1, Dir::West, 3);
        corridor(random, &mut base, ex - 2, ey + 2, Dir::West, 3);
        while clean_edges(&mut base) {}
        let mut rooms = [Grid::new(11, 11, 5), Grid::new(11, 11, 5), Grid::new(11, 11, 5)];
        identify_rooms(random, &base, &mut rooms[0]);
        identify_rooms(random, &base, &mut rooms[1]);
        rooms[0].fill((ex + 1, ey), (ex + 1, ey + 1), CARPET);
        rooms[1].fill((ex + 1, ey), (ex + 1, ey + 1), CARPET);
        let mut grid = Self { third: Grid::new(base.width, base.height, 5), base, rooms, entrance: (ex, ey) };
        grid.setup_third_floor(random);
        let third = grid.third.clone();
        identify_rooms(random, &third, &mut grid.rooms[2]);
        grid
    }

    pub fn is_room(&self, x: i32, y: i32, floor: usize, id: i32) -> bool {
        self.rooms[floor].get(x, y) & ROOM_ID == id
    }

    /// The side of a 1x2 room's cell its other cell lies on.
    pub fn room_1x2_dir(&self, x: i32, y: i32, floor: usize, id: i32) -> Option<Dir> {
        Dir::ALL.into_iter().find(|d| self.is_room(x + d.step().0, y + d.step().1, floor, id))
    }

    /// A random 1x2 second-floor room with a door gets the stairs up; the third floor grows from
    /// its other cell.
    fn setup_third_floor(&mut self, random: &mut Mt19937) {
        let second = &self.rooms[1];
        let mut found = Vec::new();
        for y in 0..self.third.height {
            for x in 0..self.third.width {
                let v = second.get(x, y);
                if v & ROOM_TYPE == ROOM_1X2 && v & DOOR == DOOR {
                    found.push((x, y));
                }
            }
        }
        let blocked = |third: &mut Grid| {
            let (w, h) = (third.width, third.height);
            third.fill((0, 0), (w, h), 5);
        };
        if found.is_empty() {
            blocked(&mut self.third);
            return;
        }
        let (sx, sy) = found[random.next_int(found.len() as i32) as usize];
        let v = self.rooms[1].get(sx, sy);
        self.rooms[1].set(sx, sy, v | STAIRS);
        let d = self.room_1x2_dir(sx, sy, 1, v & ROOM_ID).expect("1x2 room's other cell");
        let (tx, ty) = (sx + d.step().0, sy + d.step().1);
        for y in 0..self.third.height {
            for x in 0..self.third.width {
                if !self.base.is_house(x, y) {
                    self.third.set(x, y, 5);
                } else if (x, y) == (sx, sy) {
                    self.third.set(x, y, 3);
                } else if (x, y) == (tx, ty) {
                    self.third.set(x, y, 3);
                    self.rooms[2].set(x, y, CARPET);
                }
            }
        }
        let open: Vec<Dir> = Dir::ALL.into_iter().filter(|d| self.third.get(tx + d.step().0, ty + d.step().1) == 0).collect();
        if open.is_empty() {
            blocked(&mut self.third);
            self.rooms[1].set(sx, sy, v);
            return;
        }
        let d = open[random.next_int(open.len() as i32) as usize];
        corridor(random, &mut self.third, tx + d.step().0, ty + d.step().1, d, 4);
        while clean_edges(&mut self.third) {}
    }
}

/// 1480b72e0: a corridor cell, up to eight tries to branch on (never back, east only half the
/// time), then rooms on its sides and ahead.
fn corridor(random: &mut Mt19937, grid: &mut Grid, x: i32, y: i32, dir: Dir, depth: i32) {
    if depth <= 0 {
        return;
    }
    let (dx, dy) = dir.step();
    grid.set(x, y, 1);
    grid.set_if(x + dx, y + dy, 0, 1);
    for _ in 0..8 {
        let d = Dir::from_2d(random.next_int(4));
        if d == dir.opposite() || d == Dir::East && random.next_boolean() {
            continue;
        }
        let (nx, ny) = (x + dx, y + dy);
        let (sx, sy) = d.step();
        if grid.get(nx + sx, ny + sy) == 0 && grid.get(nx + sx * 2, ny + sy * 2) == 0 {
            corridor(random, grid, nx + sx, ny + sy, d, depth - 1);
            break;
        }
    }
    let ((cx, cy), (wx, wy)) = (dir.cw().step(), dir.ccw().step());
    grid.set_if(x + cx, y + cy, 0, 2);
    grid.set_if(x + wx, y + wy, 0, 2);
    grid.set_if(x + dx + cx, y + dy + cy, 0, 2);
    grid.set_if(x + dx + wx, y + dy + wy, 0, 2);
    grid.set_if(x + dx * 2, y + dy * 2, 0, 2);
    grid.set_if(x + cx * 2, y + cy * 2, 0, 2);
    grid.set_if(x + wx * 2, y + wy * 2, 0, 2);
}

/// Clear cells walled in on three sides, or on two with at most one house corner, become rooms.
fn clean_edges(grid: &mut Grid) -> bool {
    let mut changed = false;
    for y in 0..grid.height {
        for x in 0..grid.width {
            if grid.get(x, y) != 0 {
                continue;
            }
            let sides = [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().filter(|(a, b)| grid.is_house(x + a, y + b)).count();
            let corners = [(1, 1), (-1, 1), (1, -1), (-1, -1)].iter().filter(|(a, b)| grid.is_house(x + a, y + b)).count();
            if sides >= 3 || sides == 2 && corners <= 1 {
                grid.set(x, y, 2);
                changed = true;
            }
        }
    }
    changed
}

/// 1480b7a60: room cells in an order shuffled BDS's way (`nextInt(i)` for i from the last index
/// down, never swapping a cell with itself), each unclaimed one growing into a 2x2, 1x2 or 1x1
/// room with the next id, its door cell drawn among its corners touching a corridor.
fn identify_rooms(random: &mut Mt19937, grid: &Grid, rooms: &mut Grid) {
    let mut cells = Vec::new();
    for y in 0..grid.height {
        for x in 0..grid.width {
            if grid.get(x, y) == 2 {
                cells.push((x, y));
            }
        }
    }
    for i in (1..cells.len()).rev() {
        let j = random.next_int(i as i32) as usize;
        cells.swap(i, j);
    }
    let mut id = 10;
    for (x, y) in cells {
        if rooms.get(x, y) != 0 {
            continue;
        }
        let free = |a: i32, b: i32| rooms.get(a, b) == 0 && grid.get(a, b) == 2;
        let (mut x0, mut x1, mut y0, mut y1) = (x, x, y, y);
        let kind = if free(x + 1, y) && free(x, y + 1) && free(x + 1, y + 1) {
            (x1, y1) = (x + 1, y + 1);
            ROOM_2X2
        } else if free(x - 1, y) && free(x, y + 1) && free(x - 1, y + 1) {
            (x0, y1) = (x - 1, y + 1);
            ROOM_2X2
        } else if free(x - 1, y) && free(x, y - 1) && free(x - 1, y - 1) {
            (x0, y0) = (x - 1, y - 1);
            ROOM_2X2
        } else if free(x + 1, y) {
            x1 = x + 1;
            ROOM_1X2
        } else if free(x, y + 1) {
            y1 = y + 1;
            ROOM_1X2
        } else if free(x - 1, y) {
            x0 = x - 1;
            ROOM_1X2
        } else if free(x, y - 1) {
            y0 = y - 1;
            ROOM_1X2
        } else {
            ROOM_1X1
        };
        let mut dx = if random.next_boolean() { x0 } else { x1 };
        let mut dy = if random.next_boolean() { y0 } else { y1 };
        let flip = |v: i32, a: i32, b: i32| if v == a { b } else { a };
        let mut door = DOOR;
        if !grid.edges_to(dx, dy, 1) {
            dx = flip(dx, x0, x1);
            dy = flip(dy, y0, y1);
            if !grid.edges_to(dx, dy, 1) {
                dy = flip(dy, y0, y1);
                if !grid.edges_to(dx, dy, 1) {
                    dx = flip(dx, x0, x1);
                    dy = flip(dy, y0, y1);
                    if !grid.edges_to(dx, dy, 1) {
                        door = 0;
                        (dx, dy) = (x0, y0);
                    }
                }
            }
        }
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                let flags = if (cx, cy) == (dx, dy) { ORIGIN | door } else { 0 };
                rooms.set(cx, cy, flags | kind | id);
            }
        }
        id += 1;
    }
}
