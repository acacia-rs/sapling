//! Boats, minecarts and camels pull in the mobs that bump into them, while a seat is free
//! (`pull_in_entities`; docs/research/vehicles.md "Pull-in"); BDS's boat took a zombie it had
//! thrown out straight back (research/captures/bds-ride.txt). Players are never pulled in.

use bevy_ecs::prelude::*;

use super::{BOAT, CHEST_BOAT};
use crate::body::Body;
use crate::components::InLayer;
use crate::creatures::Creature;
use crate::entities::Kind;
use crate::riding::Riding;

/// Mobs this wide or wider do not fit in a boat.
const BOAT_WIDTH: f32 = 1.375;

/// Seats the mobs touching each vehicle with a free seat.
pub(super) fn tick(world: &mut World) {
    let vehicles: Vec<(Entity, Entity, String, Body)> = world
        .query::<(Entity, &InLayer, &Kind, &Body)>()
        .iter(world)
        .filter(|(_, _, kind, _)| crate::riding::layout(&kind.0).is_some_and(|layout| layout.pulls_in))
        .map(|(vehicle, layer, kind, body)| (vehicle, layer.0, kind.0.clone(), *body))
        .collect();
    let vehicles: Vec<_> = vehicles.into_iter().filter(|(vehicle, ..)| crate::riding::seats_left(world, *vehicle) > 0).collect();
    if vehicles.is_empty() {
        return;
    }
    let mobs: Vec<(Entity, Entity, Body)> = world.query_filtered::<(Entity, &InLayer, &Body), (With<Creature>, Without<Riding>)>().iter(world).map(|(mob, layer, body)| (mob, layer.0, *body)).collect();
    for (vehicle, layer, kind, body) in vehicles {
        let (low, high) = body.bounds();
        let touching = mobs.iter().filter(|(mob, mob_layer, mob_body)| {
            let (other_low, other_high) = mob_body.bounds();
            let fits = !(kind == BOAT || kind == CHEST_BOAT) || mob_body.size[0] < BOAT_WIDTH;
            *mob != vehicle && *mob_layer == layer && fits && (0..3).all(|axis| low[axis] < other_high[axis] && high[axis] > other_low[axis])
        });
        for (mob, ..) in touching {
            if world.get::<Riding>(*mob).is_none() && crate::riding::seat(world, *mob, vehicle).is_err() && crate::riding::seats_left(world, vehicle) == 0 {
                break;
            }
        }
    }
}
