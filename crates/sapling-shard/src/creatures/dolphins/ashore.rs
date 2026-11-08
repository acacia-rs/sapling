//! A dolphin out of water, as BDS's on the walled tray of `f14edolphin`
//! (research/captures/bds-f14edolphin.txt): a tick after it came to rest it hops 0.4216 up (0.5 on
//! the fall of a tick on the ground) and up to 0.2 across, falling as `Physics::DOLPHIN`. Out of
//! water 2400 ticks (`drying_out_timer`'s 120 s) it dries out, hurt 1 a hit with no cause
//! (`dolphin_dried`'s `damage_over_time`; the hit's guard spaces them 10 ticks), until water or
//! rain wets it again.
// TODO: BDS also turns it to a random yaw each hop; its first dry hit came 2401 ticks after it
// landed and 2407 after it was summoned in the air, so where the count starts is not pinned.

use bevy_ecs::prelude::*;

use super::Dolphin;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::Tick;

const HOP_UP: f32 = 0.4216;
const HOP_ACROSS: f32 = 0.2;
const DRIES: u64 = 2400;

/// Out of water: a dolphin a tick on the ground hops.
pub(super) fn flop(world: &mut World, entity: Entity, body: &Body) {
    let now = world.resource::<Tick>().0;
    let Some(mut dolphin) = world.get_mut::<Dolphin>(entity) else { return };
    if !body.on_ground {
        dolphin.landed = None;
        return;
    }
    match dolphin.landed {
        None => dolphin.landed = Some(now),
        Some(landed) if now > landed => {
            dolphin.landed = None;
            let mut dice = world.get_resource_or_insert_with(Dice::default);
            let mut across = || (dice.fraction() * 2.0 - 1.0) * HOP_ACROSS;
            let hop = [body.motion[0] + across(), HOP_UP, body.motion[2] + across()];
            if let Some(mut body) = world.get_mut::<Body>(entity) {
                body.throw(hop);
            }
            super::super::swimming::heard_flop(world, entity, body);
        }
        Some(_) => {}
    }
}

/// A tick of the dolphin, `wet` or not: whether it is dried out.
pub(in crate::creatures) fn dried(world: &mut World, entity: Entity, wet: bool) -> bool {
    let now = world.resource::<Tick>().0;
    let Some(mut dolphin) = world.get_mut::<Dolphin>(entity) else { return false };
    dolphin.ashore = if wet { None } else { Some(dolphin.ashore.unwrap_or(now)) };
    dolphin.ashore.is_some_and(|since| now >= since + DRIES)
}
