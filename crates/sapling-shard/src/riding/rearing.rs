//! A ridden horse's jump, as BDS showed one (research/captures/bds-horse-jump.txt): its rider's
//! client makes the jump, by the horse's own `horse.jump_strength` (creatures/strides.rs), and
//! the horse rears (Flags bit 39) from the tick it rises until it lands, told to its rider and
//! its viewers, and cleared the tick it lands.

use acacia_proto::types::MetadataFlags1;
use bevy_ecs::prelude::*;

use crate::entities;

/// Moves under this much a tick up or down count as none.
const STILL: f32 = 1e-4;
/// A falling horse whose feet are within this of a block's top has landed.
const GROUND: f32 = 1e-3;

/// A ridden horse in a jump: whether it has started falling.
#[derive(Component)]
pub(super) struct Rearing {
    falling: bool,
}

/// Whether a body of `width` with its feet at `feet` stands on a block's top.
pub(super) fn grounded(level: &sapling_world::World, [x, y, z]: [f32; 3], width: f32) -> bool {
    let half = width / 2.0;
    let under = acacia_physics::Aabb { min: [x - half, y - GROUND, z - half], max: [x + half, y - GROUND / 10.0, z + half] };
    crate::terrain::physics_view(level, |view| {
        let mut hits = Vec::new();
        acacia_physics::WorldView::collisions(view, &under, &mut hits);
        !hits.is_empty()
    })
}

/// After the horse `vehicle` (`id`, of `kind`) moved `rise` up as its driver's client said,
/// `landed` on a block or not: it rears as it leaves the ground and stops the tick it lands
/// (research/captures/bds-horse-jump.txt). The flags to tell, if they changed.
pub(super) fn rear(world: &mut World, vehicle: Entity, id: u64, kind: &str, rise: f32, landed: bool) -> Option<bytes::Bytes> {
    let rearing = world.get::<Rearing>(vehicle).map(|rearing| rearing.falling);
    let on = match rearing {
        None if rise > STILL => true,
        Some(true) if rise > -STILL || landed => false,
        Some(false) if rise < -STILL && landed => false,
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
