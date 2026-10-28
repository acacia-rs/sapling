//! A ridden horse's jump, as BDS showed one (research/captures/bds-horse-jump.txt): its rider's
//! client makes the jump, by the horse's own `horse.jump_strength` (creatures/strides.rs), and
//! the horse rears (Flags bit 39) from the tick it rises until it lands, told to its rider and
//! its viewers.
// TODO: BDS cleared it the tick the horse landed; it goes here the tick after, when the
// reported height first stops falling.

use acacia_proto::types::MetadataFlags1;
use bevy_ecs::prelude::*;

use crate::entities;

/// Moves under this much a tick up or down count as none.
const STILL: f32 = 1e-4;

/// A ridden horse in a jump: whether it has started falling.
#[derive(Component)]
pub(super) struct Rearing {
    falling: bool,
}

/// After the horse `vehicle` (`id`, of `kind`) moved `rise` up as its driver's client said:
/// it rears as it leaves the ground and stops once landed. The flags to tell, if they changed.
pub(super) fn rear(world: &mut World, vehicle: Entity, id: u64, kind: &str, rise: f32) -> Option<bytes::Bytes> {
    let rearing = world.get::<Rearing>(vehicle).map(|rearing| rearing.falling);
    let on = match rearing {
        None if rise > STILL => true,
        Some(true) if rise > -STILL => false,
        Some(_) => {
            if rise < -STILL {
                world.entity_mut(vehicle).insert(Rearing { falling: true });
            }
            return None;
        }
        None => return None,
    };
    if on {
        world.entity_mut(vehicle).insert(Rearing { falling: false });
    } else {
        world.entity_mut(vehicle).remove::<Rearing>();
    }
    let word = entities::mob_flags(world, vehicle, kind, MetadataFlags1::REARING, on);
    Some(super::packets::flags(id, word))
}
