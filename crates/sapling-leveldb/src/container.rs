//! What a container's block entity holds: `Items`, each stack with the `Slot` it lies in.

use acacia_proto::nbt::{List, Value};
use acacia_world::BlockRegistry;

use crate::ItemStack;

const ITEMS: &str = "Items";
const COMPOUND: u8 = 10;

/// A block entity with nothing but its kind (`Chest`, `Barrel`) and its block.
pub fn new_block_entity(id: &str, [x, y, z]: [i32; 3]) -> Value {
    let field = |key: &str, value| (key.to_string(), value);
    Value::Compound(vec![field("id", Value::String(id.into())), field("x", Value::Int(x)), field("y", Value::Int(y)), field("z", Value::Int(z))])
}

/// The stacks of a container with their slots; stacks without a slot are left out.
pub fn container_items(entity: &Value) -> Vec<(u8, ItemStack)> {
    let Some(Value::List(items)) = entity.get(ITEMS) else { return Vec::new() };
    let slotted = |stored: &Value| match stored.get("Slot") {
        Some(Value::Byte(slot)) => Some((u8::try_from(*slot).ok()?, ItemStack::parse(stored)?)),
        _ => None,
    };
    items.items.iter().filter_map(slotted).collect()
}

/// The block entity holding `items` instead of what it held; its other fields stay.
pub fn with_container_items(entity: &Value, items: &[(u8, ItemStack)], registry: &BlockRegistry) -> Value {
    let stored = |(slot, stack): &(u8, ItemStack)| {
        let mut fields = stack.fields(registry);
        fields.push(("Slot".into(), Value::Byte(*slot as i8)));
        Value::Compound(fields)
    };
    let mut fields = if let Value::Compound(fields) = entity { fields.clone() } else { Vec::new() };
    fields.retain(|(key, _)| key != ITEMS);
    fields.push((ITEMS.into(), Value::List(List { tag: COMPOUND, items: items.iter().map(stored).collect() })));
    Value::Compound(fields)
}

/// The block entity as clients are sent it: they are shown a container's items when it opens.
pub(crate) fn without_items(entity: &Value) -> Value {
    match entity {
        Value::Compound(fields) => Value::Compound(fields.iter().filter(|(key, _)| key != ITEMS).cloned().collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_container_keeps_its_items_by_slot_and_its_other_fields() {
        let registry = BlockRegistry::vanilla();
        let stack = |name: &str, count| ItemStack { name: name.into(), count, damage: 0, block: None, tag: None };
        let chest = new_block_entity("Chest", [1, 64, -3]);
        assert!(container_items(&chest).is_empty());
        let items = vec![(0, stack("minecraft:diamond", 5)), (26, stack("minecraft:stick", 64))];
        let filled = with_container_items(&chest, &items, &registry);
        assert_eq!((container_items(&filled), filled.get("id")), (items, chest.get("id")));
        let emptied = with_container_items(&filled, &[], &registry);
        assert!(container_items(&emptied).is_empty() && emptied.get("x") == Some(&Value::Int(1)));
        assert_eq!(without_items(&filled), chest);
    }
}
