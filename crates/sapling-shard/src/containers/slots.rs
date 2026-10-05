//! The slots of the containers the shard holds: read and changed by the requests of the players
//! who have one open and by a furnace's own burning, written to the block entities at every
//! change, and spilled when the block is broken.

use acacia_proto::nbt::Value;
use acacia_proto::packets::InventorySlot;
use acacia_proto::types::WindowIDVarint;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{ItemStack, new_block_entity, with_container_items};

use super::{CHEST, FIRST_ID, Viewing, block_name, content, keep_entity, kind, lid, opened, pairs, shut_out, stored_slots};
use crate::components::{Layer, Outgoing};
use crate::drops;
use crate::inventory::{Held, wire};

/// A stack under a stack id no player's own stack has.
pub(crate) fn new_held(world: &mut World, stack: ItemStack) -> Held {
    let mut open = opened(world);
    open.ids += 1;
    Held { stack, id: FIRST_ID + open.ids }
}

/// The slots of the container a player has open.
pub(crate) fn slots_of(world: &World, player: Entity) -> Option<Vec<Option<Held>>> {
    let viewing = world.get::<Viewing>(player)?;
    slots_at(world, viewing.layer, viewing.pos)
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

/// Lets go of the slots of a container nobody has open: they are in its block entities.
pub(crate) fn release(world: &mut World, layer: Entity, pos: BlockPos) {
    let mut open = opened(world);
    if open.chests.get(&(layer, pos)).is_some_and(|chest| chest.viewers.is_empty()) {
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
    if let Some(Viewing { layer, pos, .. }) = world.get::<Viewing>(player).copied() {
        set_slots(world, layer, pos, slots, Some(player));
    }
}

/// Makes `slots` what the held container at `pos` holds: in the world, where each half keeps
/// its own, and slot by changed slot on the screens of those who have it open, but for `knows`,
/// whose request made the change and is answered with it.
pub(crate) fn set_slots(world: &mut World, layer: Entity, pos: BlockPos, slots: Vec<Option<Held>>, knows: Option<Entity>) {
    let Some((halves, per_half, changed)) = opened(world).chests.get_mut(&(layer, pos)).filter(|chest| chest.slots != slots).map(|chest| {
        let changed: Vec<usize> = (0..slots.len().min(chest.slots.len())).filter(|slot| chest.slots[*slot] != slots[*slot]).collect();
        chest.slots = slots.clone();
        (chest.halves.clone(), chest.per_half, changed)
    }) else {
        return;
    };
    for (viewer, window) in viewers(world, layer, pos).into_iter().filter(|(viewer, _)| Some(*viewer) != knows) {
        let Some(mut out) = world.get_mut::<Outgoing>(viewer) else { continue };
        for slot in &changed {
            let window_id = WindowIDVarint::from_raw(window.into());
            out.push(&InventorySlot { window_id, slot: *slot as u32, container: None, storage_item: None, item: wire(&slots[*slot], true) });
        }
    }
    for (half, held) in halves.iter().zip(slots.chunks(per_half)) {
        let Some(entity) = entity_holding(world, layer, *half, held) else { return };
        if !world.get_mut::<Layer>(layer).is_some_and(|mut level| level.world.set_block_entity(*half, &entity)) {
            tracing::warn!(?half, "a container's items could not be kept");
        }
    }
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

/// Gives the one-cell container held at `pos` its block entity again, after its block became
/// another and took the entity with it, and shows it to those who see the block.
pub(crate) fn rewrite(world: &mut World, layer: Entity, pos: BlockPos) {
    let entity = slots_at(world, layer, pos).and_then(|slots| entity_holding(world, layer, pos, &slots));
    if let Some(entity) = entity {
        keep_entity(world, layer, pos, &entity);
    }
}

/// Empties the container block at `pos` onto the ground, shuts its screens and leaves a double
/// chest's other half a single one; call before the block goes, with which its entity goes.
/// Other blocks are left alone.
pub(crate) fn spill(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(kind) = block_name(world, layer, pos).and_then(kind) else { return };
    let halves = pairs::halves(world, layer, pos);
    let half = halves.iter().position(|half| *half == pos).unwrap_or(0);
    let slots = match shut_out(world, layer, halves[0]) {
        Some(chest) => {
            lid::set(world, layer, &halves, false);
            chest.slots.chunks(chest.per_half).nth(half).map(<[_]>::to_vec).unwrap_or_default()
        }
        None => stored_slots(world, layer, pos, kind.slots),
    };
    pairs::unpair(world, layer, &halves, pos);
    for held in slots.into_iter().flatten() {
        drops::spill(world, layer, held.stack, pos);
    }
}
