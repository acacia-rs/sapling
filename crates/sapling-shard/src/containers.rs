//! Blocks that hold items (chests, barrels, hoppers and the three furnaces), kept in the block's entity as
//! the game keeps them, shown to the players who have the block open and changed by their
//! inventory requests (inventory/requests.rs). Two paired chests are one container of 54
//! (pairs.rs); a furnace also burns (furnaces.rs). A crafting table's and a stonecutter's
//! screens open and close the same way, with nothing to hold. What BDS does, and the evidence:
//! docs/research/containers-crafting.md, sections A to D.
// TODO: a hopper holds items but moves none; no shulker or ender chests; a player can stay at an open container from any
// distance; a chest under a solid block still opens; anything goes into a furnace's output slot.

mod lid;
mod pairs;
mod slots;

use std::collections::HashMap;

use acacia_proto::packets::ContainerClose;
use acacia_proto::types::{WindowID, WindowIDVarint, WindowType};
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_data::Station;
use sapling_world::{container_items, new_block_entity};

use crate::components::{Layer, Outgoing, PacketQueues, Tick};
use crate::inventory::{Held, Inventory, wire};
use crate::blocks::keep_entity;
use crate::{furnaces, inventory};

pub(crate) use slots::{new_held, release, resend, rewrite, set_slots, slots_at, slots_of, spill, store, viewers};

/// Stack ids of what a container held when it was opened; players' own count up from 1.
const FIRST_ID: i32 = 1 << 24;
const TABLE: &str = "minecraft:crafting_table";
const STONECUTTER: &str = "minecraft:stonecutter_block";

/// What kind of container a block is.
#[derive(Clone, Copy)]
struct Kind {
    /// The `id` of its block entity.
    entity: &'static str,
    /// A double chest has them twice, the lead half's first.
    slots: usize,
    window: WindowType,
    /// Whether those near see it open and shut.
    has_lid: bool,
}

const CHEST: Kind = Kind { entity: "Chest", slots: 27, window: WindowType::Container, has_lid: true };
const SMELTER_SLOTS: usize = 3;

impl Kind {
    fn is_smelter(self) -> bool {
        self.slots == SMELTER_SLOTS
    }
}

fn kind(block: &str) -> Option<Kind> {
    let smelter = |entity, window| Kind { entity, slots: SMELTER_SLOTS, window, has_lid: false };
    Some(match block {
        "minecraft:chest" | "minecraft:trapped_chest" => CHEST,
        "minecraft:barrel" => Kind { entity: "Barrel", ..CHEST },
        "minecraft:hopper" => Kind { entity: "Hopper", slots: 5, window: WindowType::Hopper, has_lid: false },
        "minecraft:furnace" | "minecraft:lit_furnace" => smelter("Furnace", WindowType::Furnace),
        "minecraft:blast_furnace" | "minecraft:lit_blast_furnace" => smelter("BlastFurnace", WindowType::BlastFurnace),
        "minecraft:smoker" | "minecraft:lit_smoker" => smelter("Smoker", WindowType::Smoker),
        _ => return None,
    })
}

pub(crate) fn block_name(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<&'static str> {
    let level = &world.get::<Layer>(layer)?.world;
    level.registry().get(level.block(x, y, z)).map(|state| state.name)
}

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Container,
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
    per_half: usize,
    slots: Vec<Option<Held>>,
    viewers: Vec<Entity>,
}

/// The containers whose slots the shard holds: each while someone has it open, a furnace also
/// while it has something to burn. By their lead half.
#[derive(Resource, Default)]
pub(crate) struct OpenContainers {
    chests: HashMap<(Entity, BlockPos), Chest>,
    ids: i32,
}

fn opened(world: &mut World) -> Mut<'_, OpenContainers> {
    world.get_resource_or_insert_with(OpenContainers::default)
}

fn content(window: u8, slots: &[Option<Held>]) -> acacia_proto::packets::InventoryContent {
    sapling_data::inventory_content(WindowIDVarint::from_raw(window.into()), slots.iter().map(|slot| wire(slot, true)).collect())
}

/// Gives a container block that was just placed at `pos` its empty block entity, and a chest
/// its other half if one stands beside it; other blocks are left alone.
pub(crate) fn furnish(world: &mut World, layer: Entity, pos: BlockPos, block: &str) {
    if let Some(kind) = kind(block) {
        keep_entity(world, layer, pos, &new_block_entity(kind.entity, pos));
        pairs::pair_placed(world, layer, pos);
    }
}

/// The first `count` slots of what the block entity at `pos` holds, under new stack ids.
fn stored_slots(world: &mut World, layer: Entity, pos: BlockPos, count: usize) -> Vec<Option<Held>> {
    let items = world.get::<Layer>(layer).and_then(|layer| layer.world.block_entity(pos)).map(|entity| container_items(&entity.nbt)).unwrap_or_default();
    let mut slots = vec![None; count];
    for (slot, stack) in items.into_iter().filter(|(slot, _)| usize::from(*slot) < count) {
        slots[usize::from(slot)] = Some(new_held(world, stack));
    }
    slots
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
    let halves = if kind.entity == CHEST.entity { pairs::halves(world, layer, pos) } else { vec![pos] };
    let key = (layer, halves[0]);
    if !opened(world).chests.contains_key(&key) {
        let mut slots = Vec::new();
        for half in &halves {
            slots.extend(stored_slots(world, layer, *half, kind.slots));
        }
        opened(world).chests.insert(key, Chest { halves: halves.clone(), per_half: kind.slots, slots, viewers: Vec::new() });
    }
    let Some((slots, is_first)) = opened(world).chests.get_mut(&key).map(|chest| {
        chest.viewers.push(player);
        (chest.slots.clone(), chest.viewers.len() == 1)
    }) else {
        return false;
    };
    let shown = show_screen(world, player, Viewing { layer, pos: key.1, window: 0, screen: Screen::Container }, kind.window, Some(&slots));
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
/// and its slots are let go unless it is a furnace still at work.
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
    if !furnaces::is_watched(world, viewing.layer, viewing.pos) {
        release(world, viewing.layer, viewing.pos);
    }
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
    let lids: Vec<(Entity, Vec<BlockPos>)> = open.chests.iter().filter(|(_, chest)| !chest.viewers.is_empty()).map(|((layer, _), chest)| (*layer, chest.halves.clone())).collect();
    for (layer, halves) in lids {
        lid::show_open(world, layer, &halves);
    }
}
