//! Stacks as the data crate and the wire take them.

use acacia_proto::types::ItemV4;
use sapling_data::Stack;
use sapling_world::ItemStack;

use super::Held;

pub(crate) fn stack(stored: &ItemStack) -> Stack<'_> {
    Stack { name: &stored.name, count: stored.count, damage: stored.damage, block: stored.block, tag: stored.tag.as_ref() }
}

pub(crate) fn wire(slot: &Option<Held>, with_id: bool) -> ItemV4 {
    let shown = |held: &Held| sapling_data::item(&stack(&held.stack), with_id.then_some(held.id));
    slot.as_ref().and_then(shown).unwrap_or_else(sapling_data::no_item)
}

/// Whether two stacks are of one kind, so that one can go onto the other.
pub(crate) fn stacks_with(a: &ItemStack, b: &ItemStack) -> bool {
    (&a.name, a.damage, a.block, &a.tag) == (&b.name, b.damage, b.block, &b.tag)
}
