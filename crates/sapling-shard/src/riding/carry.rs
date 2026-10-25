//! Riders kept in their seats: after vehicles moved, each rider is put at its seat, turned with
//! the vehicle. A mob rider's viewers are told its move; a player's client seats itself, and its
//! viewers learn of it from its MovePlayer (movement.rs), which names what it rides.

use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;

use super::{Riding, Seats, kind_of, layout, seats, vehicle_wire};
use crate::body::Body;
use crate::components::{InLayer, NetId, Position};
use crate::entities::{self, Kind};
use crate::sync::Told;

/// Puts every rider in its seat; a rider whose vehicle is gone (unloaded with its column) is let off.
pub(crate) fn carry(world: &mut World) {
    let stray: Vec<Entity> = world.query::<(Entity, &Riding)>().iter(world).filter(|(_, riding)| world.get::<Seats>(riding.vehicle).is_none()).map(|(rider, _)| rider).collect();
    for rider in stray {
        world.entity_mut(rider).remove::<(Riding, super::Reins)>();
    }
    let vehicles: Vec<Entity> = world.query_filtered::<Entity, With<Seats>>().iter(world).collect();
    for vehicle in vehicles {
        let riders = world.get::<Seats>(vehicle).map(|seats| seats.0.clone()).unwrap_or_default();
        let kept: Vec<Entity> = riders.iter().copied().filter(|rider| world.get::<Riding>(*rider).is_some_and(|riding| riding.vehicle == vehicle)).collect();
        if kept.is_empty() {
            world.entity_mut(vehicle).remove::<Seats>();
            continue;
        }
        if kept.len() != riders.len() {
            world.entity_mut(vehicle).insert(Seats(kept));
        }
        place(world, vehicle);
    }
}

/// Puts the riders of `vehicle` in their seats.
pub(super) fn place(world: &mut World, vehicle: Entity) {
    let (Some((wire, yaw)), Some(kind), Some(riders)) = (vehicle_wire(world, vehicle), world.get::<Kind>(vehicle).map(|kind| kind.0.clone()), world.get::<Seats>(vehicle).map(|seats| seats.0.clone())) else { return };
    let Some(layout) = layout(&kind) else { return };
    let (motion, on_ground) = world.get::<Body>(vehicle).map_or(([0.0; 3], false), |body| (body.motion, body.on_ground));
    for (index, rider) in riders.iter().enumerate() {
        let (Some(seat), Some(rider_kind)) = (layout.seat(riders.len(), index), kind_of(world, *rider)) else { continue };
        let offset = seats::offset(&seat, &rider_kind, yaw);
        let at: [f32; 3] = std::array::from_fn(|axis| wire[axis] + offset[axis]);
        if let Some(mut position) = world.get_mut::<Position>(*rider) {
            let feet = [at[0], at[1] - EYE_HEIGHT, at[2]];
            if position.feet != feet {
                (position.feet, position.on_ground) = (feet, on_ground);
            }
            continue;
        }
        carry_mob(world, *rider, at, motion, on_ground);
    }
}

/// Moves a mob rider to `at` (its feet: a mob's wire is its feet), telling its viewers.
fn carry_mob(world: &mut World, rider: Entity, at: [f32; 3], motion: [f32; 3], on_ground: bool) {
    let (Some(id), Some(layer)) = (world.get::<NetId>(rider).map(|id| id.0), world.get::<InLayer>(rider).map(|layer| layer.0)) else { return };
    let Some(mut body) = world.get_mut::<Body>(rider) else { return };
    (body.at, body.motion, body.on_ground) = (at, motion, on_ground);
    let Some(mut told) = world.get_mut::<Told>(rider) else { return };
    let packets = told.update(id, at, motion, on_ground);
    entities::tell_viewers(world, rider, layer, at.map(|c| c.floor() as i32), packets);
}
