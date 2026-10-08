//! `CoralFeature::_buildPlantArm` (148291d20): the arms of shape 0, three kinds by `nextInt(3)`
//! (kinds 1 and 2 only for the last arm).

use super::{OPPOSITE, Reef, step, up};

/// A turn: counter-clockwise when `clockwise` is false (0x14a925e88, 0x14a7efe20).
fn turn(face: usize, clockwise: bool) -> usize {
    let table: &[usize] = if clockwise { &[5, 4, 2, 3] } else { &[4, 5, 3] };
    table.get(face - 2).copied().unwrap_or(2)
}

impl Reef<'_, '_> {
    /// An arm of 3 to 5 cells from `at` toward `face`, each written where water is above it.
    pub(super) fn arm(&mut self, at: [i32; 3], color: usize, face: usize, short: bool) {
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
