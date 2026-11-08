//! A playful panda's or a baby panda's roll (`behavior.roll`, probability 0.013 a tick, a baby's
//! 0.0016), as BDS's rolled (research/captures/bds-g14panda.txt, bds-pandas-findings.md): it starts
//! with nothing else to do, `FlagsExtended` bit 3 on for 32 ticks. Its motion is set every tick to
//! 0.2 along the way it faced as it started (a baby's 0.1), and up 0.27 on the first; 7, 15 and 23
//! ticks in it stops across for the tick and leaps 0.27 again if on the ground. Slowness does not hold it.

use bevy_ecs::prelude::*;

use super::shown;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::Tick;
use crate::creatures::legs::{self, Legs};

const PANDA: &str = "minecraft:panda";
const CHANCE: f32 = 0.013;
const BABY_CHANCE: f32 = 0.0016;
const SPEED: f32 = 0.2;
const BABY_SPEED: f32 = 0.1;
const LEAP: f32 = 0.27;
const LASTS: u64 = 32;
const HOPS: [u64; 3] = [7, 15, 23];
const GRAVITY: f32 = 0.08;
const DRAG: f32 = 0.98;
/// `FlagsExtended` at rest, and the bit on while it rolls.
const REST: i64 = 1 << 25;
const ROLLING: i64 = 1 << 3;

#[derive(Component, Clone, Copy)]
pub(super) struct Rolling {
    since: u64,
    along: [f32; 2],
    rise: f32,
}

/// Before the move of a rolling panda: it rolls on, or stops after [`LASTS`]. False for one not rolling.
pub(super) fn keep_rolling(world: &mut World, entity: Entity) -> bool {
    let (Some(rolling), Some(body)) = (world.get::<Rolling>(entity).copied(), world.get::<Body>(entity).copied()) else { return false };
    let ticks = world.resource::<Tick>().0 - rolling.since;
    if ticks >= LASTS {
        world.entity_mut(entity).remove::<Rolling>();
        shown::flags_extended(world, entity, REST);
        return false;
    }
    let rise = if body.on_ground { if HOPS.contains(&ticks) { LEAP } else { 0.0 } } else { (rolling.rise - GRAVITY) * DRAG };
    let along = if HOPS.contains(&ticks) { [0.0; 2] } else { rolling.along };
    roll(world, entity, Rolling { rise, ..rolling }, along);
    true
}

/// Before the move of a panda with nothing else to do: a playful or baby one may start to roll.
pub(super) fn start(world: &mut World, entity: Entity, kind: &str) -> bool {
    let baby = crate::creatures::shown::is_baby(world, entity);
    let rolls = kind == PANDA && (baby || crate::creatures::genes::is_playful(world, entity));
    let grounded = world.get::<Body>(entity).is_some_and(|body| body.on_ground);
    if !rolls || !grounded || crate::creatures::strolling::is_fleeing(world, entity) {
        return false;
    }
    let chance = if baby { BABY_CHANCE } else { CHANCE };
    if world.get_resource_or_insert_with(Dice::default).fraction() >= chance {
        return false;
    }
    let yaw = world.get::<Legs>(entity).map_or(0.0, |legs| legs.yaw).to_radians();
    let speed = if baby { BABY_SPEED } else { SPEED };
    let rolling = Rolling { since: world.resource::<Tick>().0, along: [-yaw.sin() * speed, yaw.cos() * speed], rise: LEAP };
    shown::flags_extended(world, entity, REST | ROLLING);
    roll(world, entity, rolling, rolling.along);
    true
}

fn roll(world: &mut World, entity: Entity, rolling: Rolling, along: [f32; 2]) {
    if let Some(mut body) = world.get_mut::<Body>(entity) {
        body.fling([along[0], rolling.rise, along[1]]);
    }
    world.entity_mut(entity).insert(rolling);
    legs::rest(world, entity);
}
