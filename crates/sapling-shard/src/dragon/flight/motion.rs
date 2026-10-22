//! How the dragon moves (`EnderDragon::aiStep` 145994090, research/dragon-ai-decompile.md 3),
//! f32 as BDS: each tick its stored motion carries it first (BDS's generic move, fitted to the
//! drills), then, with a target, the climb toward it clamped to the air speed, a turn of at most
//! 50 degrees eased by the yaw speed, a push forward of 0.06, a second move, and the drag. Its
//! wings beat by how fast it goes; a flap is heard as their cosine passes -0.3.

const CLIMB: f32 = 0.1;
const TURN: f32 = 50.0;
const PUSH: f32 = 0.06;
const SLOWED: f32 = 0.8;
const FALL_DRAG: f32 = 0.91;
const AIR_FRICTION: f32 = 0.91;
const FLAP_AT: f32 = -0.3;
const SITTING_BEAT: f32 = 0.1;
const DEGREES: f32 = 57.295_776;

/// Degrees into -180..180, as BDS's `fmodf` wraps them.
pub(crate) fn wrap(degrees: f32) -> f32 {
    let mut a = (degrees + 180.0) % 360.0;
    if a < 0.0 {
        a += 360.0;
    }
    a - 180.0
}

fn normalised(v: [f32; 3]) -> [f32; 3] {
    let length = v.iter().map(|c| c * c).sum::<f32>().sqrt();
    if length < 1e-4 { [0.0; 3] } else { v.map(|c| c / length) }
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| a[axis] * b[axis]).sum()
}

/// The dragon's body in flight: where it is, its stored motion and turn, and its wings.
#[derive(Clone, Debug)]
pub(crate) struct Motion {
    pub at: [f32; 3],
    pub delta: [f32; 3],
    pub yaw: f32,
    pub yaw_speed: f32,
    /// Its most climb or fall a tick, tenfold (`AirSpeedComponent`).
    pub air_speed: f32,
    /// The turn's numerator; 0 while landing, which turns by speed alone.
    pub turn_factor: f32,
    pub target: Option<[f32; 3]>,
    pub anim: f32,
    pub prev_anim: f32,
    /// Its last move was through blocks it could not break.
    pub slowed: bool,
}

impl Motion {
    pub fn new(at: [f32; 3]) -> Self {
        Self { at, delta: [0.0; 3], yaw: 0.0, yaw_speed: 0.0, air_speed: 0.6, turn_factor: 0.7, target: None, anim: 0.0, prev_anim: 0.0, slowed: false }
    }

    /// The move BDS makes of every actor before the dragon's own: by its stored motion, after which
    /// the air takes its xz (fitted to 4 digits on research/captures/bds-dragon-watch.txt's first ticks).
    pub fn carry(&mut self) {
        self.at = std::array::from_fn(|axis| self.at[axis] + self.delta[axis]);
        (self.delta[0], self.delta[2]) = (self.delta[0] * AIR_FRICTION, self.delta[2] * AIR_FRICTION);
    }

    /// Whether a wing beat is heard this tick, by last tick's beat.
    pub fn flaps(&self) -> bool {
        let at = |anim: f32| (anim * std::f32::consts::TAU).cos();
        at(self.prev_anim) <= FLAP_AT && FLAP_AT <= at(self.anim)
    }

    /// The wings beat on: a tenth while it sits, else faster the slower it goes.
    pub fn beat(&mut self, before: [f32; 3], sitting: bool) {
        self.prev_anim = self.anim;
        let across = (self.at[0] - before[0]).hypot(self.at[2] - before[2]);
        let f = 0.2 / (across * 10.0 + 1.0) * (before[1] - self.at[1]).exp2();
        self.anim += if sitting { SITTING_BEAT } else if self.slowed { f * 0.5 } else { f };
    }

    /// The dragon's own move toward its target, if it has one.
    pub fn fly(&mut self) {
        self.yaw = wrap(self.yaw);
        let Some(target) = self.target else { return };
        let [dx, dy, dz] = std::array::from_fn(|axis| target[axis] - self.at[axis]);
        let k = self.air_speed;
        self.delta[1] += (dy / dx.hypot(dz)).clamp(-k, k) * CLIMB;
        self.yaw = wrap(self.yaw);
        let turn = wrap(180.0 - dx.atan2(dz) * DEGREES - self.yaw).clamp(-TURN, TURN);
        let a = normalised([dx, dy, dz]);
        let (sin, cos) = (self.yaw / DEGREES).sin_cos();
        let b = normalised([sin, self.delta[1], -cos]);
        let f7 = ((dot(a, b) + 0.5) / 1.5).max(0.0);
        let s = self.delta[0].hypot(self.delta[2]) + 1.0;
        let factor = if self.turn_factor != 0.0 { self.turn_factor / s.min(40.0) / s } else { s.min(40.0) / s };
        self.yaw_speed = self.yaw_speed * 0.8 + turn * factor;
        self.yaw += self.yaw_speed * 0.1;
        let f8 = 2.0 / (dx * dx + dy * dy + dz * dz + 1.0);
        let sp = PUSH * ((1.0 - f8) + f7 * f8);
        let (sin, cos) = (self.yaw / DEGREES).sin_cos();
        self.delta[0] += sin * sp;
        self.delta[2] -= cos * sp;
        let moved = if self.slowed { self.delta.map(|c| c * SLOWED) } else { self.delta };
        self.at = std::array::from_fn(|axis| self.at[axis] + moved[axis]);
        let c = normalised(self.delta);
        let f10 = 0.8 + 0.15 * (dot(c, b) + 1.0) * 0.5;
        (self.delta[0], self.delta[1], self.delta[2]) = (self.delta[0] * f10, self.delta[1] * FALL_DRAG, self.delta[2] * f10);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn its_first_falls_are_bdss() {
        // research/captures/bds-dragon-fight.txt: a new dragon's first five steps down.
        let mut motion = Motion { target: Some([-40.0, 80.0, -40.0]), ..Motion::new([0.0, 128.0, 0.0]) };
        let mut steps = Vec::new();
        for _ in 0..5 {
            let before = motion.at[1];
            motion.carry();
            motion.fly();
            steps.push(motion.at[1] - before);
        }
        let bds = [-0.06, -0.1692, -0.2686, -0.359, -0.4413];
        assert!(steps.iter().zip(bds).all(|(step, bds)| (step - bds).abs() < 2e-4), "{steps:?}");
    }

    #[test]
    fn its_first_steps_across_are_bdss() {
        // research/captures/bds-dragon-watch.txt, by the yaws it showed; its target is far ahead.
        let mut motion = Motion { turn_factor: 0.0, ..Motion::new([0.0, 128.0, 0.0]) };
        let mut steps = Vec::new();
        for yaw in [357.19_f32, 351.56, 344.53, 336.09] {
            let before = motion.at;
            motion.carry();
            (motion.yaw, motion.yaw_speed) = (wrap(yaw), 0.0);
            let (sin, cos) = yaw.to_radians().sin_cos();
            motion.target = Some([motion.at[0] + sin * 1e4, -1e4, motion.at[2] - cos * 1e4]);
            motion.fly();
            steps.push((motion.at[0] - before[0]).hypot(motion.at[2] - before[2]));
        }
        let bds = [0.06, 0.1664, 0.2564, 0.3323];
        assert!(steps.iter().zip(bds).all(|(step, bds)| (step - bds).abs() < 3e-3), "{steps:?}");
    }
}
