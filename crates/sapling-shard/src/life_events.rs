//! A player's body and hands offered to plugins (sapling_api's life_event.rs), each asked with one
//! line from its mechanic's module. An event nobody hears is not built.

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::{
    BlockPos, ContainerCloseEvent, ContainerOpenEvent, Event, ExperienceGainEvent, FoodLossEvent, HealEvent, HealSource, ItemConsumeEvent, ItemPickupEvent, Player, SignEditEvent,
    SleepEvent, Slot,
};

use crate::components::in_world_identity;
use sapling_world::ItemStack;

use crate::inventory::{self, Inventory};
use crate::{plugins, worlds};

/// Offers the event `make` builds about `player`, with its world in focus; `None` where nobody
/// hears it or the player is not in the world.
fn ask<E: Event>(world: &mut World, player: Entity, make: impl FnOnce(&mut World, Player) -> E) -> Option<E> {
    if !plugins::listening::<E>(world) {
        return None;
    }
    let who = in_world_identity(world, player)?;
    let before = worlds::focused(world);
    worlds::focus_on(world, player);
    let event = make(world, who);
    let event = plugins::offer(world, event);
    worlds::focus(world, before);
    Some(event)
}

fn allowed<E: Event>(world: &mut World, player: Entity, make: impl FnOnce(&mut World, Player) -> E) -> bool {
    ask(world, player, make).is_none_or(|event| !event.is_stopped())
}

pub(crate) fn may_pick_up(world: &mut World, player: Entity, stack: &ItemStack) -> bool {
    allowed(world, player, |_, who| ItemPickupEvent::new(who, inventory::described(stack)))
}

/// Whether plugins let `player` finish eating what it holds.
pub(crate) fn may_consume(world: &mut World, player: Entity) -> bool {
    allowed(world, player, |world, who| {
        let held = world.get::<Inventory>(player).map(|inventory| inventory.selected);
        ItemConsumeEvent::new(who, held.and_then(|slot| inventory::item_at(world, player, Slot::Main(slot))))
    })
}

/// The experience `player` gains of `points` once plugins had their say; `None`: none at all.
pub(crate) fn experience_gained(world: &mut World, player: Entity, points: i32) -> Option<i32> {
    match ask(world, player, |_, who| ExperienceGainEvent::new(who, points)) {
        Some(event) if event.is_cancelled() => None,
        Some(event) => Some(event.amount),
        None => Some(points),
    }
}

pub(crate) fn may_lose_food(world: &mut World, player: Entity, from: f32, to: f32) -> bool {
    to >= from || allowed(world, player, |_, who| FoodLossEvent::new(who, from, to))
}

/// What `player` heals of `amount` once plugins had their say.
pub(crate) fn healed(world: &mut World, player: Entity, amount: f32, source: HealSource) -> f32 {
    match ask(world, player, |_, who| HealEvent::new(who, amount, source)) {
        Some(event) if event.is_cancelled() => 0.0,
        Some(event) => event.amount.max(0.0),
        None => amount,
    }
}

pub(crate) fn may_sleep(world: &mut World, player: Entity, bed: BlockPos) -> bool {
    allowed(world, player, |_, who| SleepEvent::new(who, bed))
}

/// Whether plugins let `player` turn the sign at `pos` from `kept` into `written`; a refused one
/// shows the sign as it was again.
pub(crate) fn may_edit_sign(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, kept: &Value, written: &Value) -> bool {
    let changed = ["FrontText", "BackText"].into_iter().map(|side| (side == "FrontText", text(kept, side), text(written, side))).find(|(_, old, new)| old != new);
    let Some((front, old, new)) = changed else { return true };
    if allowed(world, player, |_, who| SignEditEvent::new(who, pos, front, old, new)) {
        return true;
    }
    crate::blocks::keep_entity(world, layer, pos, kept);
    false
}

fn text(sign: &Value, side: &str) -> String {
    match sign.get(side).and_then(|side| side.get("Text")) {
        Some(Value::String(text)) => text.to_string(),
        _ => String::new(),
    }
}

pub(crate) fn may_open(world: &mut World, player: Entity, pos: BlockPos, block: &str) -> bool {
    allowed(world, player, |_, who| ContainerOpenEvent::new(who, pos, block.to_owned()))
}

pub(crate) fn closed(world: &mut World, player: Entity, pos: BlockPos) {
    ask(world, player, |_, who| ContainerCloseEvent::new(who, pos));
}
