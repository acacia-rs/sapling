//! Bundles' containers let go, as BDS did (research/captures/bds-container-cleanup.txt): every 15 s
//! the numbers of shown bundles no longer anywhere, held by no player and lying nowhere, go to every
//! player in one ContainerRegistryCleanup. A bundle thrown down stayed until its item was gone; one
//! cleared or replaced went at the next sweep, 0.3 to 12 s after.
// TODO: a bundle put in a chest or a mob's hands counts as gone; BDS was not drilled on it, and
// lists several in its own order (the newest first in the one capture of two).

use std::collections::BTreeSet;

use acacia_proto::packets::ContainerRegistryCleanup;
use bevy_ecs::prelude::*;

use super::bundles::{bundle_id, container};
use super::{Inventory, MAIN, Place};
use crate::components::{Outgoing, Tick, in_world};
use crate::drops::Dropped;

const SWEEP: u64 = 300;

/// The bundle numbers clients were shown the contents of.
#[derive(Resource, Default)]
struct Registered(BTreeSet<i32>);

pub(super) fn register(world: &mut World, id: i32) {
    world.get_resource_or_insert_with(Registered::default).0.insert(id);
}

/// Every [`SWEEP`] ticks, tells every player of the shown bundles that are no longer anywhere.
pub(crate) fn sweep(world: &mut World) {
    if !world.resource::<Tick>().0.is_multiple_of(SWEEP) || world.get_resource::<Registered>().is_none_or(|registered| registered.0.is_empty()) {
        return;
    }
    let players = in_world(world);
    let mut kept: BTreeSet<i32> = world.query::<&Dropped>().iter(world).filter_map(|dropped| bundle_id(dropped.stack())).collect();
    for inventory in players.iter().filter_map(|player| world.get::<Inventory>(*player)) {
        let places = (0..MAIN).map(Place::Main).chain([Place::Offhand, Place::Cursor]).chain((0..inventory.container.len()).map(Place::Container));
        kept.extend(places.filter_map(|place| bundle_id(&inventory.slot(place)?.as_ref()?.stack)));
    }
    let mut registered = world.resource_mut::<Registered>();
    let gone: Vec<i32> = registered.0.iter().rev().copied().filter(|id| !kept.contains(id)).collect();
    if gone.is_empty() {
        return;
    }
    registered.0.retain(|id| kept.contains(id));
    let packet = ContainerRegistryCleanup { removed_containers: gone.into_iter().map(container).collect() };
    for player in players {
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            out.push(&packet);
        }
    }
}
