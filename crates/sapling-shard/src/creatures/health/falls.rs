//! A mob's falls: how far it has dropped since it left the ground, and what the landing costs.

use super::{Cause, Health};

/// A fall this far does not hurt; each block more, begun, costs 1.
const SAFE_FALL: f32 = 3.0;
/// A landing on a stalagmite's tip counts this much more of the fall, this many times over
/// (`SpeleothemBlock` fall-on 14660bc00; research/captures/bds-i2drip.txt: 8 for some 5 blocks).
const SPIKE_MORE: f32 = 2.0;
const SPIKE_TIMES: f32 = 2.0;

/// What a landing is onto: a block that counts this share of the fall (heat.rs), or a standing
/// dripstone's tip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Ground {
    Soft(f32),
    Spike,
}

impl Ground {
    /// What a drop of `drop` onto it costs one whose jumps spring it `springs` higher.
    fn cost(self, drop: f32, springs: f32) -> f32 {
        match self {
            Ground::Soft(share) => (drop * share - SAFE_FALL - springs).ceil(),
            Ground::Spike => ((drop + SPIKE_MORE - SAFE_FALL - springs) * SPIKE_TIMES).ceil(),
        }
    }

    pub(crate) fn cause(self) -> Cause {
        if self == Ground::Spike { Cause::Stalagmite } else { Cause::Fall }
    }
}

impl Health {
    /// A tick's move from `from` to the height `to`, landing `on_ground` or `in_liquid`: the
    /// last tick's landing if it hurt, as what it cost, where that tick began and its cause. BDS
    /// hurts a tick after the move that lands.
    pub(crate) fn fall(&mut self, from: [f32; 3], to: f32, on_ground: bool, in_liquid: bool) -> Option<(f32, [f32; 3], Cause)> {
        self.fall_onto(from, to, on_ground, in_liquid, Ground::Soft(1.0))
    }

    /// [`Health::fall`] onto `ground`.
    pub(crate) fn fall_onto(&mut self, from: [f32; 3], to: f32, on_ground: bool, in_liquid: bool, ground: Ground) -> Option<(f32, [f32; 3], Cause)> {
        let due = self.landed.take().filter(|(harm, _, cause)| self.hurt(self.warded(*harm, *cause)));
        let peak = self.peak.map_or(from[1], |peak| peak.max(from[1])).max(to);
        self.peak = (!on_ground && !in_liquid).then_some(peak);
        let harm = ground.cost(peak - to, self.springs);
        if on_ground && !in_liquid && harm > 0.0 {
            self.landed = Some((harm, from, ground.cause()));
        }
        due
    }

    /// Whether it was off the ground when its last tick ended.
    pub(crate) fn is_airborne(&self) -> bool {
        self.peak.is_some()
    }

    /// What the landing of this tick will cost, the next.
    pub(crate) fn landing(&self) -> Option<f32> {
        self.landed.map(|(harm, ..)| harm)
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
