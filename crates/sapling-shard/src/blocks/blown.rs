//! What a blast leaves of the blocks it broke (docs/research/block-uses.md, "TNT").

use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos, Item};

use super::level;
use super::loot::{self, Breaker};

const STACK: u8 = 64;

/// Breaks `cells` for a blast: TNT among them is lit, and with `leaves` the rest leave what they
/// would to a bare hand that could mine them, like items in one stack, out of the last cell.
pub(crate) fn blow_out(world: &mut World, layer: Entity, cells: &[BlockPos], leaves: bool) {
    let mut left: Vec<Item> = Vec::new();
    for &cell in cells {
        if crate::tnt::catch(world, layer, cell) {
            continue;
        }
        let [x, y, z] = cell;
        let broken = level(world, layer).and_then(|level| level.registry().get(level.block(x, y, z))).map(|state| (state.name.to_owned(), state.properties.to_owned()));
        if let Some((name, properties)) = broken.filter(|_| leaves) {
            let mut items = loot::thrown(world, &name, &properties, &Breaker::UNAIDED);
            loot::add_entity_loot(world, layer, cell, &mut items);
            for item in items {
                match left.iter_mut().find(|kept| kept.name == item.name && kept.count + item.count <= STACK) {
                    Some(kept) => kept.count += item.count,
                    None => left.push(item),
                }
            }
        }
        super::update_block(world, layer, cell, &Block::new("minecraft:air"));
    }
    if let Some(&last) = cells.last() {
        loot::spill(world, layer, last, &left);
    }
}
