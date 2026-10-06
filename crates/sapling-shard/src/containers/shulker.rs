//! Shulker boxes: what one holds goes with it when it is broken, in its item's `tag` as
//! `Items`, and comes back into the block entity when that item is placed. A box holds no
//! other box (docs/research/shulker-ender.md).

use acacia_proto::nbt::{List, Value};
use bevy_ecs::prelude::*;
use sapling_api::{Face, Item};
use sapling_world::{BlockRegistry, ItemStack, container_items, with_container_items};

use crate::components::Layer;
use crate::inventory::{self, Held};

const FACING: &str = "facing";
const ITEMS: &str = "Items";

pub(crate) fn is_shulker_box(name: &str) -> bool {
    name.ends_with("shulker_box")
}

pub(super) fn refuses(_: usize, item: &ItemStack) -> bool {
    is_shulker_box(&item.name)
}

/// The block entity of a box placed against `face` from an item tagged `item_tag`.
pub(super) fn placed(entity: Value, item_tag: Option<&Value>, face: Face, registry: &BlockRegistry) -> Value {
    let items = item_tag.map(container_items).unwrap_or_default();
    let Value::Compound(mut fields) = entity else { return entity };
    fields.push((FACING.to_owned(), Value::Byte(face as i8)));
    with_container_items(&Value::Compound(fields), &items, registry)
}

/// The item a broken box `name` leaves, holding `slots`; `None` for a block no item stands for.
pub(super) fn as_item(world: &World, layer: Entity, name: &str, slots: &[Option<Held>]) -> Option<ItemStack> {
    let registry = world.get::<Layer>(layer)?.world.registry();
    let mut stack = inventory::stored(&Item::new(name, 1), registry)?;
    let items: Vec<(u8, ItemStack)> = slots.iter().enumerate().filter_map(|(slot, held)| Some((slot as u8, held.as_ref()?.stack.clone()))).collect();
    // An empty box is tagged too, with a list of no type (REC: `bds_container_probe`).
    let no_items = || Value::Compound(vec![(ITEMS.to_owned(), Value::List(List { tag: 0, items: Vec::new() }))]);
    stack.tag = Some(if items.is_empty() { no_items() } else { with_container_items(&Value::Compound(Vec::new()), &items, registry) });
    Some(stack)
}
