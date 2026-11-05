//! A mob a block was set into, at the start of its tick: onto a low block's top, or out beside a
//! full one (docs/research/mob-damage.md, "In a block").

use acacia_physics::Aabb;
use sapling_world::World as Level;

use super::{Body, TOUCH, overlapped};
use crate::terrain::physics_view;

/// The ways out, tried in this order when as near: BDS's cow in the middle of a stone cell went south
/// (research/captures/bds-bodies.txt).
const WAYS: [(usize, f32); 4] = [(2, 1.0), (2, -1.0), (0, 1.0), (0, -1.0)];

impl Body {
    /// Lifts a mob standing in a block lower than a full one onto its top: BDS's cows went onto blocks
    /// of 0.375 to 0.9375 set where they stood, the tick after, and stayed in a full one walled in
    /// all round (research/captures/bds-lifts.txt).
    pub(super) fn rise_onto_low_block(&mut self, level: &Level) {
        let feet = self.at[1];
        let under = physics_view(level, |view| overlapped(view, &self.aabb())).into_iter().filter(|b| b.min[1] <= feet + TOUCH).map(|b| b.max[1]).reduce(f32::max);
        if let Some(top) = under.filter(|top| *top > feet + TOUCH && *top - feet < 1.0 - TOUCH) {
            (self.at[1], self.motion[1]) = (top, 0.0);
        }
    }

    /// Puts a mob a full-height block reaches into at its feet beside it, its box flush with the
    /// nearest side that has room: BDS's cows 0.65 and 0.16 into a stone set at their feet went that
    /// far east the same tick (research/captures/bds-m3inblock.txt).
    pub(super) fn push_out_of_block(&mut self, level: &Level) {
        let feet = self.at[1];
        let blocks = physics_view(level, |view| overlapped(view, &self.aabb()));
        let walls: Vec<&Aabb> = blocks.iter().filter(|b| b.min[1] <= feet + TOUCH && b.max[1] - feet >= 1.0 - TOUCH).collect();
        let (Some(first), (min, max)) = (walls.first(), self.bounds()) else { return };
        let (lo, hi) = walls.iter().fold((first.min, first.max), |(lo, hi), b| (std::array::from_fn(|i| lo[i].min(b.min[i])), std::array::from_fn(|i| hi[i].max(b.max[i]))));
        let half = self.size[0] / 2.0;
        let mut ways: Vec<(f32, usize, f32)> = WAYS.iter().map(|&(axis, sign)| (if sign > 0.0 { hi[axis] - min[axis] } else { max[axis] - lo[axis] }, axis, sign)).collect();
        // Thousandths, so a tie in f32's rounding stays a tie.
        ways.sort_by_key(|way| (way.0 * 1000.0).round() as i32);
        for (_, axis, sign) in ways {
            let mut at = self.at;
            at[axis] = if sign > 0.0 { hi[axis] + half } else { lo[axis] - half };
            let room = Aabb { min: [at[0] - half + TOUCH, feet + TOUCH, at[2] - half + TOUCH], max: [at[0] + half - TOUCH, feet + self.size[1] - TOUCH, at[2] + half - TOUCH] };
            if physics_view(level, |view| overlapped(view, &room)).is_empty() {
                self.at = at;
                return;
            }
        }
    }
}
