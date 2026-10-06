//! Campfires cook: a click with raw food on a lit one lays it on the fire, and 600 ticks later it
//! drops off cooked (Java `CampfireBlockEntity`). The four places are the entity's `Item1` to
//! `Item4` with their `ItemTime1` to `ItemTime4`, as BDS keeps them
//! (docs/research/block-entities.md).
// TODO: what cooks is the smoker's recipes (the campfire ones are not read from CraftingData);
// one that was cooking when its world was saved stands still until food is added again.

use std::collections::HashSet;

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, GameMode};
use sapling_data::Station;
use sapling_world::{ItemStack, new_block_entity};

use crate::blocks::keep_entity;
use crate::components::{Layer, Mode};
use crate::drops;
use crate::inventory::{self, Inventory, Place};

const CAMPFIRES: &[&str] = &["minecraft:campfire", "minecraft:soul_campfire"];
const PLACES: [(&str, &str); 4] = [("Item1", "ItemTime1"), ("Item2", "ItemTime2"), ("Item3", "ItemTime3"), ("Item4", "ItemTime4")];
/// Ticks food takes on a campfire (Java's campfire recipes' `cookingtime`).
const COOK_TICKS: i32 = 600;

/// The campfires with food on them.
#[derive(Resource, Default)]
pub(crate) struct Cooking(HashSet<(Entity, BlockPos)>);

fn kept(world: &World, layer: Entity, pos: BlockPos) -> Option<Vec<(String, Value)>> {
    match world.get::<Layer>(layer)?.world.block_entity(pos)?.nbt.clone() {
        Value::Compound(fields) => Some(fields),
        _ => None,
    }
}

fn field<'a>(fields: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    fields.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

fn set_field(fields: &mut Vec<(String, Value)>, key: &str, value: Option<Value>) {
    fields.retain(|(name, _)| name != key);
    fields.extend(value.map(|value| (key.to_owned(), value)));
}

fn is_lit(world: &World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    world.get::<Layer>(layer).and_then(|layer| layer.world.registry().get(layer.world.block(x, y, z))).is_some_and(|state| CAMPFIRES.contains(&state.name) && state.property("extinguished") == Some("0"))
}

fn cooked(stack: &ItemStack) -> Option<ItemStack> {
    let (item, count) = sapling_data::smelted(Station::Smoker, &stack.name, stack.damage)?;
    Some(ItemStack { name: item.name.to_owned(), count: *count, damage: item.damage, block: item.block, tag: item.tag.clone() })
}

/// A click on the block `name` at `pos`: a lit campfire takes one of the food in the hand into
/// its first free place. False for any other block.
pub(crate) fn cook(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    if !CAMPFIRES.contains(&name) {
        return false;
    }
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.clone());
    let Some(stack) = held.filter(|stack| cooked(stack).is_some()) else { return true };
    // A campfire a plugin set has no entity yet.
    let mut fields = kept(world, layer, pos).unwrap_or_else(|| if let Value::Compound(fields) = new_block_entity("Campfire", pos) { fields } else { Vec::new() });
    let Some((place, time)) = PLACES.iter().find(|(place, _)| field(&fields, place).is_none()).filter(|_| is_lit(world, layer, pos)) else { return true };
    let Some(registry) = world.get::<Layer>(layer).map(|layer| layer.world.registry()) else { return true };
    set_field(&mut fields, place, Some(Value::Compound(ItemStack { count: 1, ..stack }.fields(registry))));
    set_field(&mut fields, time, Some(Value::Int(0)));
    keep_entity(world, layer, pos, &Value::Compound(fields));
    world.get_resource_or_insert_with(Cooking::default).0.insert((layer, pos));
    if world.get::<Mode>(player).is_none_or(|mode| mode.0 != GameMode::Creative) {
        inventory::change(world, player, |inventory| inventory.take(Place::Main(inventory.selected.into()), 1).is_ok());
    }
    true
}

/// Moves the food on each lit campfire a tick on; what is done drops off cooked, for everyone
/// to see. A campfire put out or gone, or with nothing left on it, is let go.
pub(crate) fn tick(world: &mut World) {
    let Some(cooking) = world.get_resource::<Cooking>().map(|cooking| cooking.0.clone()) else { return };
    for (layer, pos) in cooking {
        let Some(mut fields) = kept(world, layer, pos).filter(|_| is_lit(world, layer, pos)) else {
            world.resource_mut::<Cooking>().0.remove(&(layer, pos));
            continue;
        };
        let (mut done, mut cooks) = (Vec::new(), false);
        for (place, time) in PLACES {
            let Some(food) = field(&fields, place).and_then(ItemStack::parse) else { continue };
            let ticks = if let Some(Value::Int(ticks)) = field(&fields, time) { *ticks + 1 } else { 1 };
            if ticks < COOK_TICKS {
                set_field(&mut fields, time, Some(Value::Int(ticks)));
                cooks = true;
                continue;
            }
            set_field(&mut fields, place, None);
            set_field(&mut fields, time, Some(Value::Int(0)));
            done.extend(cooked(&food));
        }
        if !cooks {
            world.resource_mut::<Cooking>().0.remove(&(layer, pos));
        }
        if done.is_empty() {
            if let Some(mut level) = world.get_mut::<Layer>(layer) {
                level.world.set_block_entity(pos, &Value::Compound(fields));
            }
            continue;
        }
        keep_entity(world, layer, pos, &Value::Compound(fields));
        done.into_iter().for_each(|stack| drops::spill(world, layer, stack, pos));
    }
}
