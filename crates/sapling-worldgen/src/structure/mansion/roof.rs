//! Java's `createRoof` (BDS 1480b3eb0): a roof over every house cell of a floor not covered by
//! the floor above, its fronts on open sides; where an upper floor stands, small walls and wall
//! corners around it; then the roof corners and inner corners.

use super::dir::{Dir, compose, rel, turn};
use super::grid::Grid;
use super::placer::Placer;
use super::{Mirror, Pos};
use crate::structure::template::Rotation;

impl Placer<'_> {
    pub fn roof(&mut self, origin: Pos, rotation: Rotation, grid: &Grid, above: Option<&Grid>) {
        let r = |d: Dir| turn(rotation, d);
        let cw = |n: i32| compose(rotation, Rotation::from_index(n));
        let house = |x: i32, y: i32| grid.is_house(x, y);
        let covered = |x: i32, y: i32| above.is_some_and(|a| a.is_house(x, y));
        for y in 0..grid.height {
            for x in 0..grid.width {
                let p = self.cell(origin, rotation, x, y, 0);
                if !house(x, y) || covered(x, y) {
                    continue;
                }
                self.add("roof", (p.0, p.1 + 3, p.2), rotation, Mirror::None);
                if !house(x + 1, y) {
                    self.add("roof_front", rel(p, r(Dir::East), 6), rotation, Mirror::None);
                }
                if !house(x - 1, y) {
                    self.add("roof_front", rel(rel(p, r(Dir::East), 0), r(Dir::South), 7), cw(2), Mirror::None);
                }
                if !house(x, y - 1) {
                    self.add("roof_front", rel(p, r(Dir::West), 1), cw(3), Mirror::None);
                }
                if !house(x, y + 1) {
                    self.add("roof_front", rel(rel(p, r(Dir::East), 6), r(Dir::South), 6), cw(1), Mirror::None);
                }
            }
        }
        if above.is_some() {
            for y in 0..grid.height {
                for x in 0..grid.width {
                    if house(x, y) && covered(x, y) {
                        self.small_walls(self.cell(origin, rotation, x, y, 0), rotation, grid, (x, y));
                    }
                }
            }
        }
        for y in 0..grid.height {
            for x in 0..grid.width {
                let p = self.cell(origin, rotation, x, y, 0);
                if !house(x, y) || covered(x, y) {
                    continue;
                }
                if !house(x + 1, y) {
                    let e = rel(p, r(Dir::East), 6);
                    if !house(x, y + 1) {
                        self.add("roof_corner", rel(e, r(Dir::South), 6), rotation, Mirror::None);
                    } else if house(x + 1, y + 1) {
                        self.add("roof_inner_corner", rel(e, r(Dir::South), 5), rotation, Mirror::None);
                    }
                    if !house(x, y - 1) {
                        self.add("roof_corner", e, cw(3), Mirror::None);
                    } else if house(x + 1, y - 1) {
                        self.add("roof_inner_corner", rel(rel(p, r(Dir::East), 9), r(Dir::North), 2), cw(1), Mirror::None);
                    }
                }
                if !house(x - 1, y) {
                    if !house(x, y + 1) {
                        self.add("roof_corner", rel(p, r(Dir::South), 6), cw(1), Mirror::None);
                    } else if house(x - 1, y + 1) {
                        self.add("roof_inner_corner", rel(rel(p, r(Dir::South), 8), r(Dir::West), 3), cw(3), Mirror::None);
                    }
                    if !house(x, y - 1) {
                        self.add("roof_corner", p, cw(2), Mirror::None);
                    } else if house(x - 1, y - 1) {
                        self.add("roof_inner_corner", rel(p, r(Dir::South), 1), cw(2), Mirror::None);
                    }
                }
            }
        }
    }

    fn small_walls(&mut self, p: Pos, rotation: Rotation, grid: &Grid, (x, y): (i32, i32)) {
        let r = |d: Dir| turn(rotation, d);
        let cw = |n: i32| compose(rotation, Rotation::from_index(n));
        let house = |a: i32, b: i32| grid.is_house(a, b);
        if !house(x + 1, y) {
            self.add("small_wall", rel(p, r(Dir::East), 7), rotation, Mirror::None);
        }
        if !house(x - 1, y) {
            self.add("small_wall", rel(rel(p, r(Dir::West), 1), r(Dir::South), 6), cw(2), Mirror::None);
        }
        if !house(x, y - 1) {
            self.add("small_wall", rel(rel(p, r(Dir::West), 0), r(Dir::North), 1), cw(3), Mirror::None);
        }
        if !house(x, y + 1) {
            self.add("small_wall", rel(rel(p, r(Dir::East), 6), r(Dir::South), 7), cw(1), Mirror::None);
        }
        if !house(x + 1, y) {
            if !house(x, y - 1) {
                self.add("small_wall_corner", rel(rel(p, r(Dir::East), 7), r(Dir::North), 2), rotation, Mirror::None);
            }
            if !house(x, y + 1) {
                self.add("small_wall_corner", rel(rel(p, r(Dir::East), 8), r(Dir::South), 7), cw(1), Mirror::None);
            }
        }
        if !house(x - 1, y) {
            if !house(x, y - 1) {
                self.add("small_wall_corner", rel(rel(p, r(Dir::West), 2), r(Dir::North), 1), cw(3), Mirror::None);
            }
            if !house(x, y + 1) {
                self.add("small_wall_corner", rel(rel(p, r(Dir::West), 1), r(Dir::South), 8), cw(2), Mirror::None);
            }
        }
    }
}
