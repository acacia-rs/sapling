//! Who rides what: players and mobs seated on vehicles (boats, minecarts) and on rideable mobs.

use crate::{EntityId, ShardCtx, Target};

/// How `/ride ... start_riding` brings a rider and its vehicle together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Mount {
    /// The rider is put on the vehicle where the vehicle is (`teleport_rider`).
    #[default]
    RiderToVehicle,
    /// The vehicle is brought to the rider first (`teleport_ride`).
    VehicleToRider,
}

impl ShardCtx<'_> {
    /// Seats `rider` on the entity `vehicle` in its first free seat, as `/ride start_riding` does: the
    /// vehicle must take riders of the rider's family and have a seat left, and the rider must not
    /// already ride it. A rider on another vehicle leaves that one first. False where it was not seated.
    pub fn ride(&mut self, rider: &Target, vehicle: EntityId, mount: Mount) -> bool {
        self.backend.ride(rider, vehicle, mount)
    }

    /// Takes `rider` off what it rides, onto the free spot beside it vanilla's would; false for one
    /// that rides nothing.
    pub fn dismount(&mut self, rider: &Target) -> bool {
        self.backend.dismount(rider)
    }

    /// Takes every rider off `vehicle`; how many there were.
    pub fn evict_riders(&mut self, vehicle: EntityId) -> usize {
        self.backend.evict_riders(vehicle)
    }

    /// Who rides `vehicle`, the driver's seat first; empty for one nobody rides or that is gone.
    pub fn riders(&self, vehicle: EntityId) -> Vec<Target> {
        self.backend.riders(vehicle)
    }

    /// What `rider` rides.
    pub fn vehicle_of(&self, rider: &Target) -> Option<EntityId> {
        self.backend.vehicle_of(rider)
    }

    /// How many more riders `vehicle` takes now: 0 for a full one or an entity nobody rides.
    pub fn seats_left(&self, vehicle: EntityId) -> usize {
        self.backend.seats_left(vehicle)
    }
}

/// The riding side of [`crate::Backend`].
pub trait RidingBackend {
    fn ride(&mut self, _rider: &Target, _vehicle: EntityId, _mount: Mount) -> bool {
        false
    }
    fn dismount(&mut self, _rider: &Target) -> bool {
        false
    }
    fn evict_riders(&mut self, _vehicle: EntityId) -> usize {
        0
    }
    fn riders(&self, _vehicle: EntityId) -> Vec<Target> {
        Vec::new()
    }
    fn vehicle_of(&self, _rider: &Target) -> Option<EntityId> {
        None
    }
    fn seats_left(&self, _vehicle: EntityId) -> usize {
        0
    }
}

#[cfg(test)]
impl RidingBackend for crate::testing::Fake {}
