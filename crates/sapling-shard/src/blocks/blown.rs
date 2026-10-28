//! What a blast leaves of the blocks it broke (docs/research/block-uses.md, "TNT", and
//! "Blasts").

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Item};

use super::level;
use super::loot::{self, Breaker};

/// A merged stack holds this many more items for every [`GROUP`] stacks already made
/// (BDS's `Explosion::explode`, which caps at the item's own stack too).
const MERGED: u32 = 16;
const GROUP: usize = 5;

/// Breaks `cells` for a blast: TNT among them is lit, and the rest, each by the chance `leaves`,
/// leave what they would to a bare hand that could mine them. Like items are merged into stacks
/// that each fall out of the cell whose block started them.
pub(crate) fn blow_out(world: &mut World, layer: Entity, cells: &[BlockPos], leaves: f32) {
    let mut left: Vec<(Item, BlockPos)> = Vec::new();
    for &cell in cells {
        if crate::tnt::catch(world, layer, cell) {
            continue;
        }
        let [x, y, z] = cell;
        let broken = level(world, layer).and_then(|level| level.registry().get(level.block(x, y, z))).map(|state| (state.name.to_owned(), state.properties.to_owned()));
        let kept = leaves >= 1.0 || (leaves > 0.0 && world.get_resource_or_insert_with(super::Dice::default).fraction() < leaves);
        if let Some((name, properties)) = broken.filter(|_| kept) {
            super::infested::release(world, layer, cell, &name, false);
            let mut items = loot::thrown(world, &name, &properties, &Breaker::UNAIDED);
            let mut tags = loot::add_entity_loot(world, layer, cell, &mut items);
            // A tagged stack (a banner's patterns) is its own, merged with none.
            let (tagged, plain): (Vec<Item>, Vec<Item>) = items.into_iter().partition(|item| tags.tags(&item.name));
            loot::spill_tagged(world, layer, cell, &tagged, &mut tags);
            for item in plain {
                merge(&mut left, item, cell);
            }
        }
        super::update_block(world, layer, cell, &Block::new("minecraft:air"));
    }
    for (item, cell) in left {
        loot::spill(world, layer, cell, &[item]);
    }
}

/// Adds `item` to the stacks like it, each up to the cap, and starts a stack at `cell` with what
/// is left over.
fn merge(left: &mut Vec<(Item, BlockPos)>, mut item: Item, cell: BlockPos) {
    let cap = (MERGED * (left.len() / GROUP) as u32 + MERGED).min(u32::from(sapling_data::max_stack(&item.name)));
    for (stack, _) in left.iter_mut().filter(|(stack, _)| stack.name == item.name && stack.damage == item.damage) {
        let sum = u32::from(stack.count) + u32::from(item.count);
        if sum <= cap {
            stack.count = sum as u8;
            return;
        }
        stack.count = cap as u8;
        item.count = (sum - cap) as u8;
    }
    left.push((item, cell));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirt(count: u8) -> Item {
        Item::new("minecraft:dirt", count)
    }

    #[test]
    fn a_blasts_dirt_merges_in_sixteens_out_of_the_cell_that_began_each_stack_as_on_bds() {
        let mut left = Vec::new();
        for i in 0..20 {
            merge(&mut left, dirt(1), [i, 0, 0]);
        }
        assert_eq!(left.iter().map(|(item, cell)| (item.count, cell[0])).collect::<Vec<_>>(), [(16, 0), (4, 16)]);
    }

    #[test]
    fn past_five_stacks_a_merged_stack_holds_sixteen_more() {
        let mut left: Vec<_> = (0..5).map(|i| (Item::new(format!("minecraft:item{i}"), 1), [i, 0, 0])).collect();
        merge(&mut left, dirt(20), [9, 0, 0]);
        merge(&mut left, dirt(20), [9, 0, 0]);
        assert_eq!(left[5].0.count, 32);
        assert_eq!(left[6].0.count, 8);
    }
}
