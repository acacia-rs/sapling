//! What a change of riders tells: every rider's link, and the seats that moved.

use bevy_ecs::prelude::*;
use bytes::Bytes;

use super::packets::{self, LinkKind};
use super::{Seats, kind_of, layout};
use crate::components::NetId;

/// Whose seat a change of riders tells: the newcomer's, and those that moved.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Change {
    /// One just seated, with all of BDS's items; the others' moved seats as positions alone.
    Newcomer(Entity),
    /// The rider at this index left: the others' moved seats as positions alone.
    Left(usize),
}

/// Every rider of `vehicle`'s link, the first as its driver.
pub(super) fn links(world: &World, vehicle: Entity) -> Vec<Bytes> {
    let vehicle_id = world.get::<NetId>(vehicle).map_or(0, |id| id.0);
    let riders = world.get::<Seats>(vehicle).map(|seats| seats.0.clone()).unwrap_or_default();
    let kind = |index: usize| if index == 0 { LinkKind::Driver } else { LinkKind::Passenger };
    riders.iter().enumerate().filter_map(|(index, rider)| world.get::<NetId>(*rider).map(|id| packets::link(vehicle_id, id.0, kind(index), false))).collect()
}

/// The seats of `vehicle`'s riders (their wire positions in the vehicle's frame): BDS told none
/// that stayed put (a happy ghast's, a player's shoulders: bds-as3ghast.txt, bds-as3parrot.txt).
pub(super) fn seat_packets(world: &World, vehicle: Entity, change: Change) -> Vec<Bytes> {
    let (Some(kind), Some(riders)) = (kind_of(world, vehicle), world.get::<Seats>(vehicle)) else { return Vec::new() };
    let Some(layout) = layout(&kind) else { return Vec::new() };
    let count = riders.0.len();
    let mut packets = Vec::new();
    for (index, rider) in riders.0.iter().enumerate() {
        let Some((seat, id)) = layout.seat(count, index).zip(world.get::<NetId>(*rider)) else { continue };
        let Some(offset) = super::rider_offset(world, &seat, *rider, 0.0) else { continue };
        let whole = change == Change::Newcomer(*rider);
        let before = match change {
            Change::Newcomer(_) => (count - 1, index),
            Change::Left(left) => (count + 1, if index >= left { index + 1 } else { index }),
        };
        if whole || layout.moved(count, index, before) {
            packets.push(packets::seat(id.0, &seat, offset, whole));
        }
    }
    packets
}
