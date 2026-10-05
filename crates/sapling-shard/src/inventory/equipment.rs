//! What others see of a player's items (the hand, the armour, the offhand), the slot a client
//! says it holds, and the inventory screen. MobEquipment shapes are the ones BDS announces a
//! stored mob's items with (`bds_entity_probe`); a player's hand names its hotbar slot.

use acacia_proto::packets::{Interact, InteractActionId, MobEquipment};
use acacia_proto::types::{ItemV4, WindowID};
use bevy_ecs::prelude::*;
use bytes::Bytes;

use super::{HOTBAR, Held, Inventory, stack, wire};
use crate::components::{NetId, Outgoing, PacketQueues, Position, Stage};
use crate::visibility::ToViewers;

/// Window ids a screen can have; the client counts every screen it opens.
const LAST_WINDOW: u8 = 99;
/// `runtime_entity_id` of a screen that is no entity's.
const NO_ENTITY: i64 = -1;

impl Inventory {
    /// The id of the screen the client is about to be shown.
    pub(crate) fn next_window(&mut self) -> u8 {
        self.window = self.window % LAST_WINDOW + 1;
        self.window
    }
}

/// The item in a player's hand, as AddPlayer and MobEquipment carry it: without a stack id.
pub(crate) fn in_hand(inventory: &Inventory) -> ItemV4 {
    wire(inventory.in_hand(), false)
}

fn hand(id: u64, inventory: &Inventory) -> MobEquipment {
    MobEquipment { runtime_entity_id: id, item: in_hand(inventory), slot: inventory.selected, selected_slot: inventory.selected, window_id: WindowID::Inventory }
}

fn offhand(id: u64, inventory: &Inventory) -> MobEquipment {
    MobEquipment { runtime_entity_id: id, item: wire(&inventory.offhand, false), slot: 0, selected_slot: 0, window_id: WindowID::Offhand }
}

fn armor(id: u64, inventory: &Inventory) -> acacia_proto::packets::MobArmorEquipment {
    let slots: Vec<_> = inventory.armor.iter().map(|slot| slot.as_ref().map(|held: &Held| stack(&held.stack))).collect();
    sapling_data::mob_armor(id, &slots)
}

/// What follows a player's AddPlayer, which already shows the hand: armour and offhand, if any.
pub(crate) fn spawn_packets(id: u64, inventory: &Inventory) -> Vec<Bytes> {
    let worn = inventory.worn();
    let armor = worn[1..5].iter().any(|id| *id != 0).then(|| sapling_net::encode(&armor(id, inventory)));
    let offhand = (worn[5] != 0).then(|| sapling_net::encode(&offhand(id, inventory)));
    armor.into_iter().chain(offhand).collect()
}

/// Queues, once per player whose shown items changed, what its viewers need to know.
pub(crate) fn announce(mut to_viewers: ResMut<ToViewers>, mut players: Query<(Entity, &NetId, &Stage, &mut Inventory)>) {
    for (entity, id, stage, mut inventory) in &mut players {
        let now = inventory.worn();
        if *stage != Stage::InWorld || inventory.shown == Some(now) {
            continue;
        }
        // A new viewer learns the current state with AddPlayer; only changes are announced.
        let Some(told) = inventory.shown.replace(now) else { continue };
        if told[0] != now[0] {
            to_viewers.push(entity, &hand(id.0, &inventory));
        }
        if told[1..5] != now[1..5] {
            to_viewers.push(entity, &armor(id.0, &inventory));
        }
        if told[5] != now[5] {
            to_viewers.push(entity, &offhand(id.0, &inventory));
        }
    }
}

/// The hotbar slot each client says it switched to.
pub(crate) fn select(mut packets: ResMut<PacketQueues>, mut players: Query<&mut Inventory>) {
    for (entity, packet) in packets.take::<MobEquipment>() {
        let Ok(mut inventory) = players.get_mut(entity) else { continue };
        if packet.window_id == WindowID::Inventory && usize::from(packet.selected_slot) < HOTBAR && inventory.selected != packet.selected_slot {
            (inventory.selected, inventory.unsaved) = (packet.selected_slot, true);
        }
    }
}

/// Opens the inventory screen for a client that asks, which it waits for (as it does for the
/// close: `containers::dispatch_closes`). The screen names no entity, as from BDS
/// (`ServerPlayer::openInventory` passes -1).
pub(crate) fn screens(mut packets: ResMut<PacketQueues>, mut players: Query<(&Position, &mut Inventory, &mut Outgoing)>) {
    for (entity, interact) in packets.take::<Interact>() {
        let Ok((position, mut inventory, mut out)) = players.get_mut(entity) else { continue };
        if interact.action_id == InteractActionId::OpenInventory {
            let (window, block) = (inventory.next_window(), position.feet.map(|c| c.floor() as i32));
            out.push(&sapling_data::inventory_screen(WindowID::from_raw(window.into()), block, NO_ENTITY));
        }
    }
}
