//! What a mob's next tick is told to do: walk, swim up, fly, be thrown, be jostled.

use super::{Body, InLiquid, LEVITATES, Liquid, Physics, SLOW_FALL, liquid};

impl Body {
    /// A mob's slowing at the start of its tick, by gravity and drag in the air or `liquid`.
    pub(super) fn drag(&mut self, physics: &Physics, liquid: Option<Liquid>, stood: bool) {
        if physics.liquid == InLiquid::Swims && liquid == Some(Liquid::Water) {
            liquid::swim(self, physics.water_drag);
        } else if let Some(liquid) = liquid {
            liquid::sink(self, liquid);
        } else {
            // BDS slows a mob after its move by where it stood before it (research/captures/bds-chase.txt).
            let across = if stood { physics.ground_drag } else { physics.air_drag };
            (self.motion[0], self.motion[2]) = (self.motion[0] * across, self.motion[2] * across);
            let falling = self.motion[1] < 0.0;
            if self.lifted > 0.0 {
                self.motion[1] = (self.motion[1] + (self.lifted - self.motion[1]) * LEVITATES) * physics.fall_drag;
                return;
            }
            let gravity = if falling && self.slowed { SLOW_FALL } else { physics.gravity };
            self.motion[1] = (self.motion[1] - gravity) * physics.fall_drag;
            if falling {
                self.motion[1] *= physics.flutter;
            }
        }
    }

    /// Its motion as BDS holds it after a tick, which viewers are told: a mob's slows after its
    /// move there, where sapling slows it as its next tick starts (a jump's 0.42 up is told
    /// 0.3332; research/captures/bds-chase.txt, bds-kinds2.txt).
    pub fn held_motion(&self, physics: &Physics) -> [f32; 3] {
        if self.flung || !matches!(physics.liquid, InLiquid::Sinks | InLiquid::Swims) {
            return self.motion;
        }
        let mut held = *self;
        held.drag(physics, self.liquid, self.stood);
        held.motion[1] += self.geysered;
        if held.climbing {
            held.motion[1] = super::climb::CLIMB;
        }
        held.motion
    }

    /// Has a mob crowding this one push it by `by` across in its next tick, after the drag.
    pub fn jostle(&mut self, by: [f32; 2]) {
        self.jostled = [self.jostled[0] + by[0], self.jostled[1] + by[1]];
    }

    /// Throws a mob at `speed`: its next tick moves it by that, and the tick after slows it as
    /// one that left the ground it is on now (research/captures/bds-hit.txt).
    pub fn throw(&mut self, speed: [f32; 3]) {
        (self.thrown, self.flung) = (Some(speed), false);
    }

    /// As [`Body::throw`], its motion told as it is, with no drag to come (a frog's leap: BDS
    /// told it so every tick, research/captures/bds-amphibians.txt).
    pub fn fling(&mut self, speed: [f32; 3]) {
        (self.thrown, self.flung) = (Some(speed), true);
    }

    /// As [`Body::throw`], its first move `lift` higher, as a breeze leaps.
    pub fn launch(&mut self, speed: [f32; 3], lift: f32) {
        (self.thrown, self.lift) = (Some(speed), lift);
    }

    /// Has a mob walk: its next tick adds `speed` across, after the drag, and with `rise` goes up
    /// that fast (research/captures/bds-bite.txt, bds-chase.txt).
    pub fn walk(&mut self, speed: [f32; 2], rise: Option<f32>) {
        (self.walking, self.leaping) = ([speed[0], 0.0, speed[1]], rise);
    }

    /// Has a swimming mob's next tick add `speed` up, after the drag, on top of its walk.
    pub fn swim_up(&mut self, speed: f32) {
        self.walking[1] += speed;
    }

    /// Has a mob's ticks in the air rise towards `speed` in place of falling, as levitation does
    /// (0 for none): a fifth of the way each tick, then the fall's drag (bds-a3fx.txt).
    pub fn levitate(&mut self, speed: f32) {
        self.lifted = speed;
    }

    /// Has a mob's gravity be 0.01 in place of its own on the ticks it starts already falling, as slow
    /// falling does: a still mob's first tick fell by the full 0.08 (bds-a3fx2.txt).
    pub fn fall_slowly(&mut self, slowly: bool) {
        self.slowed = slowly;
    }

    /// Whether its last move was cut short by a block on any axis, by more than `f32::EPSILON`: BDS's
    /// `CollisionFlagComponent`, set so in `FinalizeMoveSystem` (Windows 14469ee70).
    pub fn collided(&self) -> bool {
        self.collided
    }

    /// Has a flying mob's next tick add `speed`, every way, after the drag.
    pub fn fly(&mut self, speed: [f32; 3]) {
        self.walking = speed;
    }
}
