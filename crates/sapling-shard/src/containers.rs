//! Chests and barrels: 27 slots kept in the block's entity, as the game keeps them, shown to the
//! players who have the block open and changed by their inventory requests (inventory/requests.rs).
//! Two paired chests are one container of 54 (pairs.rs). A crafting table's and a stonecutter's
//! screens open and close the same way, with nothing to hold. What BDS does, and the evidence:
//! docs/research/containers-crafting.md, sections A to C.
// TODO: no furnaces, hoppers, shulker or ender chests; a player can stay at an open chest from
// any distance; a chest under a solid block still opens.

mod lid;
mod pairs;

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{ContainerClose, InventorySlot};
use acacia_proto::types::{WindowID, WindowIDVarint, WindowType};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_data::Station;
use sapling_world::{ItemStack, container_items, new_block_entity, with_container_items};

use crate::components::{Layer, Outgoing, PacketQueues, Tick};
use crate::inventory::{Held, Inventory, wire};
use crate::{blocks, drops, inventory};

/// The slots of one chest or barrel; a double chest has them twice, the lead half's first.
const SLOTS: usize = 27;
/// Stack ids of what a container held when it was opened; players' own count up from 1.
const FIRST_ID: i32 = 1 << 24;
const TABLE: &str = "minecraft:crafting_table";
const STONECUTTER: &str = "minecraft:stonecutter_block";

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Chest,
    Station(Station),
}

/// The block whose screen a player has open, and the window its client knows it as. For a
/// double chest the block is its lead half.
#[derive(Component, Clone, Copy)]
pub(crate) struct Viewing {
    layer: Entity,
    pos: BlockPos,
    window: u8,
    screen: Screen,
}

struct Chest {
    /// Its cells, the lead half first.
    halves: Vec<BlockPos>,
    slots: Vec<Option<Held>>,
    viewers: Vec<Entity>,
}

/// The containers that are open, each until its last viewer closes it, by their lead half.
#[derive(Resource, Default)]
pub(crate) struct OpenContainers {
    chests: HashMap<(Entity, BlockPos), Chest>,
    ids: i32,
}

/// The `id` of the block entity a container block has; `None` for every other block.
fn kind(block: &str) -> Option<&'static str> {
    match block {
        "minecraft:chest" | "minecraft:trapped_chest" => Some("Chest"),
        "minecraft:barrel" => Some("Barrel"),
        _ => None,
    }
}

fn opened(world: &mut World) -> Mut<'_, OpenContainers> {
    world.get_resource_or_insert_with(OpenContainers::default)
}

fn content(window: u8, slots: &[Option<Held>]) -> acacia_proto::packets::InventoryContent {
    sapling_data::inventory_content(WindowIDVarint::from_raw(window.into()), slots.iter().map(|slot| wire(slot, true)).collect())
}

/// Makes `entity` what the block at `pos` holds and shows it to those who see the block.
fn keep_entity(world: &mut World, layer: Entity, pos: BlockPos, entity: &Value) {
    let Some(mut level) = world.get_mut::<Layer>(layer) else { return };
    if !level.world.set_block_entity(pos, entity) {
        return tracing::warn!(?pos, "a block's entity could not be kept");
    }
    let shown: Vec<_> = level.world.block_entity(pos).map(|entity| blocks::block_entity_data(pos, entity)).into_iter().collect();
    blocks::to_viewers(world, layer, pos, &shown);
}

/// Gives a container block that was just placed at `pos` its empty block entity, and a chest
/// its other half if one stands beside it; other blocks are left alone.
pub(crate) fn furnish(world: &mut World, layer: Entity, pos: BlockPos, block: &str) {
    if let Some(id) = kind(block) {
        keep_entity(world, layer, pos, &new_block_entity(id, pos));
        pairs::pair_placed(world, layer, pos);
    }
}

/// What the block entity at `pos` holds, under new stack ids.
fn stored_slots(world: &mut World, layer: Entity, pos: BlockPos) -> Vec<Option<Held>> {
    let items = world.get::<Layer>(layer).and_then(|layer| layer.world.block_entity(pos)).map(|entity| container_items(&entity.nbt)).unwrap_or_default();
    let mut open = opened(world);
    let mut slots = vec![None; SLOTS];
    for (slot, stack) in items.into_iter().filter(|(slot, _)| usize::from(*slot) < SLOTS) {
        open.ids += 1;
        slots[usize::from(slot)] = Some(Held { stack, id: FIRST_ID + open.ids });
    }
    slots
}

/// Opens the screen of the block `block` at `pos` for a player: a container's with what it
/// holds, a crafting table's, a stonecutter's. False for a block without one.
pub(crate) fn open(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, block: &str) -> bool {
    let station = [(TABLE, Station::Table), (STONECUTTER, Station::Stonecutter)].into_iter().find(|(name, _)| *name == block);
    if let Some((_, station)) = station {
        return show_screen(world, player, Viewing { layer, pos, window: 0, screen: Screen::Station(station) }, None);
    }
    if kind(block).is_none() {
        return false;
    }
    close(world, player);
    let halves = pairs::halves(world, layer, pos);
    let key = (layer, halves[0]);
    if !opened(world).chests.contains_key(&key) {
        let mut slots = Vec::new();
        for half in &halves {
            slots.extend(stored_slots(world, layer, *half));
        }
        opened(world).chests.insert(key, Chest { halves: halves.clone(), slots, viewers: Vec::new() });
    }
    let Some((slots, is_first)) = opened(world).chests.get_mut(&key).map(|chest| {
        chest.viewers.push(player);
        (chest.slots.clone(), chest.viewers.len() == 1)
    }) else {
        return false;
    };
    let shown = show_screen(world, player, Viewing { layer, pos: key.1, window: 0, screen: Screen::Chest }, Some(&slots));
    if shown && is_first {
        lid::set(world, layer, &halves, true);
    }
    shown
}

/// Sends a player the screen of the block it now looks into, under the next window id, with
/// the slots of a container.
fn show_screen(world: &mut World, player: Entity, viewing: Viewing, slots: Option<&[Option<Held>]>) -> bool {
    close(world, player);
    let mut query = world.query::<(&mut Inventory, &mut Outgoing)>();
    let Ok((mut inventory, mut out)) = query.get_mut(world, player) else { return false };
    let window = inventory.next_window();
    let window_type = match viewing.screen {
        Screen::Station(Station::Stonecutter) => WindowType::Stonecutter,
        Screen::Station(_) => WindowType::Workbench,
        Screen::Chest => WindowType::Container,
    };
    out.push(&sapling_data::block_screen(WindowID::from_raw(window.into()), window_type, viewing.pos));
    if let Some(slots) = slots {
        out.push(&content(window, slots));
    }
    world.entity_mut(player).insert(Viewing { window, ..viewing });
    true
}

/// Where a player crafts: at the table or stonecutter whose screen it has open, or by hand.
pub(crate) fn station(world: &World, player: Entity) -> Station {
    match world.get::<Viewing>(player).map(|viewing| viewing.screen) {
        Some(Screen::Station(station)) => station,
        _ => Station::Hand,
    }
}

/// Ends a player's look into a block's screen; the last one to leave a container shuts its lid.
pub(crate) fn close(world: &mut World, player: Entity) {
    let Some(viewing) = stop_viewing(world, player) else { return };
    let key = (viewing.layer, viewing.pos);
    let mut open = opened(world);
    let Some(chest) = open.chests.get_mut(&key) else { return };
    chest.viewers.retain(|viewer| *viewer != player);
    if !chest.viewers.is_empty() {
        return;
    }
    let halves = open.chests.remove(&key).map(|chest| chest.halves).unwrap_or_default();
    lid::set(world, viewing.layer, &halves, false);
}

fn stop_viewing(world: &mut World, player: Entity) -> Option<Viewing> {
    let viewing = world.get::<Viewing>(player).copied()?;
    world.entity_mut(player).remove::<Viewing>();
    Some(viewing)
}

/// Shuts the screens of everyone at the container whose lead half is `lead`, as the server's
/// doing, and forgets its slots: they are kept in its block entities.
fn shut_out(world: &mut World, layer: Entity, lead: BlockPos) -> Option<Chest> {
    let chest = opened(world).chests.remove(&(layer, lead))?;
    for viewer in &chest.viewers {
        let Some(viewing) = stop_viewing(world, *viewer) else { continue };
        if let Some(mut out) = world.get_mut::<Outgoing>(*viewer) {
            out.push(&ContainerClose { window_id: WindowID::from_raw(viewing.window.into()), window_type: WindowType::Container, server: true });
        }
    }
    Some(chest)
}

/// Confirms every screen a client closes, which it waits for, and ends its look into the
/// block that screen showed.
pub(crate) fn dispatch_closes(world: &mut World) {
    let closes = world.resource_mut::<PacketQueues>().take::<ContainerClose>();
    for (player, closed) in closes {
        let window = closed.window_id.to_raw();
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            out.push(&ContainerClose { server: false, ..closed });
        }
        if world.get::<Viewing>(player).is_some_and(|viewing| i64::from(viewing.window) == window) {
            close(world, player);
        }
        inventory::clear_grid(world, player);
    }
}

/// Every [`lid::REPEAT`] ticks, shows the open lids again: to those who came by since.
pub(crate) fn show_lids_again(world: &mut World) {
    if world.resource::<Tick>().0 % lid::REPEAT != 0 {
        return;
    }
    let Some(open) = world.get_resource::<OpenContainers>() else { return };
    let lids: Vec<(Entity, Vec<BlockPos>)> = open.chests.iter().map(|((layer, _), chest)| (*layer, chest.halves.clone())).collect();
    for (layer, halves) in lids {
        lid::show_open(world, layer, &halves);
    }
}

/// The slots of the container a player has open.
pub(crate) fn slots_of(world: &World, player: Entity) -> Option<Vec<Option<Held>>> {
    let viewing = world.get::<Viewing>(player)?;
    Some(world.get_resource::<OpenContainers>()?.chests.get(&(viewing.layer, viewing.pos))?.slots.clone())
}

/// Shows a player the container it has open as it is.
pub(crate) fn resend(world: &mut World, player: Entity) {
    let (Some(viewing), Some(slots)) = (world.get::<Viewing>(player).copied(), slots_of(world, player)) else { return };
    if let Some(mut out) = world.get_mut::<Outgoing>(player) {
        out.push(&content(viewing.window, &slots));
    }
}

/// Makes `slots` what the container a player has open holds: in the world, where each half
/// keeps its own, and slot by changed slot on the screens of the others who have it open.
pub(crate) fn store(world: &mut World, player: Entity, slots: Vec<Option<Held>>) {
    let Some(Viewing { layer, pos, .. }) = world.get::<Viewing>(player).copied() else { return };
    let Some((halves, others, changed)) = opened(world).chests.get_mut(&(layer, pos)).filter(|chest| chest.slots != slots).map(|chest| {
        let changed: Vec<usize> = (0..slots.len().min(chest.slots.len())).filter(|slot| chest.slots[*slot] != slots[*slot]).collect();
        chest.slots = slots.clone();
        (chest.halves.clone(), chest.viewers.iter().copied().filter(|viewer| *viewer != player).collect::<Vec<_>>(), changed)
    }) else {
        return;
    };
    for other in others {
        let Some(window) = world.get::<Viewing>(other).map(|viewing| WindowIDVarint::from_raw(viewing.window.into())) else { continue };
        let Some(mut out) = world.get_mut::<Outgoing>(other) else { continue };
        for slot in &changed {
            out.push(&InventorySlot { window_id: window, slot: *slot as u32, container: None, storage_item: None, item: wire(&slots[*slot], true) });
        }
    }
    for (half, held) in halves.iter().zip(slots.chunks(SLOTS)) {
        let items: Vec<(u8, ItemStack)> = held.iter().enumerate().filter_map(|(slot, held)| Some((slot as u8, held.as_ref()?.stack.clone()))).collect();
        let Some(level) = world.get::<Layer>(layer) else { return };
        // A chest a handler set, or one stored without its entity, gets one with its first items.
        let before = level.world.block_entity(*half).map_or_else(|| new_block_entity("Chest", *half), |entity| entity.nbt.clone());
        let entity = with_container_items(&before, &items, level.world.registry());
        if !world.get_mut::<Layer>(layer).is_some_and(|mut level| level.world.set_block_entity(*half, &entity)) {
            tracing::warn!(?half, "a container's items could not be kept");
        }
    }
}

/// Empties the container block at `pos` onto the ground, shuts its screens and leaves a double
/// chest's other half a single one; call before the block goes, with which its entity goes.
pub(crate) fn spill(world: &mut World, layer: Entity, pos: BlockPos) {
    let halves = pairs::halves(world, layer, pos);
    let half = halves.iter().position(|half| *half == pos).unwrap_or(0);
    let slots = match shut_out(world, layer, halves[0]) {
        Some(chest) => {
            lid::set(world, layer, &halves, false);
            chest.slots.chunks(SLOTS).nth(half).map(<[_]>::to_vec).unwrap_or_default()
        }
        None => stored_slots(world, layer, pos),
    };
    pairs::unpair(world, layer, &halves, pos);
    for held in slots.into_iter().flatten() {
        drops::spill(world, layer, held.stack, pos);
    }
}
