//! A generated container's loot: its `LootTable` rolled the first time the shard holds it (opened,
//! fed by a hopper, broken), then both fields dropped, as BDS fills it on first open. BDS's unpack
//! (144878e90) seeds its `Random` (MT19937) with `LootTableSeed` (a seed of 0 from the level's
//! random) and fills the container as `LootTable::fill` (14127e3b0) does (docs/research/structure-block-entities.md,
//! "Loot on first open").

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Item};
use sapling_data::Looted;
use sapling_data::enchant::Draws;
use sapling_world::{ItemStack, container_items, with_container_items};
use sapling_worldgen::random::Mt19937;

use crate::blocks::{Dice, keep_entity};
use crate::components::Layer;

const TABLE: &str = "LootTable";
const SEED: &str = "LootTableSeed";

struct Bedrock<'a>(&'a mut Mt19937);

impl Draws for Bedrock<'_> {
    fn int(&mut self, bound: i32) -> i32 {
        self.0.next_int(bound)
    }

    fn float(&mut self) -> f32 {
        self.0.next_float()
    }
}

/// Rolls the loot the block entity at `pos` still waits for into its first `slots` slots.
pub(super) fn unpack(world: &mut World, layer: Entity, pos: BlockPos, slots: usize) {
    let Some(entity) = world.get::<Layer>(layer).and_then(|layer| layer.world.block_entity(pos)) else { return };
    let Some(Value::String(table)) = entity.nbt.get(TABLE) else { return };
    let (table, nbt) = (table.clone(), entity.nbt.clone());
    let seed = match nbt.get(SEED) {
        Some(Value::Int(seed)) if *seed != 0 => *seed as u32,
        _ => world.get_resource_or_insert_with(Dice::default).roll(u32::MAX),
    };
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let mut items = container_items(&nbt);
    let free: Vec<u8> = (0..slots.min(usize::from(u8::MAX)) as u8).filter(|slot| !items.iter().any(|(at, _)| at == slot)).collect();
    let registry = level.registry();
    items.extend(rolled(&table, seed, free).into_iter().filter_map(|(slot, looted)| Some((slot, kept_stack(looted, registry)?))));
    let Value::Compound(mut fields) = with_container_items(&nbt, &items, registry) else { return };
    fields.retain(|(key, _)| key != TABLE && key != SEED);
    keep_entity(world, layer, pos, &Value::Compound(fields));
}

/// A rolled stack as a world keeps it, its data and tag with it.
pub(crate) fn kept_stack(looted: Looted, registry: &sapling_world::BlockRegistry) -> Option<ItemStack> {
    let item = Item { damage: looted.data, ..Item::new(looted.name, looted.count.min(64) as u8) };
    let mut stack = crate::inventory::stored(&item, registry)?;
    stack.tag = looted.tag.or(stack.tag);
    Some(stack)
}

/// What `table` rolled with `seed` puts in the `free` slots, by slot.
pub(crate) fn rolled(table: &str, seed: u32, free: Vec<u8>) -> Vec<(u8, Looted)> {
    let mut random = Mt19937::new(seed);
    let rolled = sapling_data::drawn_table_loot(table, &mut Bedrock(&mut random));
    fill(rolled, free, &mut random)
}

/// What `table` draws with `seed`, unplaced.
pub(crate) fn drawn(table: &str, seed: u32) -> Vec<Looted> {
    sapling_data::drawn_table_loot(table, &mut Bedrock(&mut Mt19937::new(seed)))
}

/// MSVC's `std::shuffle` on BDS's generator: each later element swapped with one at or before it.
fn shuffle<T>(list: &mut [T], random: &mut Mt19937) {
    for i in 1..list.len() {
        let j = (random.next_u32() % (i as u32 + 1)) as usize;
        list.swap(i, j);
    }
}

/// `LootTable::fill` (14127e3b0): the free slots shuffled; stacks of more than one set aside and,
/// while the slots left over the single stacks were more than none at the start (BDS never counts
/// again), one of them at a time split in two, each part of more than one set aside again on a coin;
/// then every stack shuffled and put in the slots from the back of the list. Stacks past the last
/// free slot are left out.
fn fill(rolled: Vec<Looted>, mut free: Vec<u8>, random: &mut Mt19937) -> Vec<(u8, Looted)> {
    shuffle(&mut free, random);
    let (mut split, mut stacks): (Vec<_>, Vec<_>) = rolled.into_iter().filter(|looted| looted.count > 0).partition(|looted| looted.count > 1);
    if free.len() > stacks.len() {
        while !split.is_empty() {
            let looted = split.remove(random.next_int(split.len() as i32) as usize);
            let part = random.next_int((looted.count >> 1) as i32) as u32 + 1;
            for count in [looted.count - part, part] {
                let piece = Looted { count, ..looted.clone() };
                if count > 1 && random.next_boolean() { split.push(piece) } else { stacks.push(piece) }
            }
        }
    }
    stacks.append(&mut split);
    shuffle(&mut stacks, random);
    stacks.into_iter().map_while(|stack| Some((free.pop()?, stack))).collect()
}

#[cfg(test)]
mod tests;
