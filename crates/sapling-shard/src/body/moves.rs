//! What a mob's next tick is told to do: walk, swim up, fly, be thrown, be jostled.

use super::Body;

impl Body {
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

    /// Has a flying mob's next tick add `speed`, every way, after the drag.
    pub fn fly(&mut self, speed: [f32; 3]) {
        self.walking = speed;
    }
}
