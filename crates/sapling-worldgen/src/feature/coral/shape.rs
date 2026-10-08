//! `CoralFeature::_placeCoral` (14828b9e0) and its helpers `_starFormation` (1482914f0) and
//! `_buildPlantArm` (148291d20). Every builder stops at the first cell it may not overwrite.

use super::{OPPOSITE, Reef, fits, is_water, solid, step, up};

/// A branch's second face by its first (north, south, west, east) and `nextInt(3)` (0x14a925a60..).
const SECOND_FACE: [[usize; 3]; 4] = [[5, 3, 4], [5, 2, 4], [5, 3, 2], [2, 3, 4]];

/// A branch's third face by `nextInt(2)`: one of the two faces the first two left (byte pairs
/// 0x14a925a6c..0x14a925a77, in BDS's order).
fn third_face(first: usize, second: usize) -> [usize; 2] {
    match (first.min(second), first.max(second)) {
        (2, 5) => [3, 4],
        (2, 3) => [5, 4],
        (2, 4) => [3, 5],
        (3, 5) => [2, 4],
        (4, 5) => [3, 2],
        _ => [2, 5],
    }
}

/// `_buildPlantArm`'s turn: counter-clockwise when `clockwise` is false (0x14a925e88, 0x14a7efe20).
fn turn(face: usize, clockwise: bool) -> usize {
    let table: &[usize] = if clockwise { &[5, 4, 2, 3] } else { &[4, 5, 3] };
    table.get(face - 2).copied().unwrap_or(2)
}

impl Reef<'_, '_> {
    /// The three shapes fall through like a switch without breaks: shape 0 builds all three, shape
    /// 1 the last two; a shape that stops goes on to the next.
    pub(super) fn place_coral(&mut self, at: [i32; 3], color: usize) {
        let shape = self.random.next_int(3);
        if shape == 0 {
            self.branching(at, color);
        }
        if shape <= 1 {
            self.mound(at, color);
        }
        self.pillar(at, color);
    }

    fn fits(&self, p: [i32; 3]) -> bool {
        fits(self.view, p)
    }

    /// A side decoration: the face drawn before the chance.
    fn side(&mut self, p: [i32; 3]) {
        let face = self.random.next_int(4) as usize + 2;
        if self.random.next_float() < 0.4 {
            self.sides.push((p, face));
        }
    }

    /// `_starFormation`: the centre, then each of +x, -x, +z, -z `reach` away by chance, on solid ground.
    fn star(&mut self, p: [i32; 3], color: usize, chance: f32, reach: i32) {
        if !self.try_set(p, color) {
            return;
        }
        for [dx, dz] in [[reach, 0], [-reach, 0], [0, reach], [0, -reach]] {
            if self.random.next_float() < chance {
                let q = [p[0] + dx, p[1], p[2] + dz];
                if !super::can_overwrite(self.view, q, color) || !solid(self.view, up(q, -1)) {
                    return;
                }
                self.set(q, super::CORAL_BLOCKS[color]);
            }
        }
    }

    /// The diagonal tries of the first tall layer (the (-j, +j) corner twice: BDS's own repeat).
    fn diagonals(&mut self, [x, y, z]: [i32; 3], color: usize, j: i32) {
        for [dx, dz] in [[j, j], [-j, j], [-j, j], [j, -j]] {
            if self.random.next_float() < 0.25 {
                let q = [x + dx, y, z + dz];
                if !super::can_overwrite(self.view, q, color) || !solid(self.view, up(q, -1)) {
                    return;
                }
                self.set(q, super::CORAL_BLOCKS[color]);
            }
        }
    }

    /// Shape 0: a stack of star layers (every layer's centre written at the base: BDS's own), a
    /// stem and up to three plant arms.
    fn branching(&mut self, at: [i32; 3], color: usize) {
        let [x, y, z] = at;
        let h = self.random.next_int(2) + 1;
        for i in 0..=h {
            let k = 2 * h + 1 - 2 * i;
            if !self.fits([x, y + i + 1, z]) || !self.try_set(at, color) {
                return;
            }
            if k < 2 {
                continue;
            }
            let chance = |j: i32, i: i32| (j as f32 * 0.1 + 0.5) - i as f32 * 0.15;
            if h == 2 && i == 0 {
                for j in 1..=k / 2 {
                    self.star(at, color, chance(j, 0), 0);
                    self.diagonals(at, color, j);
                }
            } else {
                for j in 1..=k / 2 {
                    self.star([x, y + i, z], color, chance(j, i), i);
                }
            }
        }
        let base = [x, y + h, z];
        if !self.fits(up(base, 1)) {
            return;
        }
        let first = self.random.next_int(4) as usize + 2;
        let second = SECOND_FACE[first - 2][self.random.next_int(3) as usize];
        let third = third_face(first, second)[self.random.next_int(2) as usize];
        if !self.try_set(base, color) {
            return;
        }
        self.arm(step(base, first), color, first, false);
        let extra = self.random.next_int(2);
        for (u, face) in (1..=extra + 1).zip([second, third]) {
            let p = up(base, u);
            if !self.try_set(p, color) {
                return;
            }
            self.arm(step(p, face), color, face, u == extra + 1);
        }
    }

    /// Shape 1: a cornerless square floor on solid ground, a cross and a stem.
    fn mound(&mut self, [x, y, z]: [i32; 3], color: usize) {
        let n = self.random.next_int(2);
        let (side, max) = (n + 1, 2 * n + 2);
        let (cx, cz) = (x - side, z - side);
        for a in 0..=max {
            for b in 0..=max {
                if (a == 0 || a == max) && (b == 0 || b == max) {
                    continue;
                }
                let p = [cx + a, y, cz + b];
                if self.random.next_int(5) != 0 && solid(self.view, up(p, -1)) && !self.try_set(p, color) {
                    return;
                }
            }
        }
        // BDS writes the corner it left out, not the centre.
        self.try_set([cx, y, cz], color);
        let q = [x, y + 1, z];
        if !self.fits(up(q, 1)) {
            return;
        }
        if !self.try_set(q, color) {
            return;
        }
        for (dx, dz, bound) in [(1, 0, 2), (-1, 0, 3), (0, 1, 4), (0, -1, 3)] {
            if self.random.next_int(bound) != 0 && !self.try_set([x + dx, y + 1, z + dz], color) {
                return;
            }
        }
        self.star([x, y + 2, z], color, 0.33, 1);
        self.star([x, y + 3, z], color, 0.7, 1);
        let top = [x, y + 4, z];
        let t = self.random.next_int(2);
        for v in 0..=t {
            if !self.fits(up(top, v + 1)) || !self.try_set(up(top, v), color) {
                return;
            }
            self.side(top);
        }
        if self.random.next_float() < 0.4 {
            self.tops.push(up(top, t + 1));
        }
    }

    /// Shape 2: a 3x3 floor and a short stem (6 in 7), else a tall stem with three ladders.
    fn pillar(&mut self, at: [i32; 3], color: usize) {
        if self.random.next_int(7) == 0 {
            self.ladders(at, color);
            return;
        }
        let [x, y, z] = at;
        for a in 0..3 {
            for b in 0..3 {
                let p = [x - 1 + a, y, z - 1 + b];
                if self.random.next_int(3) != 0 && !is_water(super::block(self.view, up(p, -1))) && !self.try_set(p, color) {
                    return;
                }
            }
        }
        let q = [x, y + 1, z];
        if !self.fits(up(q, 1)) || !self.try_set(q, color) {
            return;
        }
        for [dx, dz] in [[1, 0], [-1, 0], [0, 1], [0, -1]] {
            let p = [x + dx, y + 1, z + dz];
            if self.random.next_int(4) != 0 && !is_water(super::block(self.view, up(p, -1))) {
                if !self.try_set(p, color) {
                    return;
                }
                if self.random.next_float() < 0.2 {
                    self.tops.push(up(p, 1));
                }
            }
        }
        if self.fits([x, y + 3, z]) {
            if !self.try_set([x, y + 2, z], color) {
                return;
            }
            if self.random.next_float() < 0.2 {
                self.tops.push([x, y + 3, z]);
            }
        }
        if self.random.next_float() >= 0.25 || !self.fits([x, y + 4, z]) || !self.try_set([x, y + 3, z], color) {
            return;
        }
        if self.random.next_float() < 0.1 {
            self.tops.push([x, y + 4, z]);
        }
    }

    /// Shape 2's tall case: a stem, a cell or two to each side, then three runs along a face.
    fn ladders(&mut self, [x, y, z]: [i32; 3], color: usize) {
        let a = self.random.next_int(4) as usize;
        let side = [5, 4, 2, 3][a];
        let other = OPPOSITE[side];
        let wide = self.random.next_int(2) != 0;
        let height = self.random.next_int(3);
        if !self.fits([x, y + height + 3, z]) {
            return;
        }
        for v in 0..=height {
            if !self.try_set([x, y + v, z], color) {
                return;
            }
        }
        let centre = [x, y + height + 1, z];
        let (left, right) = (step(centre, side), step(centre, other));
        // Written in this order; the last two are where the outer runs start.
        let cells = if !wide {
            vec![left, right]
        } else if !self.random.next_boolean() {
            vec![right, left, step(right, other)]
        } else {
            vec![left, step(left, side), right]
        };
        for &p in &cells {
            if !self.try_set(p, color) {
                return;
            }
        }
        let (l, r) = (cells[cells.len() - 2], cells[cells.len() - 1]);
        let run = a + 2;
        for (idx, start) in [step(centre, run), up(step(l, run), 1), up(step(r, run), 1)].into_iter().enumerate() {
            let length = self.random.next_int(2) + if idx != 0 { 4 } else { 2 };
            let mut p = start;
            for _ in 0..length {
                if !self.try_set(p, color) {
                    return;
                }
                if self.random.next_float() < 0.1 {
                    self.tops.push(up(p, 1));
                }
                for face in [side, other] {
                    if self.random.next_float() < 0.6 {
                        self.sides.push((p, face));
                    }
                }
                let rise = self.random.next_float() < 0.25 && self.fits(up(p, 2));
                p = up(step(p, run), i32::from(rise));
            }
        }
    }

    /// `_buildPlantArm`: an arm of 3 to 5 cells from `at` toward `face`, with decorations.
    fn arm(&mut self, at: [i32; 3], color: usize, face: usize, short: bool) {
        let kind = if short { self.random.next_int(2) + 1 } else { self.random.next_int(3) };
        let cell = |reef: &mut Self, p: [i32; 3]| reef.fits(up(p, 1)) && reef.try_set(p, color);
        match kind {
            0 => {
                if !cell(self, at) {
                    return;
                }
                let c = step(at, face);
                if !cell(self, up(c, 1)) {
                    return;
                }
                let c = up(step(c, face), 2);
                if !cell(self, c) {
                    return;
                }
                self.side(c);
                let c = up(c, 1);
                if !cell(self, c) {
                    return;
                }
                self.side(c);
                let c = up(step(c, OPPOSITE[face]), 1);
                if !cell(self, c) || self.random.next_float() >= 0.2 {
                    return;
                }
                self.tops.push(up(c, 1));
            }
            1 => {
                if !cell(self, at) {
                    return;
                }
                self.side(at);
                if !cell(self, up(at, 1)) {
                    return;
                }
                // BDS records the arm's base, not the cell it wrote, here and below.
                self.side(at);
                let c = up(step(at, face), 1);
                for dy in 0..3 {
                    if !cell(self, up(c, dy)) {
                        return;
                    }
                    self.side(c);
                }
                if self.random.next_float() < 0.2 {
                    self.tops.push(up(c, 3));
                }
            }
            _ => {
                if !self.try_set(at, color) {
                    return;
                }
                let mut c = up(step(at, face), 1);
                for turned in [false, true] {
                    if turned {
                        let clockwise = self.random.next_boolean();
                        c = up(step(c, turn(face, clockwise)), 1);
                    }
                    for dy in 0..2 {
                        if !cell(self, up(c, dy)) {
                            return;
                        }
                        self.side(c);
                    }
                }
                if self.random.next_float() < 0.2 {
                    self.tops.push(up(c, 2));
                }
            }
        }
    }
}
