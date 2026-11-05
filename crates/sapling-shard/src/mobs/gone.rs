//! A mob taken away and told gone at once, ahead of what this tick's [`super::update`] adds: BDS
//! removes a surviving eye of ender before its item appears (research/captures/bds-eye-throw.txt).

use acacia_proto::packets::RemoveEntity;
use bevy_ecs::prelude::*;

use super::SeesMobs;
use crate::components::Outgoing;

/// Removes `mob` and tells each player that has it so now.
pub(crate) fn remove_told(world: &mut World, mob: Entity) {
    let mut viewers = world.query::<(&mut SeesMobs, &mut Outgoing)>();
    for (mut sees, mut out) in viewers.iter_mut(world) {
        if let Some(id) = sees.mobs.remove(&mob) {
            out.push(&RemoveEntity { entity_id_self: id as i64 });
        }
    }
    super::remove(world, mob);
}
