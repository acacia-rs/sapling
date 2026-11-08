//! A villager's inventory: its pack's `minecraft:inventory` of 8 slots, filled by `/replaceitem
//! entity … slot.inventory` and by the food it picks up (love.rs).

use acacia_proto::packets::TakeItemEntity;
use bevy_ecs::prelude::*;
use sapling_api::MobSlotRefusal;
use sapling_world::ItemStack;

use crate::body::Body;
use crate::components::NetId;
use crate::drops;

pub(super) const SLOTS: usize = 8;

#[derive(Component, Clone, Debug, Default)]
pub(crate) struct Pantry(pub [Option<ItemStack>; SLOTS]);

impl Pantry {
    /// How many of `name` it holds.
    pub fn count(&self, name: &str) -> u32 {
        self.0.iter().flatten().filter(|stack| stack.name == name).map(|stack| u32::from(stack.count)).sum()
    }

    /// Takes up to `count` of `name` out; how many it took.
    pub fn take(&mut self, name: &str, mut count: u32) -> u32 {
        let wanted = count;
        for slot in self.0.iter_mut() {
            let Some(stack) = slot.as_mut().filter(|stack| stack.name == name) else { continue };
            let taken = count.min(u32::from(stack.count));
            stack.count -= taken as u8;
            count -= taken;
            if stack.count == 0 {
                *slot = None;
            }
        }
        wanted - count
    }

    /// Puts `stack` in, onto its kind's stacks first; what did not fit.
    pub fn add(&mut self, mut stack: ItemStack) -> Option<ItemStack> {
        for slot in self.0.iter_mut().flatten().filter(|held| held.name == stack.name && held.damage == stack.damage) {
            let moved = stack.count.min(64 - slot.count.min(64));
            (slot.count, stack.count) = (slot.count + moved, stack.count - moved);
        }
        if stack.count == 0 {
            return None;
        }
        match self.0.iter_mut().find(|slot| slot.is_none()) {
            Some(free) => {
                *free = Some(stack);
                None
            }
            None => Some(stack),
        }
    }
}

/// What a villager picks up (its pack's `shareables`: the food, and a peasant's seeds and wheat,
/// which BDS's farmer took up as it reaped, bds-an1farm.txt).
pub(super) const FOODS: [&str; 7] = [
    "minecraft:bread", "minecraft:carrot", "minecraft:potato", "minecraft:beetroot", "minecraft:wheat_seeds", "minecraft:beetroot_seeds", "minecraft:wheat",
];
/// `behavior.pickup_items`: items within `max_dist` 3, walked to at 0.5 of its 0.5.
const SEES: f32 = 3.0;
const FETCHES: f32 = 0.25;
/// Its box widened by this much across takes an item up (piglins/pickup.rs).
const REACH: f32 = 1.3;
const HEIGHT: f32 = 1.9;

/// Before its move: a grown villager goes for food lying within [`SEES`] and takes the whole stack
/// up once it touches it (bds-g11breed.txt: 24 bread in one `TakeItemEntity`). False with none near.
pub(super) fn fetch(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    if crate::creatures::is_baby(world, entity) {
        return false;
    }
    let near = |at: [f32; 3]| (0..3).map(|axis| (at[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt();
    let lying = drops::lying_in(world, layer, body.at.map(|c| c - SEES), body.at.map(|c| c + SEES));
    let nearest = lying
        .into_iter()
        .filter(|(item, stack)| FOODS.contains(&stack.name.as_str()) && drops::mobs_may_take(world, *item))
        .filter_map(|(item, stack)| world.get::<Body>(item).map(|lies| (item, stack, lies.at)))
        .filter(|(.., at)| near(*at) <= SEES)
        .min_by(|a, b| near(a.2).total_cmp(&near(b.2)));
    let Some((item, stack, at)) = nearest else { return false };
    let touches = (at[0] - body.at[0]).abs() <= REACH && (at[2] - body.at[2]).abs() <= REACH && (body.at[1]..=body.at[1] + HEIGHT).contains(&at[1]);
    if !touches {
        crate::creatures::legs::walk(world, entity, layer, body, [at[0], at[2]], FETCHES, 0.0);
        return true;
    }
    let (Some(item_id), Some(id)) = (world.get::<NetId>(item).map(|id| id.0), world.get::<NetId>(entity).map(|id| id.0)) else { return false };
    let mut pantry = world.get::<Pantry>(entity).cloned().unwrap_or_default();
    let left = pantry.add(stack.clone());
    let taken = stack.count - left.map_or(0, |left| left.count);
    if taken == 0 {
        return false;
    }
    world.entity_mut(entity).insert(pantry);
    let told = sapling_net::encode(&TakeItemEntity { runtime_entity_id: item_id, target: id as u32 });
    crate::blocks::to_viewers(world, layer, at.map(|c| c.floor() as i32), &[told]);
    drops::take(world, item, taken);
    true
}

/// Writes each kept villager's pantry where BDS saves it: `ChestItems` by slot
/// (research/captures/bds-f14nkeep-dump.txt: eight slots, `InventoryVersion` beside them).
pub(crate) fn keep(world: &mut World) {
    let kept: Vec<(Entity, Vec<(u8, ItemStack)>)> = world
        .query_filtered::<(Entity, &Pantry), With<crate::entities::Stored>>()
        .iter(world)
        .map(|(entity, pantry)| (entity, pantry.0.iter().enumerate().filter_map(|(slot, stack)| Some((slot as u8, stack.clone()?))).collect()))
        .collect();
    for (entity, items) in kept {
        crate::entities::restore(world, entity, |compound, registry| sapling_world::chested_actor(compound, &items, registry).ok());
    }
}

/// A villager a world folder kept holds its `ChestItems` again.
pub(crate) fn adopt(world: &mut World, entity: Entity, compound: &[u8]) {
    let mut pantry = Pantry::default();
    for (slot, stack) in sapling_world::actor_chest(compound).into_iter().filter(|(slot, _)| usize::from(*slot) < SLOTS) {
        pantry.0[usize::from(slot)] = Some(stack);
    }
    if pantry.0.iter().any(Option::is_some) {
        world.entity_mut(entity).insert(pantry);
    }
}

/// `/replaceitem entity <villager> slot.inventory <slot>`: false for a mob that is no villager.
pub(crate) fn put(world: &mut World, mob: Entity, slot: usize, stack: Option<ItemStack>) -> Option<Result<(), MobSlotRefusal>> {
    world.get::<super::Trader>(mob).filter(|trader| trader.leaves_at.is_none())?;
    if slot >= SLOTS {
        return Some(Err(MobSlotRefusal::BadSlot { size: SLOTS }));
    }
    let mut pantry = world.get::<Pantry>(mob).cloned().unwrap_or_default();
    pantry.0[slot] = stack;
    world.entity_mut(mob).insert(pantry);
    Some(Ok(()))
}
