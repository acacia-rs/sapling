//! How each kind of body falls and slows.

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
    /// A breeze, which keeps its speed in the air (research/captures/bds-breeze-findings.md).
    pub const BREEZE: Physics = Physics { air_drag: 1.0, fall_drag: 1.0, ..Physics::MOB };
    /// A ghast, which nothing pulls down and which keeps 0.91 of its speed every way
    /// (research/captures/bds-ghast-findings.md).
    pub const FLYER: Physics = Physics { gravity: 0.0, air_drag: 0.91, fall_drag: 0.91, ground_drag: 0.91, ..Physics::MOB };
}
