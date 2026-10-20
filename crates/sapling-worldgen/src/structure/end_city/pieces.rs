//! The city's pieces as `createStructureStart` (148313a10) lays them out: Java 1.12's `EndCityPieces`, its four
//! generators (tower 148315580, fat tower 1483167c0, bridge 148317be0, house 148318c80) and `recursiveChildren`
//! (148315150), draw for draw, on the start's blueprint generator.

use crate::random::Mt19937;
use crate::structure::BoundingBox;
use crate::structure::template::{Rotation, Template};

/// A template piece: its template under `endcity/`, its origin, its turn, whether it places air.
#[derive(Debug, Clone)]
pub struct Piece {
    pub name: &'static str,
    pub pos: (i32, i32, i32),
    pub rotation: Rotation,
    /// Overwrite pieces place their air; insert pieces skip it.
    pub overwrite: bool,
    /// The generator call that made it (`u32 >> 1` of a draw; the bridge's ends are -1), for the overlap test.
    kind: i64,
    pub bb: BoundingBox,
}

impl Piece {
    fn new(name: &'static str, pos: (i32, i32, i32), rotation: Rotation, overwrite: bool) -> Self {
        let (sx, sy, sz) = Template::get(&format!("endcity/{name}")).size;
        let (sx, sz) = if matches!(rotation, Rotation::Clockwise90 | Rotation::CounterClockwise90) { (sz, sx) } else { (sx, sz) };
        // Java 1.12's box (141f79610) spans size + 1 in x and z.
        let (x0, z0, x1, z1) = match rotation {
            Rotation::None => (0, 0, sx, sz),
            Rotation::Clockwise90 => (-sx, 0, 0, sz),
            Rotation::Clockwise180 => (-sx, -sz, 0, 0),
            Rotation::CounterClockwise90 => (0, -sz, sx, 0),
        };
        let bb = BoundingBox::new(pos.0 + x0, pos.1, pos.2 + z0, pos.0 + x1, pos.1 + sy - 1, pos.2 + z1);
        Self { name, pos, rotation, overwrite, kind: 0, bb }
    }

    fn moved(mut self, (dx, dy, dz): (i32, i32, i32)) -> Self {
        self.pos = (self.pos.0 + dx, self.pos.1 + dy, self.pos.2 + dz);
        let bb = &mut self.bb;
        (bb.x0, bb.y0, bb.z0, bb.x1, bb.y1, bb.z1) = (bb.x0 + dx, bb.y0 + dy, bb.z0 + dz, bb.x1 + dx, bb.y1 + dy, bb.z1 + dz);
        self
    }
}

const TOWER_BRIDGES: [(i32, (i32, i32, i32)); 4] = [(0, (1, -1, 0)), (1, (6, -1, 1)), (3, (0, -1, 5)), (2, (5, -1, 6))];
const FAT_TOWER_BRIDGES: [(i32, (i32, i32, i32)); 4] = [(0, (4, -1, 0)), (1, (12, -1, 4)), (3, (0, -1, 8)), (2, (8, -1, 12))];

#[derive(Clone, Copy)]
enum Generator {
    Tower,
    FatTower,
    Bridge,
    House,
}

fn turned(rotation: Rotation, by: i32) -> Rotation {
    Rotation::from_index(rotation as i32 + by)
}

/// A new piece at `parent`'s origin moved by `offset` turned as the parent is (`addPiece` 148314da0); returns its index.
fn add(list: &mut Vec<Piece>, parent: &Piece, offset: (i32, i32, i32), name: &'static str, rotation: Rotation, overwrite: bool) -> usize {
    list.push(Piece::new(name, parent.pos, rotation, overwrite).moved(parent.rotation.apply(offset)));
    list.len() - 1
}

struct City<'r> {
    random: &'r mut Mt19937,
    ship: bool,
}

impl City<'_> {
    /// `recursiveChildren`: the generator's pieces join `list` unless one meets a piece of `list` from another call
    /// than `parent`'s.
    fn recurse(&mut self, generator: Generator, depth: i32, parent: &Piece, pos: (i32, i32, i32), list: &mut Vec<Piece>) -> bool {
        if depth > 8 {
            return false;
        }
        let mut local = Vec::new();
        if !self.generate(generator, depth, parent, pos, &mut local) {
            return false;
        }
        let kind = i64::from(self.random.next_int_unbounded());
        local.iter_mut().for_each(|p| p.kind = kind);
        for p in &local {
            if list.iter().find(|q| q.bb.intersects(&p.bb)).is_some_and(|q| q.kind != parent.kind) {
                return false;
            }
        }
        list.extend(local);
        true
    }

    fn generate(&mut self, generator: Generator, depth: i32, parent: &Piece, pos: (i32, i32, i32), list: &mut Vec<Piece>) -> bool {
        match generator {
            Generator::Tower => self.tower(depth, parent, list),
            Generator::FatTower => self.fat_tower(depth, parent, list),
            Generator::Bridge => self.bridge(depth, parent, list),
            Generator::House => self.house(depth, parent, pos, list),
        }
    }

    fn tower(&mut self, depth: i32, parent: &Piece, list: &mut Vec<Piece>) -> bool {
        let r = parent.rotation;
        let a = self.random.next_int(2);
        let b = self.random.next_int(2);
        let mut t = add(list, parent, (3 + a, -3, 3 + b), "tower_base", r, true);
        t = add(list, &list[t].clone(), (0, 7, 0), "tower_piece", r, true);
        let floor_first = self.random.next_int(3) == 0;
        let n = self.random.next_int(3);
        let mut floor = floor_first.then_some(t);
        for j in 0..=n {
            t = add(list, &list[t].clone(), (0, 4, 0), "tower_piece", r, true);
            if j < n && self.random.next_boolean() {
                floor = Some(t);
            }
        }
        if let Some(floor) = floor {
            let floor = list[floor].clone();
            for (turn, offset) in TOWER_BRIDGES {
                if self.random.next_boolean() {
                    let end = add(list, &floor, offset, "bridge_end", turned(r, turn), true);
                    let end = list[end].clone();
                    self.recurse(Generator::Bridge, depth + 1, &end, (0, 0, 0), list);
                }
            }
        } else if depth != 7 {
            let top = list[t].clone();
            return self.recurse(Generator::FatTower, depth + 1, &top, (0, 0, 0), list);
        }
        add(list, &list[t].clone(), (-1, 4, -1), "tower_top", r, true);
        true
    }

    fn fat_tower(&mut self, depth: i32, parent: &Piece, list: &mut Vec<Piece>) -> bool {
        let r = parent.rotation;
        let mut t = add(list, parent, (-3, 4, -3), "fat_tower_base", r, true);
        t = add(list, &list[t].clone(), (0, 4, 0), "fat_tower_middle", r, true);
        let mut i = 0;
        while i < 2 && self.random.next_int(3) != 0 {
            t = add(list, &list[t].clone(), (0, 8, 0), "fat_tower_middle", r, true);
            let middle = list[t].clone();
            for (turn, offset) in FAT_TOWER_BRIDGES {
                if self.random.next_boolean() {
                    let end = add(list, &middle, offset, "bridge_end", turned(r, turn), true);
                    let end = list[end].clone();
                    self.recurse(Generator::Bridge, depth + 1, &end, (0, 0, 0), list);
                }
            }
            i += 1;
        }
        add(list, &list[t].clone(), (-2, 8, -2), "fat_tower_top", r, true);
        true
    }

    fn bridge(&mut self, depth: i32, parent: &Piece, list: &mut Vec<Piece>) -> bool {
        let r = parent.rotation;
        let n = self.random.next_int(4);
        let mut t = add(list, parent, (0, 0, -4), "bridge_piece", r, true);
        list[t].kind = -1;
        let mut j = 0;
        for _ in 0..=n {
            let last = list[t].clone();
            if self.random.next_boolean() {
                t = add(list, &last, (0, j, -4), "bridge_piece", r, true);
                j = 0;
            } else {
                t = if self.random.next_boolean() { add(list, &last, (0, j, -4), "bridge_steep_stairs", r, true) } else { add(list, &last, (0, j, -8), "bridge_gentle_stairs", r, true) };
                j = 4;
            }
        }
        let last = list[t].clone();
        if !self.ship && self.random.next_int(10 - depth) == 0 {
            let a = self.random.next_int(8);
            let b = self.random.next_int(10);
            add(list, &last, (a - 8, j, b - 70), "ship", r, true);
            self.ship = true;
        } else if !self.recurse(Generator::House, depth + 1, &last, (-3, j + 1, -11), list) {
            return false;
        }
        let end = add(list, &last, (4, j, 0), "bridge_end", turned(r, 2), true);
        list[end].kind = -1;
        true
    }

    fn house(&mut self, depth: i32, parent: &Piece, pos: (i32, i32, i32), list: &mut Vec<Piece>) -> bool {
        if depth >= 9 {
            return false;
        }
        let r = parent.rotation;
        let mut t = add(list, parent, pos, "base_floor", r, true);
        let kind = self.random.next_int(3);
        if kind == 0 {
            add(list, &list[t].clone(), (-1, 4, -1), "base_roof", r, true);
            return true;
        }
        t = add(list, &list[t].clone(), (-1, 0, -1), "second_floor_2", r, false);
        if kind == 1 {
            t = add(list, &list[t].clone(), (-1, 8, -1), "second_roof", r, false);
        } else {
            t = add(list, &list[t].clone(), (-1, 4, -1), "third_floor_c", r, false);
            t = add(list, &list[t].clone(), (-1, 8, -1), "third_roof", r, true);
        }
        let top = list[t].clone();
        self.recurse(Generator::Tower, depth + 1, &top, (0, 0, 0), list);
        true
    }
}

/// The pieces of a city whose base floor's origin is `origin`, turned by `rotation`, drawing from `random`.
pub fn generate(random: &mut Mt19937, origin: (i32, i32, i32), rotation: Rotation) -> Vec<Piece> {
    let base = Piece::new("base_floor", origin, rotation, true);
    let mut pieces = vec![base.clone()];
    let mut parent = base;
    for (offset, name, overwrite) in [((-1, 0, -1), "second_floor", false), ((-1, 4, -1), "third_floor", false), ((-1, 8, -1), "third_roof", true)] {
        let t = add(&mut pieces, &parent, offset, name, rotation, overwrite);
        parent = pieces[t].clone();
    }
    let roof = parent;
    City { random, ship: false }.recurse(Generator::Tower, 1, &roof, (0, 0, 0), &mut pieces);
    pieces
}
