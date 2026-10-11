//! The actor a lying item is kept as.

use acacia_proto::nbt::Value;
use acacia_world::BlockRegistry;

use super::{floats, written};
use crate::ItemStack;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actors::actors;

    #[test]
    fn a_lying_item_is_read_back_as_it_was_written() {
        let registry = BlockRegistry::vanilla();
        let stack = ItemStack { name: "minecraft:beef".into(), count: 3, damage: 0, block: None, tag: None };
        let read = actors(&new_item_actor(77, &stack, &registry, [5.5, 64.0, 5.5], [0.0; 3])).unwrap().remove(0);
        assert_eq!((read.identifier.as_str(), read.position, read.item), ("minecraft:item", [5.5, 64.0, 5.5], Some(stack)));
    }
}
