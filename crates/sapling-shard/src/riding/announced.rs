//! What players who come to see a vehicle later learn of its riders: each rider's seat and link go
//! into its own announcement (mobs/linked.rs, visibility.rs), as BDS's `AddEntity`s and
//! `AddPlayer`s carried them (research/captures/bds-m3rideview.txt: `AddPlayer` with the seat,
//! its link, then the boat).

use bevy_ecs::prelude::*;

use super::packets::{self, LinkKind};
use super::{Seats, kind_of, layout};
use crate::components::NetId;
use crate::mobs::{Linked, Mob};

/// Files `vehicle`'s riders' seats and links with their announcements.
pub(super) fn file(world: &mut World, vehicle: Entity) {
    let riders = world.get::<Seats>(vehicle).map(|seats| seats.0.clone()).unwrap_or_default();
    let vehicle_id = world.get::<NetId>(vehicle).map_or(0, |id| id.0);
    let layout = kind_of(world, vehicle).and_then(|kind| layout(&kind));
    let mut links = Vec::new();
    for (index, rider) in riders.iter().enumerate() {
        let (Some(id), Some(_)) = (world.get::<NetId>(*rider).map(|id| id.0), kind_of(world, *rider)) else { continue };
        let kind = if index == 0 { LinkKind::Driver } else { LinkKind::Passenger };
        links.push(packets::link_of(vehicle_id, id, kind, false));
        let Some(seat) = layout.and_then(|layout| layout.seat(riders.len(), index)) else { continue };
        let Some(offset) = super::rider_offset(world, &seat, *rider, 0.0) else { continue };
        let mut linked = linked(world, *rider);
        linked.seat = packets::seat_items(&seat, offset, true);
        linked.rides = Some((vehicle, index == 0, packets::link(vehicle_id, id, kind, false)));
    }
    if world.get::<Mob>(vehicle).is_some() {
        linked(world, vehicle).links = links;
    }
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
