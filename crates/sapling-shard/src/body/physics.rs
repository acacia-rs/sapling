//! How each kind of body falls and slows.

use acacia_physics::constants::{DEFAULT_AIR_FRICTION, DEFAULT_BLOCK_FRICTION, DEFAULT_GROUND_DRAG, NORMAL_GRAVITY, NORMAL_GRAVITY_MULTIPLIER};

use super::InLiquid;

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
    /// Whether it goes through blocks.
    pub passes: bool,
    /// What a swimmer keeps of its speed every way in water (`InLiquid::Swims`).
    pub water_drag: f32,
}

/// Water's drag on a swimmer, BDS's `minecraft:water_movement` default (research/captures/bds-dolphin-findings.md).
const SWIM_DRAG: f32 = 0.8;

impl Physics {
    /// Experience orbs, which bounce (BDS `ExperienceOrb::normalTick`), and float.
    pub const ORB: Physics = Physics { gravity: 0.04, air_drag: 0.98, fall_drag: 0.98, ground_drag: DEFAULT_BLOCK_FRICTION * 0.98, bounce: -0.5, liquid: InLiquid::Buoyant, burns: false, passes: false, water_drag: SWIM_DRAG };
    /// Items, which do not (BDS `ItemActor::postNormalTick`, docs/research/items.md).
    pub const ITEM: Physics = Physics { gravity: 0.04, air_drag: 0.98, fall_drag: 0.98, ground_drag: DEFAULT_BLOCK_FRICTION * 0.98, bounce: 0.0, liquid: InLiquid::Floats, burns: true, passes: false, water_drag: SWIM_DRAG };
    pub const FIREPROOF_ITEM: Physics = Physics { burns: false, ..Physics::ITEM };
    /// A falling block, which stops where it lands (BDS `FallingBlockActor::normalTick`).
    pub const FALLING_BLOCK: Physics = Physics { gravity: 0.04, air_drag: 0.98, fall_drag: 0.98, ground_drag: 0.0, bounce: 0.0, liquid: InLiquid::Ignores, burns: false, passes: false, water_drag: SWIM_DRAG };
    /// Primed TNT, which slides on where it lands (research/captures/bds-tnt.txt).
    pub const TNT: Physics = Physics { ground_drag: 0.686, ..Physics::FALLING_BLOCK };
    /// A mob that does not walk, as BDS moved an armour stand: its speed is worked out before the
    /// move (research/captures/bds-water.txt). The drag across, 0.91 in the air and 0.6 x 0.91 on
    /// the ground, is the Java game's (`LivingEntity.travel`).
    /// What lava and fire do to one: creatures/harm.rs.
    pub const MOB: Physics = Physics { gravity: NORMAL_GRAVITY, air_drag: DEFAULT_AIR_FRICTION, fall_drag: NORMAL_GRAVITY_MULTIPLIER, ground_drag: DEFAULT_GROUND_DRAG, bounce: 0.0, liquid: InLiquid::Sinks, burns: false, passes: false, water_drag: SWIM_DRAG };
    /// A breeze, which keeps its speed in the air (research/captures/bds-breeze-findings.md).
    pub const BREEZE: Physics = Physics { air_drag: 1.0, fall_drag: 1.0, ..Physics::MOB };
    /// A ghast, which nothing pulls down and which keeps 0.91 of its speed every way
    /// (research/captures/bds-ghast-findings.md).
    pub const FLYER: Physics = Physics { gravity: 0.0, air_drag: 0.91, fall_drag: 0.91, ground_drag: 0.91, ..Physics::MOB };
    /// A dolphin, which swims in water and is a mob out of it (research/captures/bds-dolphin-findings.md).
    pub const SWIMMER: Physics = Physics { liquid: InLiquid::Swims, ..Physics::MOB };
    /// A vex, which flies as a ghast does and through blocks (research/captures/bds-evoker-findings.md).
    pub const GHOST: Physics = Physics { passes: true, ..Physics::FLYER };
    /// A turtle, whose `water_movement` keeps 0.9 (research/captures/bds-wander-findings.md).
    pub const TURTLE: Physics = Physics { water_drag: 0.9, ..Physics::SWIMMER };
    /// A bat, which nothing pulls down, keeps 0.91 of its speed across and 0.6 of it up or down
    /// (research/captures/bds-wander-findings.md).
    pub const BAT: Physics = Physics { fall_drag: 0.6, ..Physics::FLYER };
    /// An allay, which keeps 0.8645 of its speed every way (bds-wander-findings.md).
    pub const HOVER: Physics = Physics { air_drag: 0.8645, fall_drag: 0.8645, ground_drag: 0.8645, ..Physics::FLYER };
    /// A parrot in flight keeps 0.92 of its speed every way; its fall is its wings' (creatures/wings.rs).
    pub const PARROT: Physics = Physics { air_drag: 0.92, fall_drag: 0.92, ..Physics::FLYER };
}
