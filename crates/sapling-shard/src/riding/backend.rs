//! The riding side of the handlers' context: plugins and `/ride` seat and unseat riders.

use bevy_ecs::prelude::*;
use sapling_api::{EntityId, Mount, RidingBackend, Target};

use super::{Riding, Seats, can_seat, evict, seat, seats_left, unseat};
use crate::body::Body;
use crate::components::{Identity, NetId, Position};
use crate::entities;
use crate::handlers::Ctx;
use crate::sync::Told;

impl Ctx<'_> {
    fn of_target(&self, target: &Target) -> Option<Entity> {
        match target {
            Target::Player(player) => self.entity(player.id),
            Target::Entity(id) => entities::find(self.world, *id),
            _ => None,
        }
    }

    fn as_target(&self, entity: Entity) -> Option<Target> {
        match self.world.get::<Identity>(entity) {
            Some(identity) => Some(Target::Player(identity.0.clone())),
            None => Some(Target::Entity(EntityId(self.world.get::<NetId>(entity)?.0))),
        }
    }
}

/// Brings `vehicle` to where `rider` stands, as `/ride ... teleport_ride` does.
fn bring(world: &mut World, vehicle: Entity, rider: Entity) {
    let Some(at) = world.get::<Position>(rider).map(|at| at.feet).or_else(|| world.get::<Body>(rider).map(|body| body.at)) else { return };
    if let Some(mut body) = world.get_mut::<Body>(vehicle) {
        (body.at, body.motion) = (at, [0.0; 3]);
    }
    if let Some(mut told) = world.get_mut::<Told>(vehicle) {
        told.jump();
    }
}

impl RidingBackend for Ctx<'_> {
    fn ride(&mut self, rider: &Target, vehicle: EntityId, mount: Mount) -> bool {
        let (Some(rider), Some(vehicle)) = (self.of_target(rider), entities::find(self.world, vehicle)) else { return false };
        if can_seat(self.world, rider, vehicle).is_err() {
            return false;
        }
        if mount == Mount::VehicleToRider {
            bring(self.world, vehicle, rider);
        }
        seat(self.world, rider, vehicle).is_ok()
    }

    fn dismount(&mut self, rider: &Target) -> bool {
        let Some(rider) = self.of_target(rider) else { return false };
        unseat(self.world, rider, true)
    }

    fn evict_riders(&mut self, vehicle: EntityId) -> usize {
        entities::find(self.world, vehicle).map_or(0, |vehicle| evict(self.world, vehicle))
    }

    fn riders(&self, vehicle: EntityId) -> Vec<Target> {
        let riders = entities::find(self.world, vehicle).and_then(|vehicle| self.world.get::<Seats>(vehicle)).map(|seats| seats.0.clone()).unwrap_or_default();
        riders.into_iter().filter_map(|rider| self.as_target(rider)).collect()
    }

    fn vehicle_of(&self, rider: &Target) -> Option<EntityId> {
        let riding = self.world.get::<Riding>(self.of_target(rider)?)?;
        Some(EntityId(riding.id))
    }

    fn seats_left(&self, vehicle: EntityId) -> usize {
        entities::find(self.world, vehicle).map_or(0, |vehicle| seats_left(self.world, vehicle))
    }
}
