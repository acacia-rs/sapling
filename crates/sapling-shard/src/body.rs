//! A body the server alone moves (falling blocks, items, orbs, mobs), as BDS moves such actors: gravity,
//! then the move, cut short by the collision boxes of blocks, then drag
//! (docs/research/falling-blocks.md, experience.md, items.md), and water and lava (liquid.rs).
//! What viewers are told: sync.rs.
// TODO: only items are lifted out of a block they are in, and only up.

mod liquid;

use bevy_ecs::prelude::Component;
pub(crate) use liquid::{InLiquid, Liquid, in_liquid};
use sapling_world::World as Level;

const ARM: &str = "piston_arm_collision";
/// The speed a mob's jump starts up at.
const JUMP: f32 = 0.42;
/// Room left between boxes that touch, against float error.
const EPS: f32 = 1e-5;

/// How a kind of body falls and slows.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Physics {
    pub gravity: f32,
    /// Horizontal motion kept from one tick to the next in the air.
    pub air_drag: f32,
    /// Vertical motion kept from one tick to the next.
    pub fall_drag: f32,
    /// Horizontal motion kept on the ground: a normal block's friction 0.6 x 0.98.
    pub ground_drag: f32,
    /// What the vertical motion is multiplied by on the ground.
    pub bounce: f32,
    pub liquid: InLiquid,
    /// Whether lava sets it on fire.
    pub burns: bool,
}

impl Physics {
    /// Experience orbs, which bounce (BDS `ExperienceOrb::normalTick`), and float.
    pub const ORB: Physics = Physics { gravity: 0.04, air_drag: 0.98, fall_drag: 0.98, ground_drag: 0.588, bounce: -0.5, liquid: InLiquid::Buoyant, burns: false };
    /// Items, which do not (BDS `ItemActor::postNormalTick`, docs/research/items.md).
    pub const ITEM: Physics = Physics { gravity: 0.04, air_drag: 0.98, fall_drag: 0.98, ground_drag: 0.588, bounce: 0.0, liquid: InLiquid::Floats, burns: true };
    pub const FIREPROOF_ITEM: Physics = Physics { burns: false, ..Physics::ITEM };
    /// A falling block, which stops where it lands (BDS `FallingBlockActor::normalTick`).
    pub const FALLING_BLOCK: Physics = Physics { gravity: 0.04, air_drag: 0.98, fall_drag: 0.98, ground_drag: 0.0, bounce: 0.0, liquid: InLiquid::Ignores, burns: false };
    /// Primed TNT, which slides on where it lands (research/captures/bds-tnt.txt).
    pub const TNT: Physics = Physics { ground_drag: 0.686, ..Physics::FALLING_BLOCK };
    /// A mob that does not walk, as BDS moved an armour stand: its speed is worked out before the
    /// move (research/captures/bds-water.txt). The drag across, 0.91 in the air and 0.6 x 0.91 on
    /// the ground, is the Java game's (`LivingEntity.travel`).
    /// What lava and fire do to one: creatures/harm.rs.
    pub const MOB: Physics = Physics { gravity: 0.08, air_drag: 0.91, fall_drag: 0.98, ground_drag: 0.546, bounce: 0.0, liquid: InLiquid::Sinks, burns: false };
}

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
    /// The speed a hit throws a mob at; its next tick is that move alone.
    thrown: Option<[f32; 3]>,
    /// The speed across its own legs add in its next tick.
    walking: [f32; 2],
    leaping: bool,
    /// Whether a mob was on the ground before its last move: that move's drag goes by it.
    stood: bool,
}

/// A block's collision box in world coordinates.
type Bounds = ([f32; 3], [f32; 3]);

impl Body {
    pub fn new(at: [f32; 3], motion: [f32; 3], size: [f32; 2]) -> Self {
        Self { at, motion, size, on_ground: false, liquid: None, wave: 0.0, thrown: None, walking: [0.0; 2], leaping: false, stood: false }
    }

    /// Throws a mob at `speed`: its next tick moves it by that, and the tick after slows it as
    /// one that left the ground it is on now (research/captures/bds-hit.txt).
    pub fn throw(&mut self, speed: [f32; 3]) {
        self.thrown = Some(speed);
    }

    /// Has a mob walk: its next tick adds `speed` across, after the drag, and with `leap` goes
    /// up at a jump's speed (research/captures/bds-bite.txt, bds-chase.txt).
    pub fn walk(&mut self, speed: [f32; 2], leap: bool) {
        (self.walking, self.leaping) = (speed, leap);
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

    /// Lifts a body whose box is in a block onto that block's top, stopped, as BDS did an item
    /// dropped 0.05 into the floor (research/captures/bds-kill.txt); the tick's step lands it.
    pub fn lift_out(&mut self, level: &Level) {
        let (min, max) = self.bounds();
        let overlaps = |(low, high): &Bounds| (0..3).all(|axis| min[axis] < high[axis] - EPS && max[axis] > low[axis] + EPS);
        if let Some(top) = boxes_in(level, (min, max)).iter().filter(|other| overlaps(other)).map(|(_, high)| high[1]).reduce(f32::max) {
            (self.at[1], self.motion) = (top, [0.0; 3]);
        }
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
            (InLiquid::Sinks, _) => return self.mob_step(level, physics, before),
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
            self.on_ground = self.slide(level, speed);
            return speed;
        }
        if let Some(liquid) = liquid {
            liquid::sink(self, liquid);
        } else {
            // BDS slows a mob after its move by where it stood before it (research/captures/bds-chase.txt).
            let across = if stood { physics.ground_drag } else { physics.air_drag };
            (self.motion[0], self.motion[2]) = (self.motion[0] * across, self.motion[2] * across);
            self.motion[1] = (self.motion[1] - physics.gravity) * physics.fall_drag;
        }
        (self.motion[0], self.motion[2]) = (self.motion[0] + walking[0], self.motion[2] + walking[1]);
        if leaping {
            self.motion[1] = JUMP;
        }
        let wanted = self.motion;
        self.on_ground = self.slide(level, wanted);
        if self.on_ground {
            self.motion[1] = 0.0;
        }
        wanted
    }

    /// Moves by `motion` as far as the blocks let it, an axis at a time (up and down first, as the
    /// game does); motion into a block is stopped, except downward, which the bounce answers.
    /// Whether it came down onto something.
    fn slide(&mut self, level: &Level, motion: [f32; 3]) -> bool {
        let (min, max) = self.bounds();
        let reach = (std::array::from_fn(|axis| min[axis] + motion[axis].min(0.0)), std::array::from_fn(|axis| max[axis] + motion[axis].max(0.0)));
        let boxes = boxes_in(level, reach);
        let mut landed = false;
        for axis in [1, 0, 2] {
            let (min, max) = self.bounds();
            let Some(face) = boxes.iter().filter_map(|other| stop(axis, motion[axis], (min, max), *other)).reduce(|a, b| if motion[axis] < 0.0 { a.max(b) } else { a.min(b) }) else {
                self.at[axis] += motion[axis];
                continue;
            };
            // Snapped onto the face it met, so that a body at rest is exactly on it.
            let (low, high) = if axis == 1 { (0.0, self.size[1]) } else { (self.size[0] / 2.0, self.size[0] / 2.0) };
            self.at[axis] = if motion[axis] < 0.0 { face + low } else { face - high };
            landed |= axis == 1 && motion[axis] < 0.0;
            if axis != 1 || motion[axis] > 0.0 {
                self.motion[axis] = 0.0;
            }
        }
        landed
    }
}

/// Where a body moving `delta` along `axis` meets `other`: the face it stops at, if in its way.
fn stop(axis: usize, delta: f32, (min, max): Bounds, (other_min, other_max): Bounds) -> Option<f32> {
    let across = (0..3).filter(|other| *other != axis).all(|o| min[o] < other_max[o] - EPS && max[o] > other_min[o] + EPS);
    match () {
        _ if !across || delta == 0.0 => None,
        _ if delta < 0.0 => (other_max[axis] <= min[axis] + EPS && min[axis] + delta < other_max[axis]).then_some(other_max[axis]),
        _ => (other_min[axis] >= max[axis] - EPS && max[axis] + delta > other_min[axis]).then_some(other_min[axis]),
    }
}

/// The collision boxes of the blocks in reach; a fence's reach 1.5 high is why a cell below counts.
fn boxes_in(level: &Level, (min, max): Bounds) -> Vec<Bounds> {
    let span = |axis: usize, below: i32| (min[axis].floor() as i32 - below)..=(max[axis].floor() as i32);
    let mut boxes = Vec::new();
    for x in span(0, 0) {
        for y in span(1, 1) {
            for z in span(2, 0) {
                if !level.in_height(y) || !level.is_loaded(sapling_world::ChunkPos::of_block(x, z)) {
                    continue;
                }
                let Some(state) = level.registry().get(level.block(x, y, z)) else { continue };
                let cell = [x as f32, y as f32, z as f32];
                boxes.extend(state.boxes.iter().map(|b| (std::array::from_fn(|i| cell[i] + b.min[i]), std::array::from_fn(|i| cell[i] + b.max[i]))));
                // The block table gives a piston's arm no box; BDS's stand stood on one (bds-shove.txt).
                if state.boxes.is_empty() && state.name.ends_with(ARM) {
                    boxes.push((cell, cell.map(|c| c + 1.0)));
                }
            }
        }
    }
    boxes
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
