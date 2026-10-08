//! A player's items as plugins name them: [`Slot`]s and [`Item`]s.

use bevy_ecs::prelude::*;
use sapling_api::{Item, Slot};
use sapling_world::ItemStack;

use super::{Inventory, Place, change};

fn place(slot: Slot) -> Place {
    match slot {
        Slot::Main(slot) => Place::Main(slot.into()),
        Slot::Armor(slot) => Place::Armor(slot.into()),
        Slot::Offhand => Place::Offhand,
    }
}

/// A plugin's item as a stack: a block item places its block's first state.
pub(crate) fn stored(item: &Item, registry: &sapling_world::BlockRegistry) -> Option<ItemStack> {
    let block = registry.states_of(&item.name).next().map(|(id, _)| registry.network_hash(id));
    sapling_data::is_item(&item.name).then(|| ItemStack { name: item.name.clone(), count: item.count, damage: item.damage, block, tag: None })
}

pub(crate) fn described(stack: &ItemStack) -> Item {
    Item { name: stack.name.clone(), count: stack.count, damage: stack.damage }
}

/// What a slot holds, for a plugin.
pub(crate) fn item_at(world: &World, player: Entity, slot: Slot) -> Option<Item> {
    world.get::<Inventory>(player)?.slot(place(slot))?.as_ref().map(|held| described(&held.stack))
}

/// Sets a slot for a plugin; false for a slot that is not there.
pub(crate) fn set_item(world: &mut World, player: Entity, slot: Slot, stack: Option<ItemStack>) -> bool {
    change(world, player, |inventory| inventory.put(place(slot), stack)) == Some(true)
}
