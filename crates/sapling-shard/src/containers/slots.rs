//! The slots of the containers the shard holds: read and changed by the requests of the players
//! who have one open, by furnaces and by hoppers, written to the block entities at every change,
//! and spilled when the block is broken.

use acacia_proto::nbt::Value;
use acacia_proto::packets::InventorySlot;
use acacia_proto::types::WindowIDVarint;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{ItemStack, new_block_entity, with_container_items};

use super::screens::{Screen, Viewing, shut_out};
use super::{CHEST, FIRST_ID, block_name, content, kind, kind_at, lid, opened, pairs, shulker, stored_slots};
use crate::components::{Layer, Outgoing};
use crate::drops;
use crate::inventory::{Held, Inventory, wire};

/// A stack under a stack id no player's own stack has.
pub(crate) fn new_held(world: &mut World, stack: ItemStack) -> Held {
    let mut open = opened(world);
    open.ids += 1;
    Held { stack, id: FIRST_ID + open.ids }
}

/// The slots of the container a player has open: an ender chest's are the player's own.
pub(crate) fn slots_of(world: &World, player: Entity) -> Option<Vec<Option<Held>>> {
    let viewing = world.get::<Viewing>(player)?;
    match viewing.screen {
        Screen::Own => Some(world.get::<Inventory>(player)?.ender.clone()),
        Screen::Container => slots_at(world, viewing.layer, viewing.pos),
        Screen::Carried(vehicle) => crate::vehicles::cargo(world, vehicle),
        Screen::Station(_) | Screen::Workstation(_) => None,
    }
}

/// The block of the container a player has open; `None` for one that is no block.
pub(crate) fn open_pos(world: &World, player: Entity) -> Option<BlockPos> {
    let viewing = world.get::<Viewing>(player)?;
    matches!(viewing.screen, Screen::Container).then_some(viewing.pos)
}

/// What the container a player has open refuses to hold in a slot.
pub(crate) fn refusal(world: &World, player: Entity) -> Option<fn(usize, &ItemStack) -> bool> {
    let viewing = world.get::<Viewing>(player)?;
    kind_at(world, viewing.layer, viewing.pos)?.refuses
}

/// The slots of the held container whose lead half is at `pos`.
pub(crate) fn slots_at(world: &World, layer: Entity, pos: BlockPos) -> Option<Vec<Option<Held>>> {
    Some(world.get_resource::<super::OpenContainers>()?.chests.get(&(layer, pos))?.slots.clone())
}

/// The players who have the container at `pos` open, each with its window.
pub(crate) fn viewers(world: &World, layer: Entity, pos: BlockPos) -> Vec<(Entity, u8)> {
    let Some(chest) = world.get_resource::<super::OpenContainers>().and_then(|open| open.chests.get(&(layer, pos))) else { return Vec::new() };
    chest.viewers.iter().filter_map(|viewer| Some((*viewer, world.get::<Viewing>(*viewer)?.window))).collect()
}

/// Lets go of the slots of a container nobody has open and nothing ticks: they are in its
/// block entities.
pub(super) fn release(world: &mut World, layer: Entity, pos: BlockPos) {
    let mut open = opened(world);
    if open.chests.get(&(layer, pos)).is_some_and(|chest| chest.viewers.is_empty() && !chest.pinned) {
        open.chests.remove(&(layer, pos));
    }
}

/// Shows a player the container it has open as it is.
pub(crate) fn resend(world: &mut World, player: Entity) {
    let (Some(viewing), Some(slots)) = (world.get::<Viewing>(player).copied(), slots_of(world, player)) else { return };
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&content(viewing.window, &slots));
    }
}

/// Makes `slots` what the container a player has open holds, after its request.
pub(crate) fn store(world: &mut World, player: Entity, slots: Vec<Option<Held>>) {
    let Some(viewing) = world.get::<Viewing>(player).copied() else { return };
    match viewing.screen {
        Screen::Own => {
            if let Some(mut inventory) = world.get_mut::<Inventory>(player).filter(|inventory| inventory.ender != slots) {
                (inventory.ender, inventory.unsaved) = (slots, true);
            }
        }
        Screen::Container => {
            let before = slots_at(world, viewing.layer, viewing.pos).unwrap_or_default();
            let changed: Vec<usize> = (0..slots.len().min(before.len())).filter(|slot| before[*slot] != slots[*slot]).collect();
            super::again::next_tick(world, player, viewing.window, Some(changed));
            set_slots(world, viewing.layer, viewing.pos, slots.clone());
            crate::furnaces::taken_by_player(world, player, viewing.layer, viewing.pos, &before, &slots);
        }
        Screen::Carried(vehicle) => crate::vehicles::load_cargo(world, vehicle, slots),
        Screen::Station(_) | Screen::Workstation(_) => {}
    }
}

/// Makes `slots` what the held container at `pos` holds: in the world, where each half keeps
/// its own, and slot by changed slot on the screens of those who have it open, the player whose
/// request made the change too, as BDS does before answering it.
pub(crate) fn set_slots(world: &mut World, layer: Entity, pos: BlockPos, slots: Vec<Option<Held>>) {
    let Some((halves, per_half, changed)) = opened(world).chests.get_mut(&(layer, pos)).filter(|chest| chest.slots != slots).map(|chest| {
        let changed: Vec<usize> = (0..slots.len().min(chest.slots.len())).filter(|slot| chest.slots[*slot] != slots[*slot]).collect();
        chest.slots = slots.clone();
        (chest.halves.clone(), chest.per_half, changed)
    }) else {
        return;
    };
    tell_changed(world, viewers(world, layer, pos), &changed, &slots);
    halves.iter().for_each(|half| crate::blocks::container_changed(world, layer, *half));
    for (half, held) in halves.iter().zip(slots.chunks(per_half)) {
        let Some(entity) = entity_holding(world, layer, *half, held) else { return };
        if !world.get_mut::<Layer>(layer).is_some_and(|mut level| level.world.set_block_entity(*half, &entity)) {
            tracing::warn!(?half, "a container's items could not be kept");
        }
    }
    super::brewing::show_bottles(world, layer, pos, &slots);
}

/// Shows `viewers` (each with its window) the `changed` slots of `slots`, slot by slot.
pub(crate) fn tell_changed(world: &mut World, viewers: Vec<(Entity, u8)>, changed: &[usize], slots: &[Option<Held>]) {
    for (viewer, window) in viewers {
        let Some(mut out) = world.get_mut::<Outgoing>(viewer) else { continue };
        for slot in changed {
            out.push(&slot_shown(window, *slot, &slots[*slot]));
        }
    }
}

/// Slot `slot` of the container a player has open under `window`, holding `held`.
pub(super) fn slot_shown(window: u8, slot: usize, held: &Option<Held>) -> InventorySlot {
    InventorySlot { window_id: WindowIDVarint::from_raw(window.into()), slot: slot as u32, container: None, storage_item: None, item: wire(held, true) }
}

/// The players who have the goods of `vehicle` open, each with its window.
pub(crate) fn carried_viewers(world: &mut World, vehicle: Entity) -> Vec<(Entity, u8)> {
    world.query::<(Entity, &Viewing)>().iter(world).filter(|(_, viewing)| viewing.screen == Screen::Carried(vehicle)).map(|(player, viewing)| (player, viewing.window)).collect()
}

/// The block entity at `pos` holding `held`; a container a handler set, or one stored without
/// its entity, gets a new one.
fn entity_holding(world: &World, layer: Entity, pos: BlockPos, held: &[Option<Held>]) -> Option<Value> {
    let items: Vec<(u8, ItemStack)> = held.iter().enumerate().filter_map(|(slot, held)| Some((slot as u8, held.as_ref()?.stack.clone()))).collect();
    let id = block_name(world, layer, pos).and_then(kind).unwrap_or(CHEST).entity;
    let level = &world.get::<Layer>(layer)?.world;
    let before = level.block_entity(pos).map_or_else(|| new_block_entity(id, pos), |entity| entity.nbt.clone());
    Some(with_container_items(&before, &items, level.registry()))
}

/// Empties the container block at `pos` onto the ground, shuts its screens and leaves a double
/// chest's other half a single one; call before the block goes, with which its entity goes. A
/// furnace leaves its result's experience. A shulker box keeps what it holds: it drops as an
/// item holding it, broken `by_creative` only when it holds something (BDS 148b54c10). Other
/// blocks are left alone.
pub(crate) fn spill(world: &mut World, layer: Entity, pos: BlockPos, by_creative: bool) {
    let Some((name, kind)) = block_name(world, layer, pos).and_then(|name| Some((name, kind(name)?))) else { return };
    let halves = pairs::halves(world, layer, pos);
    let half = halves.iter().position(|half| *half == pos).unwrap_or(0);
    let slots = match shut_out(world, layer, halves[0]) {
        Some(chest) => {
            lid::set(world, layer, &halves, false);
            chest.slots.chunks(chest.per_half).nth(half).map(<[_]>::to_vec).unwrap_or_default()
        }
        None if kind.holds_items() => stored_slots(world, layer, pos, kind.slots),
        None => Vec::new(),
    };
    pairs::unpair(world, layer, &halves, pos);
    crate::hoppers::forget(world, layer, pos);
    crate::furnaces::forget(world, layer, pos);
    crate::brewing::forget(world, layer, pos);
    crate::furnaces::broken(world, layer, pos, &slots);
    if shulker::is_shulker_box(name) {
        let drops = !by_creative || slots.iter().any(Option::is_some);
        if let Some(item) = shulker::as_item(world, layer, name, &slots).filter(|_| drops) {
            drops::spill(world, layer, item, pos);
        }
        return;
    }
    for held in slots.into_iter().flatten() {
        drops::spill(world, layer, held.stack, pos);
    }
}
