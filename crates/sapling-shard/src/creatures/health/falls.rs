//! A mob's falls: how far it has dropped since it left the ground, and what the landing costs.

use super::{Cause, Health};

/// A fall this far does not hurt; each block more, begun, costs 1.
const SAFE_FALL: f32 = 3.0;

impl Health {
    /// A tick's move from `from` to the height `to`, landing `on_ground` or `in_liquid`: the
    /// last tick's landing if it hurt, as what it cost and where that tick began. BDS hurts a
    /// tick after the move that lands.
    pub(crate) fn fall(&mut self, from: [f32; 3], to: f32, on_ground: bool, in_liquid: bool) -> Option<(f32, [f32; 3])> {
        self.fall_onto(from, to, on_ground, in_liquid, 1.0)
    }

    /// [`Health::fall`] onto a block that counts the fall's height times `soften` (heat.rs).
    pub(crate) fn fall_onto(&mut self, from: [f32; 3], to: f32, on_ground: bool, in_liquid: bool, soften: f32) -> Option<(f32, [f32; 3])> {
        let due = self.landed.take().filter(|(harm, _)| self.hurt(self.warded(*harm, Cause::Fall)));
        let peak = self.peak.map_or(from[1], |peak| peak.max(from[1])).max(to);
        self.peak = (!on_ground && !in_liquid).then_some(peak);
        let harm = ((peak - to) * soften - SAFE_FALL - self.springs).ceil();
        if on_ground && !in_liquid && harm > 0.0 {
            self.landed = Some((harm, from));
        }
        due
    }

    /// Whether it was off the ground when its last tick ended.
    pub(crate) fn is_airborne(&self) -> bool {
        self.peak.is_some()
    }

    /// What the landing of this tick will cost, the next.
    pub(crate) fn landing(&self) -> Option<f32> {
        self.landed.map(|(harm, _)| harm)
    }

    /// How far it has fallen when it comes from `from` to `to`.
    pub(crate) fn dropped(&self, from: f32, to: f32) -> f32 {
        self.peak.map_or(from, |peak| peak.max(from)).max(to) - to
    }

    /// Forgets how far it has fallen: a fall from `at` starts anew.
    pub(crate) fn break_fall(&mut self, at: f32) {
        if self.peak.is_some() {
            self.peak = Some(at);
        }
    }
}
