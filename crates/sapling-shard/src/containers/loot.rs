//! A generated container's loot: its `LootTable` rolled with its `LootTableSeed` into shuffled
//! empty slots the first time the shard holds it (opened, fed by a hopper, broken), then both
//! fields dropped, as BDS fills it on first open (docs/research/structure-block-entities.md).
// TODO: [U] BDS splits stacks over the free slots as Java's `LootTable.fill` does; here each stack takes one slot.

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Item};
use sapling_world::{container_items, with_container_items};

use crate::blocks::{Dice, keep_entity};
use crate::components::Layer;

const TABLE: &str = "LootTable";
const SEED: &str = "LootTableSeed";

/// Rolls the loot the block entity at `pos` still waits for into its first `slots` slots.
pub(super) fn unpack(world: &mut World, layer: Entity, pos: BlockPos, slots: usize) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let Some(entity) = level.block_entity(pos) else { return };
    let Some(Value::String(table)) = entity.nbt.get(TABLE) else { return };
    let seed = match entity.nbt.get(SEED) {
        Some(Value::Int(seed)) => *seed as u32,
        _ => 0,
    };
    let mut dice = Dice::seeded(u64::from(seed));
    let rolled = sapling_data::table_loot(table, &mut || dice.fraction());
    let mut items = container_items(&entity.nbt);
    let mut free: Vec<u8> = (0..slots.min(usize::from(u8::MAX)) as u8).filter(|slot| !items.iter().any(|(at, _)| at == slot)).collect();
    for i in (1..free.len()).rev() {
        free.swap(i, dice.roll(i as u32 + 1) as usize);
    }
    let registry = level.registry();
    let stacks = rolled.into_iter().filter_map(|(name, count)| crate::inventory::stored(&Item::new(name, count.min(64) as u8), registry));
    items.extend(free.into_iter().zip(stacks));
    let Value::Compound(mut fields) = with_container_items(&entity.nbt, &items, registry) else { return };
    fields.retain(|(key, _)| key != TABLE && key != SEED);
    keep_entity(world, layer, pos, &Value::Compound(fields));
}
