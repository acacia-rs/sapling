//! Bundles' containers let go, as BDS did (research/captures/bds-container-cleanup.txt): every 15 s
//! the numbers of shown bundles no longer anywhere, held by no player and in no loaded container,
//! go to every player in one ContainerRegistryCleanup, the newest first. One thrown down, cleared or
//! replaced went at the next sweep, 0.3 to 12 s after (a lying bundle has no number,
//! bundles.rs [`super::unnumbered`]); one in a chest stayed its 34 s there (bds-t2ebundle.txt).
// TODO: a bundle in a mob's hands counts as gone; the fox drilled did not take one.

use std::collections::BTreeSet;

use acacia_proto::packets::ContainerRegistryCleanup;
use bevy_ecs::prelude::*;

use super::bundles::{bundle_id, container};
use super::{Inventory, MAIN, Place};
use crate::components::{Layer, Outgoing, Tick, in_world};

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
    let mut kept = BTreeSet::new();
    for inventory in players.iter().filter_map(|player| world.get::<Inventory>(*player)) {
        let places = (0..MAIN).map(Place::Main).chain([Place::Offhand, Place::Cursor]).chain((0..inventory.container.len()).map(Place::Container));
        kept.extend(places.filter_map(|place| bundle_id(&inventory.slot(place)?.as_ref()?.stack)));
    }
    for layer in world.query::<&Layer>().iter(world) {
        let level = &layer.world;
        let stored = level.loaded().flat_map(|column| level.block_entities_in(column)).flat_map(|entity| sapling_world::container_items(&entity.nbt));
        kept.extend(stored.filter_map(|(_, stack)| bundle_id(&stack)));
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
