//! Vehicles hurt by `/damage` and struck by shots. BDS's boat and minecart took `/damage` as a hit
//! (bds-ffburn.txt: 2 fire took 20 off a boat, 16 off a cart); a TNT minecart is primed by fire and lava
//! instead. A shot meets a vehicle and goes: a creative player's breaks it, giving nothing, a tick on;
//! a flaming one primes a TNT minecart to burst at once (bds-fftnt.txt).

use bevy_ecs::prelude::*;
use sapling_api::DamageCause;

use super::{TNT_MINECART, Vehicle, carts, shake, wreck};
use crate::components::Tick;
use crate::entities::Kind;

/// A vehicle a shot broke in tick `when`: it goes in the next, leaving its item where `drops`.
#[derive(Component)]
pub(super) struct ShotDown {
    drops: bool,
    when: u64,
}

fn is_tnt(world: &World, vehicle: Entity) -> bool {
    world.get::<Kind>(vehicle).is_some_and(|kind| kind.0 == TNT_MINECART)
}

/// `/damage` of `amount` of `cause` on `vehicle`: whether it took it; `None` for no vehicle.
pub(crate) fn damaged(world: &mut World, vehicle: Entity, amount: f32, cause: DamageCause) -> Option<bool> {
    world.get::<Vehicle>(vehicle)?;
    if is_tnt(world, vehicle) && matches!(cause, DamageCause::Fire | DamageCause::FireTick | DamageCause::Lava) {
        carts::prime_burning(world, vehicle);
        return Some(true);
    }
    if shake(world, vehicle, amount)? <= 0 {
        wreck(world, vehicle, true);
    }
    Some(true)
}

/// A shot of `damage`, `flaming` or not, from a player in creative or not, struck `vehicle`.
pub(crate) fn shot(world: &mut World, vehicle: Entity, damage: f32, flaming: bool, creative: bool) {
    if flaming && is_tnt(world, vehicle) {
        carts::prime_lit(world, vehicle, 1);
        return;
    }
    let broken = creative || shake(world, vehicle, damage).is_some_and(|left| left <= 0);
    if broken {
        let when = world.resource::<Tick>().0;
        world.entity_mut(vehicle).insert(ShotDown { drops: !creative, when });
    }
}

/// Breaks the vehicles shots broke in an earlier tick.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let down: Vec<(Entity, bool)> = world.query::<(Entity, &ShotDown)>().iter(world).filter(|(_, down)| down.when < now).map(|(vehicle, down)| (vehicle, down.drops)).collect();
    for (vehicle, drops) in down {
        wreck(world, vehicle, drops);
    }
}
