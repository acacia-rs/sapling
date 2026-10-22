//! A body the server alone moves (falling blocks, items, orbs, mobs), as BDS moves such actors: gravity,
//! then the move, cut short by the collision boxes of blocks (acacia-physics `collide_box`), then drag
//! (docs/research/falling-blocks.md, experience.md, items.md), and water and lava (liquid.rs).
//! What viewers are told: sync.rs.
// TODO: only items are lifted out of a block they are in, and only up.

mod liquid;
mod physics;

use acacia_physics::constants::DEFAULT_JUMP_HEIGHT;
use acacia_physics::{Aabb, WorldView, collide_box};
use bevy_ecs::prelude::Component;
pub(crate) use liquid::{InLiquid, Liquid, in_liquid};
pub(crate) use physics::Physics;
use sapling_world::World as Level;

use crate::terrain::physics_view;

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
    leaping: bool,
    /// Whether a mob was on the ground before its last move: that move's drag goes by it.
    stood: bool,
}

/// A box in world coordinates: its lowest and highest corners.
type Bounds = ([f32; 3], [f32; 3]);

impl Body {
    pub fn new(at: [f32; 3], motion: [f32; 3], size: [f32; 2]) -> Self {
        Self { at, motion, size, on_ground: false, liquid: None, wave: 0.0, thrown: None, lift: 0.0, walking: [0.0; 3], leaping: false, stood: false }
    }

    /// Throws a mob at `speed`: its next tick moves it by that, and the tick after slows it as
    /// one that left the ground it is on now (research/captures/bds-hit.txt).
    pub fn throw(&mut self, speed: [f32; 3]) {
        self.thrown = Some(speed);
    }

    /// As [`Body::throw`], its first move `lift` higher, as a breeze leaps.
    pub fn launch(&mut self, speed: [f32; 3], lift: f32) {
        (self.thrown, self.lift) = (Some(speed), lift);
    }

    /// Has a mob walk: its next tick adds `speed` across, after the drag, and with `leap` goes
    /// up at a jump's speed (research/captures/bds-bite.txt, bds-chase.txt).
    pub fn walk(&mut self, speed: [f32; 2], leap: bool) {
        (self.walking, self.leaping) = ([speed[0], 0.0, speed[1]], leap);
    }

    /// Has a flying mob's next tick add `speed`, every way, after the drag.
    pub fn fly(&mut self, speed: [f32; 3]) {
        self.walking = speed;
    }

    /// Moves it by `by` as far as the blocks let it, its own motion kept: a piston's push.
    pub fn shove(&mut self, level: &Level, by: [f32; 3]) {
        let motion = self.motion;
        self.slide(level, by);
        self.motion = motion;
    }

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
        if let Some(top) = self.overlapped(level).iter().map(|b| b.max[1]).reduce(f32::max) {
            (self.at[1], self.motion) = (top, [0.0; 3]);
        }
    }

    /// Whether no block's collision box reaches into its own.
    pub fn is_free(&self, level: &Level) -> bool {
        self.overlapped(level).is_empty()
    }

    fn overlapped(&self, level: &Level) -> Vec<Aabb> {
        let mut boxes = Vec::new();
        physics_view(level, |view| view.collisions(&self.aabb(), &mut boxes));
        boxes
    }

    /// One tick: gravity, the move, then drag; on the ground, friction and the bounce. In water or
    /// lava, by the body's [`InLiquid`] rule, judged where the tick starts; a mob by
    /// [`Body::mob_step`]. `random` is drawn on for an orb's waves and its throw out of lava. The
    /// motion it moved by.
    pub fn step(&mut self, level: &Level, physics: &Physics, random: &mut impl FnMut() -> f32) -> [f32; 3] {
        let found = liquid::acting(level, self, physics.liquid);
        let before = std::mem::replace(&mut self.liquid, found);
        match (physics.liquid, found) {
            // A mob goes by what was found a tick before (research/captures/bds-water.txt, the armour stand).
            (InLiquid::Sinks | InLiquid::Swims, _) => return self.mob_step(level, physics, before),
            (InLiquid::Floats, Some(liquid)) => liquid::lift(self, liquid),
            (InLiquid::Buoyant, Some(Liquid::Water)) => {
                liquid::buoy(level, self, random);
                self.motion[1] -= physics.gravity;
            }
            (InLiquid::Buoyant, Some(Liquid::Lava)) => liquid::throw_up(self, random),
            _ => self.motion[1] -= physics.gravity,
        }
        let wanted = self.motion;
        let landed = self.slide(level, wanted);
        self.on_ground = landed;
        for axis in [0, 2] {
            self.motion[axis] *= if landed { physics.ground_drag } else { physics.air_drag };
        }
        self.motion[1] *= physics.fall_drag;
        if landed {
            self.motion[1] *= physics.bounce;
        }
        wanted
    }

    /// A mob's tick: its speed first, by drag and gravity in the air or in `liquid`, then the move;
    /// the motion it moved by.
    fn mob_step(&mut self, level: &Level, physics: &Physics, liquid: Option<Liquid>) -> [f32; 3] {
        let (walking, leaping) = (std::mem::take(&mut self.walking), std::mem::take(&mut self.leaping));
        let stood = std::mem::replace(&mut self.stood, self.on_ground);
        if let Some(speed) = self.thrown.take() {
            self.motion = speed;
            let moved = [speed[0], speed[1] + std::mem::take(&mut self.lift), speed[2]];
            self.on_ground = self.travel(level, physics, moved);
            return moved;
        }
        if physics.liquid == InLiquid::Swims && liquid == Some(Liquid::Water) {
            liquid::swim(self);
        } else if let Some(liquid) = liquid {
            liquid::sink(self, liquid);
        } else {
            // BDS slows a mob after its move by where it stood before it (research/captures/bds-chase.txt).
            let across = if stood { physics.ground_drag } else { physics.air_drag };
            (self.motion[0], self.motion[2]) = (self.motion[0] * across, self.motion[2] * across);
            self.motion[1] = (self.motion[1] - physics.gravity) * physics.fall_drag;
        }
        self.motion = std::array::from_fn(|axis| self.motion[axis] + walking[axis]);
        if leaping {
            self.motion[1] = DEFAULT_JUMP_HEIGHT;
        }
        let wanted = self.motion;
        self.on_ground = self.travel(level, physics, wanted);
        if self.on_ground {
            self.motion[1] = 0.0;
        }
        wanted
    }

    /// A mob's move by `motion`: through blocks if its physics pass them, else [`Body::slide`].
    fn travel(&mut self, level: &Level, physics: &Physics, motion: [f32; 3]) -> bool {
        if !physics.passes {
            return self.slide(level, motion);
        }
        self.at = std::array::from_fn(|axis| self.at[axis] + motion[axis]);
        false
    }

    /// Moves by `motion` as far as the blocks let it; motion into a block is stopped, except
    /// downward, which the bounce answers. Whether it came down onto something.
    fn slide(&mut self, level: &Level, motion: [f32; 3]) -> bool {
        let moved = physics_view(level, |view| collide_box(view, &self.aabb(), motion));
        for axis in 0..3 {
            self.at[axis] += moved[axis];
            if moved[axis] != motion[axis] && (axis != 1 || motion[axis] > 0.0) {
                self.motion[axis] = 0.0;
            }
        }
        motion[1] < 0.0 && moved[1] != motion[1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A void world with stone at y 64 and an oak slab (bottom half) at (2, 65, 0).
    fn level() -> Level {
        let mut level = Level::void();
        let stone = level.block_id("minecraft:stone").unwrap();
        level.fill([-4, 64, -4], [4, 64, 4], stone);
        let slab = level.registry().find("minecraft:oak_slab", "minecraft:vertical_half=bottom").unwrap();
        level.set_block(2, 65, 0, slab);
        level
    }

    fn fall(mut body: Body, physics: &Physics, ticks: usize) -> Body {
        let level = level();
        (0..ticks).for_each(|_| _ = body.step(&level, physics, &mut || 0.5));
        body
    }

    #[test]
    fn a_body_falls_onto_the_floor_and_rests_on_it() {
        let body = fall(Body::new([0.5, 68.0, 0.5], [0.0; 3], [0.98, 0.98]), &Physics::FALLING_BLOCK, 40);
        assert_eq!((body.at, body.on_ground), ([0.5, 65.0, 0.5], true));
        let on_slab = fall(Body::new([2.5, 68.0, 0.5], [0.0; 3], [0.25, 0.25]), &Physics::ITEM, 60);
        assert_eq!(on_slab.at[1], 65.5, "a slab's top is half a block up");
    }

    #[test]
    fn falling_takes_the_games_ticks_and_speed() {
        // 0.04 a tick, 2% drag: three blocks take 13 ticks from rest; BDS's sand, announced a
        // tick into its fall, landed 12 after that.
        let body = Body::new([0.5, 68.0, 0.5], [0.0; 3], [0.98, 0.98]);
        let ticks = (1..).find(|ticks| fall(body, &Physics::FALLING_BLOCK, *ticks).on_ground).unwrap();
        assert_eq!(ticks, 13);
    }

    #[test]
    fn a_wall_stops_a_body_and_ground_friction_slows_it() {
        let mut level = level();
        let stone = level.block_id("minecraft:stone").unwrap();
        level.set_block(1, 65, 3, stone);
        let mut body = Body::new([0.5, 65.0, 3.5], [0.5, 0.0, 0.0], [0.25, 0.25]);
        body.step(&level, &Physics::ITEM, &mut || 0.5);
        assert!((body.at[0] - 0.875).abs() < 1e-6 && body.motion[0] == 0.0, "stopped at the wall: {body:?}");
        let mut sliding = Body::new([-2.5, 65.0, 0.5], [0.2, 0.0, 0.0], [0.25, 0.25]);
        sliding.step(&level, &Physics::ITEM, &mut || 0.5);
        assert!(sliding.on_ground && (sliding.motion[0] - 0.2 * 0.588).abs() < 1e-6, "{sliding:?}");
    }
}
