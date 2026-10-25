//! A bow's or crossbow's arrows: those in the offhand first, then the inventory's from its first
//! slot (BDS's `BowItem::releaseUsing` 142eee8d0). A tipped arrow is `minecraft:arrow` whose data
//! is its potion's and one.

use bevy_ecs::prelude::*;
use sapling_world::ItemStack;

use crate::creatures::ARROW;
use crate::inventory::{self, Inventory, MAIN, Place};

/// Where the player's next arrow is, and one of it.
pub(super) fn find(world: &World, player: Entity) -> Option<(Place, ItemStack)> {
    let inventory = world.get::<Inventory>(player)?;
    let places = std::iter::once(Place::Offhand).chain((0..MAIN).map(Place::Main));
    places.into_iter().find_map(|place| {
        let held = inventory.slot(place)?.as_ref().filter(|held| held.stack.name == ARROW.kind)?;
        Some((place, ItemStack { count: 1, ..held.stack.clone() }))
    })
}

/// A plain arrow, for a creative player without any.
pub(super) fn plain() -> ItemStack {
    ItemStack { name: ARROW.kind.to_owned(), count: 1, damage: 0, block: None, tag: None }
}

/// The potion an arrow is tipped with.
pub(super) fn tip(arrow: &ItemStack) -> Option<i16> {
    (arrow.damage > 0).then(|| arrow.damage - 1)
}

/// One arrow at `place` is spent.
pub(super) fn spend(world: &mut World, player: Entity, place: Place) {
    inventory::change(world, player, |inventory| drop(inventory.take(place, 1)));
}
