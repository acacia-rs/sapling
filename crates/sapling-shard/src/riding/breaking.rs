//! A wild mount lets its rider stay a while before it decides: BDS's `MountTamingSystem` counts
//! the ticks it is ridden and rolls its temper on the 60th, taming it or throwing the rider off
//! (fcn 14848d3c0; research/captures/bds-n3ride.txt: TameSuccess or TameFail 2.9 s on).

use bevy_ecs::prelude::*;

use super::Seats;
use crate::creatures;

/// Ridden this many ticks, the mount rolls.
const RIDDEN: u32 = 60;

/// A wild mount being ridden: for how many ticks.
#[derive(Component, Default)]
pub(super) struct Breaking(u32);

/// A rider got on the wild mount `mount`.
pub(super) fn start(world: &mut World, mount: Entity) {
    world.entity_mut(mount).insert(Breaking::default());
}

/// Each tick: a ridden wild mount counts on; on the last it is tamed by its driver or throws
/// every rider off. One no longer ridden forgets its count, as BDS's resets.
pub(crate) fn tick(world: &mut World) {
    let breaking: Vec<(Entity, Option<Entity>)> = world.query_filtered::<(Entity, Option<&Seats>), With<Breaking>>().iter(world).map(|(mount, seats)| (mount, seats.and_then(|seats| seats.0.first().copied()))).collect();
    for (mount, driver) in breaking {
        let Some(driver) = driver else {
            world.entity_mut(mount).remove::<Breaking>();
            continue;
        };
        let Some(mut count) = world.get_mut::<Breaking>(mount) else { continue };
        count.0 += 1;
        if count.0 < RIDDEN {
            continue;
        }
        world.entity_mut(mount).remove::<Breaking>();
        if !creatures::tame_by_riding(world, mount, driver) {
            super::evict(world, mount);
        }
    }
}
