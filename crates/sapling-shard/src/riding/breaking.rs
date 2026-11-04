//! A wild mount lets its rider stay a while before it decides: BDS's `MountTamingSystem` counts
//! the ticks it is ridden and rolls its temper on the 60th, taming it or throwing the rider off
//! (fcn 14848d3c0; research/captures/bds-n3ride.txt: TameSuccess or TameFail 2.9 s on). A mount
//! tamed so shows its inventory alone the tick after, as BDS's did.

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::EntityProperties;
use bevy_ecs::prelude::*;

use super::Seats;
use crate::body::Body;
use crate::components::{InLayer, NetId, Tick};
use crate::creatures;
use crate::entities::{self, Kind};

/// Ridden this many ticks, the mount rolls.
const RIDDEN: u32 = 60;

/// A wild mount being ridden: for how many ticks.
#[derive(Component, Default)]
pub(super) struct Breaking(u32);

/// A mount tamed by its rider in this tick, its inventory not yet told.
#[derive(Component)]
struct Furnishing(u64);

/// A rider got on the wild mount `mount`.
pub(super) fn start(world: &mut World, mount: Entity) {
    world.entity_mut(mount).insert(Breaking::default());
}

/// Each tick: the mounts tamed last tick show their inventory; a ridden wild mount counts on, and
/// on the last it is tamed by its driver or throws every rider off. One no longer ridden forgets
/// its count, as BDS's resets.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    furnish(world, now);
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
        if creatures::tame_by_riding(world, mount, driver) {
            world.entity_mut(mount).insert(Furnishing(now));
        } else {
            super::evict(world, mount);
        }
    }
}

/// Tells the inventory of each mount tamed before tick `now`.
fn furnish(world: &mut World, now: u64) {
    let due: Vec<(Entity, String, u64, Entity, [f32; 3])> = world
        .query::<(Entity, &Furnishing, &Kind, &NetId, &InLayer, &Body)>()
        .iter(world)
        .filter(|(_, since, ..)| since.0 < now)
        .map(|(mount, _, kind, id, layer, body)| (mount, kind.0.clone(), id.0, layer.0, body.at))
        .collect();
    for (mount, kind, id, layer, at) in due {
        world.entity_mut(mount).remove::<Furnishing>();
        let metadata = super::tamed_inventory(&kind);
        if metadata.is_empty() {
            continue;
        }
        let data = SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
        entities::tell_viewers(world, mount, layer, at.map(|c| c.floor() as i32), vec![sapling_net::encode(&data)]);
    }
}
