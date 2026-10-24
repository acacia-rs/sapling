//! The actors a lying item and an experience orb are kept as.

use acacia_proto::nbt::{self, LittleEndian, Value};
use acacia_world::BlockRegistry;

use super::{floats, with_fields, written};
use crate::{Error, ItemStack};

/// A new item actor: `stack` lying with the bottom of its box at `position`. `registry` names
/// the block a block item places.
// TODO: its age is not kept: an item that comes back lies its whole time again.
pub fn new_item_actor(unique_id: i64, stack: &ItemStack, registry: &BlockRegistry, position: [f32; 3], motion: [f32; 3]) -> Vec<u8> {
    let fields = [
        ("UniqueID", Value::Long(unique_id)),
        ("identifier", Value::String("minecraft:item".into())),
        ("Pos", floats(&position)),
        ("Rotation", floats(&[0.0, 0.0])),
        ("Motion", floats(&motion)),
        ("Item", Value::Compound(stack.fields(registry))),
    ];
    written(Value::Compound(fields.into_iter().map(|(key, value)| (key.into(), value)).collect()))
}

/// The item actor kept as `compound`, holding `stack` now.
pub fn lying_stack(compound: &[u8], stack: &ItemStack, registry: &BlockRegistry) -> Result<Vec<u8>, Error> {
    with_fields(compound, [("Item", Value::Compound(stack.fields(registry)))])
}

// TODO: the name BDS keeps an orb's worth under, as remembered: no BDS record was read.
const WORTH: &str = "experience value";

/// A new orb actor worth `value`; its age is not kept either.
pub fn new_orb_actor(unique_id: i64, value: i32, position: [f32; 3], motion: [f32; 3]) -> Vec<u8> {
    let fields = [
        ("UniqueID", Value::Long(unique_id)),
        ("identifier", Value::String("minecraft:xp_orb".into())),
        ("Pos", floats(&position)),
        ("Rotation", floats(&[0.0, 0.0])),
        ("Motion", floats(&motion)),
        (WORTH, Value::Int(value)),
    ];
    written(Value::Compound(fields.into_iter().map(|(key, value)| (key.into(), value)).collect()))
}

/// The orb kept as `compound`, worth `value` now.
pub fn orb_worth(compound: &[u8], value: i32) -> Result<Vec<u8>, Error> {
    with_fields(compound, [(WORTH, Value::Int(value))])
}

/// What the orb kept as `compound` is worth, if it says.
pub fn orb_value(compound: &[u8]) -> Option<i32> {
    match nbt::read::<LittleEndian>(&mut &compound[..]).ok()?.value.get(WORTH) {
        Some(Value::Int(value)) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::actors;

    #[test]
    fn a_lying_item_is_read_back_as_it_was_written() {
        let registry = BlockRegistry::vanilla();
        let stack = ItemStack { name: "minecraft:beef".into(), count: 3, damage: 0, block: None, tag: None };
        let read = actors(&new_item_actor(77, &stack, &registry, [5.5, 64.0, 5.5], [0.0; 3])).unwrap().remove(0);
        assert_eq!((read.identifier.as_str(), read.position, read.item), ("minecraft:item", [5.5, 64.0, 5.5], Some(stack.clone())));
        let grown = ItemStack { count: 9, ..stack.clone() };
        let kept = lying_stack(&new_item_actor(77, &stack, &registry, [5.5, 64.0, 5.5], [0.0; 3]), &grown, &registry).unwrap();
        assert_eq!(actors(&kept).unwrap().remove(0).item, Some(grown));
        let orb = new_orb_actor(78, 7, [1.5, 64.0, 2.5], [0.0; 3]);
        assert_eq!((actors(&orb).unwrap().remove(0).identifier.as_str(), orb_value(&orb)), ("minecraft:xp_orb", Some(7)));
        assert_eq!(orb_worth(&orb, 20).ok().and_then(|grown| orb_value(&grown)), Some(20));
    }
}
