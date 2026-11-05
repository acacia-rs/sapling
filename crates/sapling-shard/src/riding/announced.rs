//! What players who come to see a vehicle later learn of its riders: a mob rider's seat and link go
//! into the announcements (mobs/linked.rs), as BDS's `AddEntity`s carried them; a player rider's
//! seat is told after the vehicle.

use bevy_ecs::prelude::*;
use bytes::Bytes;

use super::packets::{self, LinkKind};
use super::{Seats, kind_of, layout, seats};
use crate::components::NetId;
use crate::entities::Kind;
use crate::mobs::{Linked, Mob};

/// Files `vehicle`'s riders' seats and links with their announcements; what a later viewer is told
/// after the vehicle besides: its player riders' seats.
pub(super) fn file(world: &mut World, vehicle: Entity) -> Vec<Bytes> {
    let riders = world.get::<Seats>(vehicle).map(|seats| seats.0.clone()).unwrap_or_default();
    let vehicle_id = world.get::<NetId>(vehicle).map_or(0, |id| id.0);
    let layout = world.get::<Kind>(vehicle).and_then(|kind| layout(&kind.0));
    let (mut links, mut told) = (Vec::new(), Vec::new());
    for (index, rider) in riders.iter().enumerate() {
        let (Some(id), Some(rider_kind)) = (world.get::<NetId>(*rider).map(|id| id.0), kind_of(world, *rider)) else { continue };
        let kind = if index == 0 { LinkKind::Driver } else { LinkKind::Passenger };
        links.push(packets::link_of(vehicle_id, id, kind, false));
        let Some(seat) = layout.and_then(|layout| layout.seat(riders.len(), index)) else { continue };
        let offset = seats::offset(&seat, &rider_kind, 0.0);
        if world.get::<Mob>(*rider).is_none() {
            told.push(packets::seat(id, &seat, offset, true));
            continue;
        }
        let mut linked = linked(world, *rider);
        linked.seat = packets::seat_items(&seat, offset, true);
        linked.rides = Some((vehicle, index == 0, packets::link(vehicle_id, id, kind, false)));
    }
    if world.get::<Mob>(vehicle).is_some() {
        linked(world, vehicle).links = links;
    }
    told
}

/// Forgets `rider`'s seat and link.
pub(super) fn unfile(world: &mut World, rider: Entity) {
    if let Some(mut linked) = world.get_mut::<Linked>(rider) {
        linked.seat.clear();
        linked.rides = None;
    }
}

fn linked(world: &mut World, entity: Entity) -> Mut<'_, Linked> {
    if world.get::<Linked>(entity).is_none() {
        world.entity_mut(entity).insert(Linked::default());
    }
    world.get_mut::<Linked>(entity).expect("just inserted")
}
