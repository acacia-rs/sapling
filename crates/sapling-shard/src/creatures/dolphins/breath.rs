//! A dolphin's breath and breaches. Its `Air` (4800, health/breath.rs) runs down under water;
//! at under 150 (`SwimUpForBreathGoal`, Linux canUse 0xbdcfe90) it shows `SWIMMING` and swims
//! up at up to 0.2 a tick, as BDS's four in `f14edolphin` did at `Air` 148
//! (research/captures/bds-f14edolphin.txt; under a cap it stayed pressed to it and drowned). With its
//! eyes [`UP_THE_TOP`] up the top water cell it breaches: `SPRINTING` (never cleared after) and 0.4 along the
//! way it moves most, on top of its motion, then 0.62 up the tick after, `SWIMMING` cleared
//! (bds-dolphin.txt 51885: told (-0.0968, 0, -0.4422), then moved (-0.098, 0.62, -0.45)). Out of
//! water its head draws its whole `Air` back at once. At random (`random_breach`, interval 50,
//! 2 s apart) it breaches from the top water cell where the 6 cells it moves towards are open
//! water too, as the Java game's dolphin jumps; BDS's breached so once in a chase at the surface.

use acacia_proto::types::MetadataFlags1;
use bevy_ecs::prelude::*;

use super::{Dolphin, show_flag, swim};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::Tick;
use crate::creatures::Health;

const LOW: i16 = 150;
/// Its push up while it swims for breath: 0.2 a tick at most in water's 0.8.
const RISE: f32 = 0.04;
const EYES: f32 = 0.51;
/// It breaches once its eyes are this far up the top water cell: BDS's breached with its eyes 0.48 up
/// it (bds-dolphin.txt 51885) and breathed 2 ticks on, its eyes out after the leap's first tick.
const UP_THE_TOP: f32 = 0.4;
const BOUND: f32 = 0.4;
const LEAP: f32 = 0.62;
const EVERY: u32 = 50;
const AHEAD: i32 = 6;
const RESTS: u64 = 40;

#[derive(Clone, Copy)]
pub(super) struct Breach {
    at: u64,
    speed: [f32; 2],
}

/// The breach's second tick, begun the tick before: it leaps. Whether it was.
pub(super) fn breaching(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    let now = world.resource::<Tick>().0;
    let Some(Breach { at, speed: [x, z] }) = world.get::<Dolphin>(entity).and_then(|dolphin| dolphin.breach) else { return false };
    if let Some(mut dolphin) = world.get_mut::<Dolphin>(entity) {
        dolphin.breach = None;
    }
    if now != at + 1 {
        return false;
    }
    if let Some(mut moving) = world.get_mut::<Body>(entity) {
        moving.throw([x, LEAP, z]);
    }
    show_flag(world, entity, layer, body, MetadataFlags1::SWIMMING, false);
    true
}

/// Its breach begins: [`BOUND`] along the way it moves most, on top of its motion.
fn bound(world: &mut World, entity: Entity, layer: Entity, body: &Body) {
    let now = world.resource::<Tick>().0;
    let [mx, _, mz] = body.motion;
    let speed = if mx.abs() > mz.abs() { [mx + BOUND * mx.signum(), mz] } else { [mx, mz + BOUND * mz.signum()] };
    if let Some(mut moving) = world.get_mut::<Body>(entity) {
        moving.throw([speed[0], 0.0, speed[1]]);
    }
    if let Some(mut dolphin) = world.get_mut::<Dolphin>(entity) {
        (dolphin.breach, dolphin.breached, dolphin.rising) = (Some(Breach { at: now, speed }), Some(now), false);
    }
    show_flag(world, entity, layer, body, MetadataFlags1::SPRINTING, true);
}

/// Short of breath it swims up, and breaches once its eyes reach the top water cell. Whether it did.
pub(super) fn swim_up(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    let (Some(air), Some(rising)) = (world.get::<Health>(entity).map(Health::air), world.get::<Dolphin>(entity).map(|dolphin| dolphin.rising)) else { return false };
    if air >= LOW {
        if rising {
            if let Some(mut dolphin) = world.get_mut::<Dolphin>(entity) {
                dolphin.rising = false;
            }
            show_flag(world, entity, layer, body, MetadataFlags1::SWIMMING, false);
        }
        return false;
    }
    let eyes = [body.at[0], body.at[1] + EYES, body.at[2]];
    if swim::is_top(world, layer, eyes.map(|c| c.floor() as i32)) && eyes[1] - eyes[1].floor() >= UP_THE_TOP {
        bound(world, entity, layer, body);
        return true;
    }
    if !rising {
        if let Some(mut dolphin) = world.get_mut::<Dolphin>(entity) {
            dolphin.rising = true;
        }
        show_flag(world, entity, layer, body, MetadataFlags1::SWIMMING, true);
    }
    let yaw = world.get::<crate::creatures::legs::Legs>(entity).map_or(0.0, |legs| legs.yaw);
    swim::push(world, entity, yaw, 0.0, RISE);
    true
}

/// A breach at random from the top water cell, the way ahead open. Whether it began one.
pub(super) fn leap(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    let now = world.resource::<Tick>().0;
    if world.get::<Dolphin>(entity).and_then(|dolphin| dolphin.breached).is_some_and(|last| now < last + RESTS) {
        return false;
    }
    let [x, y, z] = body.at.map(|c| c.floor() as i32);
    if !swim::is_top(world, layer, [x, y, z]) || world.get_resource_or_insert_with(Dice::default).roll(EVERY) != 0 {
        return false;
    }
    let [mx, _, mz] = body.motion;
    let way = if mx.abs() > mz.abs() { [mx.signum() as i32, 0] } else { [0, mz.signum() as i32] };
    if (1..=AHEAD).all(|step| swim::is_top(world, layer, [x + way[0] * step, y, z + way[1] * step])) {
        bound(world, entity, layer, body);
        return true;
    }
    false
}
