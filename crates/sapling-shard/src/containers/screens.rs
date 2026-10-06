//! Opening and closing the screens of blocks: containers, crafting tables and stonecutters.

use acacia_proto::packets::ContainerClose;
use acacia_proto::types::{WindowID, WindowType};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_data::Station;

use super::{Chest, content, hold, kind, lid, opened, slots};
use crate::components::{Outgoing, PacketQueues, Tick};
use crate::inventory::{self, Held, Inventory};
use crate::furnaces;

const TABLE: &str = "minecraft:crafting_table";
const STONECUTTER: &str = "minecraft:stonecutter_block";

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Screen {
    Container,
    /// An ender chest's: the slots are the viewer's own.
    Own,
    Station(Station),
}

/// The block whose screen a player has open, and the window its client knows it as. For a
/// double chest the block is its lead half.
#[derive(Component, Clone, Copy)]
pub(crate) struct Viewing {
    pub(super) layer: Entity,
    pub(super) pos: BlockPos,
    pub(super) window: u8,
    pub(super) screen: Screen,
}

/// Opens the screen of the block `block` at `pos` for a player: a container's with what it
/// holds, a crafting table's, a stonecutter's. False for a block without one.
pub(crate) fn open(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, block: &str) -> bool {
    let station = [(TABLE, Station::Table), (STONECUTTER, Station::Stonecutter)].into_iter().find(|(name, _)| *name == block);
    if let Some((_, station)) = station {
        let window = if station == Station::Table { WindowType::Workbench } else { WindowType::Stonecutter };
        return show_screen(world, player, Viewing { layer, pos, window: 0, screen: Screen::Station(station) }, window, None);
    }
    let Some(kind) = kind(block) else { return false };
    close(world, player);
    let Some(lead) = hold(world, layer, pos) else { return false };
    let Some((halves, is_first)) = opened(world).chests.get_mut(&(layer, lead)).map(|chest| {
        chest.viewers.push(player);
        (chest.halves.clone(), chest.viewers.len() == 1)
    }) else {
        return false;
    };
    let screen = if kind.is_per_player { Screen::Own } else { Screen::Container };
    let viewing = Viewing { layer, pos: lead, window: 0, screen };
    let shown_slots = match screen {
        Screen::Own => world.get::<Inventory>(player).map(|inventory| inventory.ender.clone()),
        _ => slots::slots_at(world, layer, lead),
    };
    let shown = show_screen(world, player, viewing, kind.window, Some(&shown_slots.unwrap_or_default()));
    if shown && is_first {
        lid::set(world, layer, &halves, true);
    }
    if kind.is_smelter() {
        furnaces::watch(world, layer, pos);
    }
    shown
}

/// Sends a player the screen of the block it now looks into, under the next window id, with
/// the slots of a container.
fn show_screen(world: &mut World, player: Entity, viewing: Viewing, window_type: WindowType, slots: Option<&[Option<Held>]>) -> bool {
    close(world, player);
    let mut query = world.query::<(&mut Inventory, &mut Outgoing)>();
    let Ok((mut inventory, mut out)) = query.get_mut(world, player) else { return false };
    let window = inventory.next_window();
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

/// Ends a player's look into a block's screen. The last one to leave a container shuts its lid,
/// and its slots are let go unless it ticks.
pub(crate) fn close(world: &mut World, player: Entity) {
    let Some(viewing) = stop_viewing(world, player) else { return };
    let key = (viewing.layer, viewing.pos);
    let Some(chest) = opened(world).into_inner().chests.get_mut(&key) else { return };
    chest.viewers.retain(|viewer| *viewer != player);
    if !chest.viewers.is_empty() {
        return;
    }
    let halves = chest.halves.clone();
    lid::set(world, viewing.layer, &halves, false);
    slots::release(world, viewing.layer, viewing.pos);
}

fn stop_viewing(world: &mut World, player: Entity) -> Option<Viewing> {
    let viewing = world.get::<Viewing>(player).copied()?;
    world.entity_mut(player).remove::<Viewing>();
    Some(viewing)
}

/// Shuts the screens of everyone at the container whose lead half is `lead`, as the server's
/// doing, and forgets its slots: they are kept in its block entities.
pub(super) fn shut_out(world: &mut World, layer: Entity, lead: BlockPos) -> Option<Chest> {
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
    let Some(open) = world.get_resource::<super::OpenContainers>() else { return };
    let lids: Vec<(Entity, Vec<BlockPos>)> = open.chests.iter().filter(|(_, chest)| !chest.viewers.is_empty()).map(|((layer, _), chest)| (*layer, chest.halves.clone())).collect();
    for (layer, halves) in lids {
        lid::show_open(world, layer, &halves);
    }
}
