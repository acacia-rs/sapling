//! Opening and closing the screens of blocks: containers, crafting tables, stonecutters and the
//! workstations (workstations.rs).

use acacia_proto::packets::{ContainerClose, ContainerOpen, UpdateEquipment};
use acacia_proto::types::{WindowID, WindowType};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_data::Station;

use super::{Chest, content, hold, kind, lid, opened, slots};
use crate::components::{Outgoing, PacketQueues, Tick};
use crate::inventory::{self, Held, Inventory};
use crate::furnaces;
use crate::workstations::{self, Workstation};

const TABLE: &str = "minecraft:crafting_table";
const STONECUTTER: &str = "minecraft:stonecutter_block";

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Screen {
    Container,
    /// An ender chest's: the slots are the viewer's own.
    Own,
    Station(Station),
    Workstation(Workstation),
    /// A chest boat's or minecart's (vehicles/cargo.rs).
    Carried(Entity),
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

/// Whether the block `block` still has the screen `screen` a player has open on its cell.
pub(super) fn still_shows(screen: Screen, block: &str) -> bool {
    match screen {
        Screen::Station(station) => [(TABLE, Station::Table), (STONECUTTER, Station::Stonecutter)].contains(&(block, station)),
        Screen::Container | Screen::Own => kind(block).is_some(),
        Screen::Workstation(workstation) => workstations::of_block(block).is_some_and(|(shown, _)| shown == workstation),
        Screen::Carried(_) => false,
    }
}

/// Opens the screen of the goods `slots` of `vehicle` (its id `id`, its cell `pos`) for a player:
/// by `ContainerOpen`, or for a mount by the `UpdateEquipment` that `equipment` makes for the
/// window, which BDS sends alone (research/captures/bds-tack.txt).
pub(crate) fn show_carried(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, vehicle: (Entity, u64), window_type: WindowType, slots: &[Option<Held>], equipment: Option<&dyn Fn(u8) -> UpdateEquipment>) -> bool {
    close(world, player);
    let mut query = world.query::<(&mut Inventory, &mut Outgoing)>();
    let Ok((mut inventory, mut out)) = query.get_mut(world, player) else { return false };
    let window = inventory.next_window();
    match equipment {
        Some(equipment) => out.push(&equipment(window)),
        None => out.push(&ContainerOpen { runtime_entity_id: vehicle.1 as i64, ..sapling_data::block_screen(WindowID::from_raw(window.into()), window_type, pos) }),
    }
    out.push(&content(window, slots));
    world.entity_mut(player).insert(Viewing { layer, pos, window, screen: Screen::Carried(vehicle.0) });
    true
}

/// Opens the screen of the block `block` at `pos` for a player: a container's with what it
/// holds, a crafting table's, a stonecutter's. False for a block without one.
pub(crate) fn open(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, block: &str) -> bool {
    if super::access::is_blocked(world, layer, pos, block) {
        return true;
    }
    if (kind(block).is_some() || workstations::of_block(block).is_some() || [TABLE, STONECUTTER].contains(&block)) && !crate::life_events::may_open(world, player, pos, block) {
        return true;
    }
    let station = [(TABLE, Station::Table), (STONECUTTER, Station::Stonecutter)].into_iter().find(|(name, _)| *name == block);
    if let Some((_, station)) = station {
        let window = if station == Station::Table { WindowType::Workbench } else { WindowType::Stonecutter };
        return show_screen(world, player, Viewing { layer, pos, window: 0, screen: Screen::Station(station) }, window, None);
    }
    if let Some((workstation, window)) = workstations::of_block(block) {
        let shown = show_screen(world, player, Viewing { layer, pos, window: 0, screen: Screen::Workstation(workstation) }, window, None);
        workstations::opened(world, player, workstation);
        return shown;
    }
    let Some(kind) = kind(block) else { return false };
    close(world, player);
    let Some(lead) = hold(world, layer, pos) else { return false };
    let now = world.resource::<Tick>().0;
    let Some((halves, is_first)) = opened(world).chests.get_mut(&(layer, lead)).map(|chest| {
        chest.viewers.push(player);
        if chest.viewers.len() == 1 {
            chest.opened_at = now;
        }
        (chest.halves.clone(), chest.viewers.len() == 1)
    }) else {
        return false;
    };
    if is_first && kind.has_lid {
        crate::blocks::vibrate_at(world, layer, pos, crate::blocks::GameEvent::ContainerOpen, true);
    }
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
    if kind.is_stand() {
        crate::brewing::opened(world, layer, pos);
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
    // BDS followed a container's slots with the player's windows and the slots again, and those
    // once more a tick on (research/captures/bds-chest-moves.txt, bds-containers.txt).
    if let Some(slots) = slots {
        out.push(&content(window, slots));
        inventory.send_all(&mut out, false);
        out.push(&content(window, slots));
    }
    world.entity_mut(player).insert(Viewing { window, ..viewing });
    if slots.is_some() {
        super::again::next_tick(world, player, window, None);
    }
    true
}

/// Where a player crafts: at the table or stonecutter whose screen it has open, or by hand.
pub(crate) fn station(world: &World, player: Entity) -> Station {
    match world.get::<Viewing>(player).map(|viewing| viewing.screen) {
        Some(Screen::Station(station)) => station,
        _ => Station::Hand,
    }
}

/// The workstation whose screen a player has open, and where it stands.
pub(crate) fn workstation(world: &World, player: Entity) -> Option<(Workstation, Entity, BlockPos)> {
    let viewing = world.get::<Viewing>(player)?;
    match viewing.screen {
        Screen::Workstation(workstation) => Some((workstation, viewing.layer, viewing.pos)),
        _ => None,
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
    if super::kind_at(world, viewing.layer, viewing.pos).is_some_and(|kind| kind.has_lid) {
        crate::blocks::vibrate_at(world, viewing.layer, viewing.pos, crate::blocks::GameEvent::ContainerClose, true);
    }
}

fn stop_viewing(world: &mut World, player: Entity) -> Option<Viewing> {
    let viewing = world.get::<Viewing>(player).copied()?;
    world.entity_mut(player).remove::<Viewing>();
    crate::life_events::closed(world, player, viewing.pos);
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
/// block that screen showed. BDS's answer names no screen type, whatever the client's named
/// (research/captures/bds-lids.txt).
pub(crate) fn dispatch_closes(world: &mut World) {
    let closes = world.resource_mut::<PacketQueues>().take::<ContainerClose>();
    for (player, closed) in closes {
        let window = closed.window_id.to_raw();
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            out.push(&ContainerClose { window_type: WindowType::None, server: false, ..closed });
        }
        if world.get::<Viewing>(player).is_some_and(|viewing| i64::from(viewing.window) == window) {
            close(world, player);
        }
        crate::creatures::stop_trading(world, player, window);
        inventory::clear_grid(world, player);
    }
}

/// Every [`lid::REPEAT`] ticks from its opening, shows an open lid again: to those who came by
/// since (research/captures/bds-shut.txt).
pub(crate) fn show_lids_again(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(open) = world.get_resource::<super::OpenContainers>() else { return };
    let is_due = |chest: &&Chest| !chest.viewers.is_empty() && now > chest.opened_at && (now - chest.opened_at) % lid::REPEAT == 0;
    let lids: Vec<(Entity, Vec<BlockPos>)> = open.chests.iter().filter(|(_, chest)| is_due(chest)).map(|((layer, _), chest)| (*layer, chest.halves.clone())).collect();
    for (layer, halves) in lids {
        lid::show_open(world, layer, &halves);
    }
}
