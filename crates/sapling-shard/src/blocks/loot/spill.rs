//! Putting what a broken block leaves into the world: thrown with the shard's dice, tagged by the
//! block's entity, tossed out as item entities.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Item};

use super::{Breaker, Dice, of};
use crate::blocks::{custom, entities, level, mining, send};
use crate::components::Layer;
use crate::{drops, inventory};

const TILE_DROPS: &str = "doTileDrops";
const ENTITY_DROPS: &str = "doEntityDrops";

/// The tags a broken block's entity gives the items it leaves, which a plugin's [`Item`] cannot
/// carry: each goes, in order, on the next spilled stack of its item; a `None` keeps that stack's
/// place for an untagged held stack.
#[derive(Default)]
pub(in crate::blocks) struct Tags(Vec<(String, Option<acacia_proto::nbt::Value>)>);

impl Tags {
    /// The tag the next stack of `name` gets, taken out.
    pub(in crate::blocks) fn take(&mut self, name: &str) -> Option<acacia_proto::nbt::Value> {
        let at = self.0.iter().position(|(tagged, _)| tagged == name)?;
        self.0.remove(at).1
    }

    pub(in crate::blocks) fn one(name: &str, tag: Option<acacia_proto::nbt::Value>) -> Self {
        Tags(vec![(name.to_owned(), tag)])
    }

    fn put_on(&mut self, stack: &mut sapling_world::ItemStack) {
        if let Some(tag) = self.take(&stack.name) {
            stack.tag = Some(tag);
        }
    }
}

/// Adds to `items`, left by the block at `pos`, what its entity kept (entities.rs); the tags
/// for [`spill_tagged`]. Call before the block goes.
pub(in crate::blocks) fn add_entity_loot(world: &World, layer: Entity, pos: BlockPos, items: &mut Vec<Item>) -> Tags {
    let Some(kept) = world.get::<Layer>(layer).and_then(|layer| layer.world.block_entity(pos)) else { return Tags::default() };
    entities::into_loot(&kept.nbt, items);
    Tags(entities::loot_tags(&kept.nbt))
}

/// [`of`] with the shard's dice.
pub(in crate::blocks) fn thrown(world: &mut World, block: &str, properties: &str, by: &Breaker) -> Vec<Item> {
    if let Some(drops) = custom::drops(block) {
        return drops;
    }
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    of(block, properties, by, &mut |sides| dice.roll(sides))
}

/// What the block at `pos` would drop to `tool` (`/loot ... mine`); the block stays. Nothing from
/// air, a liquid or a cell outside the world.
pub(crate) fn mined(world: &mut World, layer: Entity, pos: BlockPos, tool: Option<&Item>) -> Vec<Item> {
    let Some(level) = level(world, layer).filter(|level| level.in_height(pos[1]) && level.is_loaded(send::column(pos))) else { return Vec::new() };
    let Some(state) = level.registry().get(level.block(pos[0], pos[1], pos[2])).filter(|state| !state.is_air() && !state.is_liquid()) else { return Vec::new() };
    let (name, properties, by) = (state.name.to_owned(), state.properties.to_owned(), mining::breaker_of(tool, &state.mining));
    thrown(world, &name, &properties, &by)
}

/// `items` as stacks; items the game lacks are left out.
fn stacks(world: &World, layer: Entity, items: &[Item]) -> Vec<sapling_world::ItemStack> {
    let Some(registry) = world.get::<Layer>(layer).map(|layer| layer.world.registry()) else { return Vec::new() };
    items.iter().filter(|item| item.count > 0).filter_map(|item| inventory::stored(item, registry)).collect()
}

/// Lays `items` where the block at `pos` was, tossed out of its cell, unless the world's
/// `doTileDrops` is off.
pub(in crate::blocks) fn spill(world: &mut World, layer: Entity, pos: BlockPos, items: &[Item]) {
    spill_tagged(world, layer, pos, items, &mut Tags::default());
}

/// [`spill`], `tags` going on the stacks of their items.
pub(in crate::blocks) fn spill_tagged(world: &mut World, layer: Entity, pos: BlockPos, items: &[Item], tags: &mut Tags) {
    if !crate::rules::on(world, layer, TILE_DROPS) {
        return;
    }
    for mut stack in stacks(world, layer, items) {
        tags.put_on(&mut stack);
        drops::spill(world, layer, stack, pos);
    }
}

/// Tosses `items` out of `centre`, where an entity that broke up was, unless the world's
/// `doEntityDrops` is off.
pub(in crate::blocks) fn let_go(world: &mut World, layer: Entity, centre: [f32; 3], items: &[Item]) {
    if !crate::rules::on(world, layer, ENTITY_DROPS) {
        return;
    }
    for stack in stacks(world, layer, items) {
        drops::let_go(world, layer, stack, centre);
    }
}
