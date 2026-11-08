//! A sneaker with no shield shows the shield's bits all the same. BDS's `ServerPlayer::normalTick`
//! (14074efc0) sets `FlagsExtended` bit 8 (`RAISED`) on a sneaker that neither uses an item nor swims,
//! whatever it holds, and bit 9 (`MOVING`) as bit 8 comes on; only a held shield's own tick clears bit
//! 9, so with none it stays through the stand-up (research/captures/bds-bg2player.txt, bds-ar2sneak.txt).
// TODO: BDS also holds bit 8 off through an arm swing and on scaffolding.

use bevy_ecs::prelude::*;

use super::{Guard, MOVING, RAISED, extended};
use crate::movement::Motion;

/// Shows a player's bits where its sneak changed them and no shield is up or coming down; whether it did.
pub(super) fn tell(world: &mut World, player: Entity, id: u64, now: u64, holds: bool, alive: bool) -> bool {
    let pose = world.get::<Motion>(player).map(|motion| motion.pose).unwrap_or_default();
    let Some(mut guard) = world.get_mut::<Guard>(player) else { return false };
    if holds && guard.lingers && !guard.bare && guard.raised.is_none() && now > guard.moving_until {
        // A shield come to hand clears the lingering bit 9 (research/captures/bds-f14csneak.txt `replaceitem`).
        guard.lingers = false;
        crate::movement::after_pose(world, player, extended(id, 0, now));
        return true;
    }
    if holds || guard.raised.is_some() || now <= guard.moving_until {
        guard.bare = false;
        return false;
    }
    let up = alive && pose.sneaking && !pose.using && !pose.swimming;
    if up == guard.bare {
        return false;
    }
    (guard.bare, guard.lingers) = (up, true);
    crate::movement::after_pose(world, player, extended(id, if up { RAISED | MOVING } else { MOVING }, now));
    true
}
