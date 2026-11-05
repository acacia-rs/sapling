//! Vehicles in a blast (docs/research/vehicles.md, research/captures/bds-c1blast.txt).

use bevy_ecs::prelude::*;

use super::{Kind, TNT_MINECART, Vehicle, carts, shake, wreck};
use crate::body::Body;
use crate::components::{InLayer, Tick};

/// A vehicle a blast broke, and the tick it goes in: BDS drops its item and takes it away the
/// tick after it showed the blast's harm.
#[derive(Component)]
pub(super) struct Wrecked(u64);

/// A blast in `layer` hurts each vehicle by what `harm_at` says it costs a body at its feet of its
/// size, as a hit does; a TNT minecart is primed instead, on a short fuse.
pub(crate) fn blasted(world: &mut World, layer: Entity, harm_at: impl Fn(&World, [f32; 3], [f32; 2]) -> Option<f32>) {
    let mut vehicles = world.query::<(Entity, &InLayer, &Body, &Kind, &Vehicle)>();
    let near: Vec<(Entity, bool, f32)> = vehicles
        .iter(world)
        .filter(|(_, in_layer, ..)| in_layer.0 == layer)
        .filter_map(|(vehicle, _, body, kind, _)| Some((vehicle, kind.0 == TNT_MINECART, harm_at(world, body.at, body.size)?)))
        .collect();
    let next = world.resource::<Tick>().0 + 1;
    for (vehicle, is_tnt, harm) in near {
        if is_tnt {
            let short = carts::SHORT_FUSE[0] + world.get_resource_or_insert_with(crate::blocks::Dice::default).roll(carts::SHORT_FUSE[1] as u32) as i32;
            carts::prime(world, vehicle, short);
        } else if shake(world, vehicle, harm).is_some_and(|left| left <= 0) {
            world.entity_mut(vehicle).insert(Wrecked(next));
        }
    }
}

/// Breaks the vehicles whose blast was the tick before.
pub(super) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let due: Vec<Entity> = world.query::<(Entity, &Wrecked)>().iter(world).filter(|(_, wrecked)| wrecked.0 <= now).map(|(vehicle, _)| vehicle).collect();
    for vehicle in due {
        wreck(world, vehicle, true);
    }
}
