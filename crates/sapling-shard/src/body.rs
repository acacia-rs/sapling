//! A body the server alone moves (falling blocks, items, orbs, mobs), as BDS moves such actors: gravity,
//! then the move, cut short by the collision boxes of blocks (acacia-physics `collide_box`), then drag
//! (docs/research/falling-blocks.md, experience.md, items.md), and water and lava (liquid.rs).
//! What viewers are told: sync.rs.

mod bubbles;
mod geyser;
mod liquid;
mod moves;
mod physics;
mod spawn_push;
mod step;
mod stuck;

use acacia_physics::{Aabb, WorldView, collide_box};
use bevy_ecs::prelude::Component;
pub(crate) use liquid::{InLiquid, Liquid, in_liquid, liquid_in, surface};
pub(crate) use physics::Physics;
pub(crate) use spawn_push::{pushed_out, pushed_out_standing};
use sapling_api::BlockPos;
use sapling_world::World as Level;

use crate::terrain::{physics_view, physics_view_clearing};

/// How close a top face is to a body's feet to stand it.
const TOUCH: f32 = 1e-4;
/// How much of the way to its levitation speed a mob's rise goes each tick.
const LEVITATES: f32 = 0.2;
/// A slow-falling mob's gravity once it falls (moves.rs `fall_slowly`).
const SLOW_FALL: f32 = 0.01;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct Body {
    /// The middle of its bottom face.
    pub at: [f32; 3],
    pub motion: [f32; 3],
    /// Width and height.
    pub size: [f32; 2],
    pub on_ground: bool,
    /// The liquid found to act on it, by its rule, where the last tick started.
    liquid: Option<Liquid>,
    /// Where the wave a buoyant body bobs on has got to.
    wave: f32,
    /// The speed a hit throws a mob at; its next tick is that move alone, this much higher.
    thrown: Option<[f32; 3]>,
    lift: f32,
    /// The speed its own legs or wings add in its next tick.
    walking: [f32; 3],
    leaping: Option<f32>,
    /// Whether a mob was on the ground before its last move: that move's drag goes by it.
    stood: bool,
    /// Whether a push up or down this tick left it on the ground: BDS pushes after the body's
    /// own move, so its next tick does not change what viewers are told (bds-shove.txt).
    pushed: Option<bool>,
    /// What mobs crowding a mob push it by across in its next tick, after the drag.
    jostled: [f32; 2],
    /// Whether its motion is told as it moves, not after the next tick's drag (a frog's leap).
    flung: bool,
    /// Whether it has had a tick: in its first a mob goes by the liquid it is in (bds-r1water.txt).
    stepped: bool,
    /// The speed up an erupting geyser gave a mob after its last move, which its next tick adds (geyser.rs).
    geysered: f32,
    /// The speed levitation lifts a mob towards, or 0 (moves.rs `levitate`).
    lifted: f32,
    /// Whether slow falling has it fall by [`SLOW_FALL`].
    slowed: bool,
    /// Whether its last slide fell short of the wanted move on any axis (moves.rs `collided`).
    collided: bool,
}

/// A box in world coordinates: its lowest and highest corners.
type Bounds = ([f32; 3], [f32; 3]);

impl Body {
    pub fn new(at: [f32; 3], motion: [f32; 3], size: [f32; 2]) -> Self {
        Self { at, motion, size, on_ground: false, liquid: None, wave: 0.0, thrown: None, lift: 0.0, walking: [0.0; 3], leaping: None, stood: false, pushed: None, jostled: [0.0; 2], flung: false, stepped: false, geysered: 0.0, lifted: 0.0, slowed: false, collided: false }
    }

    /// Moves it by `by` as far as the blocks let it, its own motion kept.
    pub fn shove(&mut self, level: &Level, by: [f32; 3]) {
        let motion = self.motion;
        self.slide(level, by);
        self.motion = motion;
    }

    /// A piston's push by `by`, through the blocks going into `moving` (they are behind it on
    /// BDS, which shifts them by the piston's progress); a push up or down also decides whether
    /// viewers are told it is on the ground this tick.
    pub fn push(&mut self, level: &Level, by: [f32; 3], moving: &[BlockPos]) {
        let motion = self.motion;
        let landed = physics_view_clearing(level, moving, |view| self.slide_in(view, by));
        self.motion = motion;
        if by[1] != 0.0 {
            self.pushed = Some(landed);
        }
    }

    /// Whether viewers are told it is on the ground where a push left it.
    pub fn pushed_ground(&self) -> bool { self.pushed.unwrap_or(self.on_ground) }

    /// The liquid found to act on it where the last tick started; a mob went by the one before.
    pub fn liquid(&self) -> Option<Liquid> {
        self.liquid
    }

    pub(crate) fn bounds(&self) -> Bounds {
        let half = self.size[0] / 2.0;
        ([self.at[0] - half, self.at[1], self.at[2] - half], [self.at[0] + half, self.at[1] + self.size[1], self.at[2] + half])
    }

    fn aabb(&self) -> Aabb {
        let (min, max) = self.bounds();
        Aabb { min, max }
    }

    /// Lifts a body whose box is in a block onto that block's top, stopped, as BDS did an item
    /// dropped 0.05 into the floor (research/captures/bds-kill.txt); the tick's step lands it.
    pub fn lift_out(&mut self, level: &Level) {
        if let Some(top) = physics_view(level, |view| overlapped(view, &self.aabb())).iter().map(|b| b.max[1]).reduce(f32::max) {
            (self.at[1], self.motion) = (top, [0.0; 3]);
        }
    }

    /// Whether no block's collision box reaches into its own.
    pub fn is_free(&self, level: &Level) -> bool {
        physics_view(level, |view| overlapped(view, &self.aabb())).is_empty()
    }

    /// One tick: gravity, the move, then drag; on the ground, friction and no fall left. In water or
    /// lava, by the body's [`InLiquid`] rule, judged where the tick starts; a mob by
    /// [`Body::mob_step`]. `random` is drawn on for an orb's waves and its throw out of lava. The
    /// motion it moved by.
    pub fn step(&mut self, level: &Level, physics: &Physics, random: &mut impl FnMut() -> f32) -> [f32; 3] {
        // BDS's piston pushes after a body's own tick: one pushed up or down this tick does not
        // move again in it (research/captures/bds-e3pmob.txt).
        if let Some(on_ground) = self.pushed.take() {
            self.on_ground = on_ground;
            return [0.0; 3];
        }
        self.own_step(level, physics, random)
    }

    fn own_step(&mut self, level: &Level, physics: &Physics, random: &mut impl FnMut() -> f32) -> [f32; 3] {
        let found = liquid::acting(level, self, physics.liquid);
        let before = std::mem::replace(&mut self.liquid, found);
        let first = !std::mem::replace(&mut self.stepped, true);
        match (physics.liquid, found) {
            // A mob goes by what was found a tick before (research/captures/bds-water.txt, the armour stand).
            (InLiquid::Sinks | InLiquid::Swims, _) => return self.mob_step(level, physics, if first { found } else { before }),
            (InLiquid::Floats, Some(liquid)) => liquid::lift(self, liquid),
            (InLiquid::Buoyant, Some(Liquid::Water)) => {
                liquid::buoy(level, self, random);
                self.motion[1] -= physics.gravity;
            }
            (InLiquid::Buoyant, Some(Liquid::Lava)) => liquid::throw_up(self, random),
            _ => self.motion[1] -= physics.gravity,
        }
        bubbles::push(level, self);
        let wanted = self.motion;
        let landed = self.slide(level, wanted);
        self.on_ground = landed;
        for axis in [0, 2] {
            self.motion[axis] *= if landed { physics.ground_drag } else { physics.air_drag };
        }
        self.motion[1] *= physics.fall_drag;
        if landed {
            self.motion[1] = 0.0;
        }
        wanted
    }

    /// A mob's tick: its speed first, by drag and gravity in the air or in `liquid`, then the move;
    /// the motion it moved by.
    fn mob_step(&mut self, level: &Level, physics: &Physics, liquid: Option<Liquid>) -> [f32; 3] {
        let (walking, leaping) = (std::mem::take(&mut self.walking), std::mem::take(&mut self.leaping));
        let [jostle_x, jostle_z] = std::mem::take(&mut self.jostled);
        let walking = [walking[0] + jostle_x, walking[1], walking[2] + jostle_z];
        let stood = std::mem::replace(&mut self.stood, self.on_ground);
        self.rise_onto_low_block(level);
        self.push_out_of_block(level);
        if let Some(speed) = self.thrown.take() {
            self.motion = speed;
            let lift = std::mem::take(&mut self.lift);
            if lift > 0.0 {
                // A leap is off the ground at once: BDS's breeze kept its launch speed across (bds-g10breeze.txt).
                self.stood = false;
            }
            let moved = [speed[0], speed[1] + lift, speed[2]];
            self.on_ground = self.travel(level, physics, moved, stood);
            return moved;
        }
        self.flung = false;
        self.drag(physics, liquid, stood);
        self.motion[1] += std::mem::take(&mut self.geysered);
        self.motion = std::array::from_fn(|axis| self.motion[axis] + walking[axis]);
        if let Some(rise) = leaping {
            self.motion[1] = rise;
        }
        bubbles::push_columns(level, self);
        let wanted = self.motion;
        self.on_ground = self.travel(level, physics, wanted, stood);
        if self.on_ground {
            self.motion[1] = 0.0;
        }
        self.geysered = geyser::boost(level, self, self.held_motion(physics)[1]);
        wanted
    }

    /// A mob's slowing at the start of its tick, by gravity and drag in the air or `liquid`.
    fn drag(&mut self, physics: &Physics, liquid: Option<Liquid>, stood: bool) {
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
        held.motion
    }

    /// A mob's move by `motion`: through blocks if its physics pass them, else [`Body::slide`] with
    /// its step up (step.rs) where it `stood` on the ground or lands.
    fn travel(&mut self, level: &Level, physics: &Physics, motion: [f32; 3], stood: bool) -> bool {
        if !physics.passes {
            return physics_view(level, |view| {
                let moved = self.step_move(view, motion, physics.step, stood);
                self.slide_by(view, motion, moved)
            });
        }
        self.at = std::array::from_fn(|axis| self.at[axis] + motion[axis]);
        false
    }

    /// Moves by `motion` as far as the blocks let it; motion into a block is stopped, except
    /// downward, which the caller settles. Whether it came down onto something.
    fn slide(&mut self, level: &Level, motion: [f32; 3]) -> bool {
        physics_view(level, |view| self.slide_in(view, motion))
    }

    fn slide_in(&mut self, view: &impl WorldView, motion: [f32; 3]) -> bool {
        let moved = collide_box(view, &self.aabb(), motion);
        self.slide_by(view, motion, moved)
    }

    /// Moves by `moved` of the wanted `motion`: see [`Body::slide`].
    fn slide_by(&mut self, view: &impl WorldView, motion: [f32; 3], moved: [f32; 3]) -> bool {
        let start = self.aabb();
        self.collided = (0..3).any(|axis| (moved[axis] - motion[axis]).abs() > f32::EPSILON);
        for axis in 0..3 {
            self.at[axis] += moved[axis];
            if moved[axis] != motion[axis] && (axis != 1 || motion[axis] > 0.0) {
                self.motion[axis] = 0.0;
            }
        }
        // Only a top face under its feet stands it: one held up inside a block does not (BDS's stand
        // half-pushed by a piston, bds-shove.txt). The fall is stopped before the move across, so a mob
        // whose box clears an edge this tick still stood on it (bds-bc1step.txt).
        let fell = start.translate([0.0, moved[1], 0.0]);
        motion[1] < 0.0 && moved[1] != motion[1] && (rests_on_top(view, &fell) || rests_on_top(view, &start.translate(moved)))
    }
}

/// Whether a block's top face is where the bottom of `bounds` is.
fn rests_on_top(view: &impl WorldView, bounds: &Aabb) -> bool {
    overlapped(view, &bounds.extend_down(TOUCH)).iter().any(|b| (bounds.min[1] - b.max[1]).abs() <= TOUCH)
}

/// The collision boxes of blocks that reach into `bounds`.
fn overlapped(view: &impl WorldView, bounds: &Aabb) -> Vec<Aabb> {
    let mut boxes = Vec::new();
    view.collisions(bounds, &mut boxes);
    boxes
}

#[cfg(test)]
mod tests;
