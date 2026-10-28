//! What others see of a player's items (the hand, the armour, the offhand), the slot a client
//! says it holds, and the inventory screen. MobEquipment shapes are the ones BDS announces a
//! stored mob's items with (`bds_entity_probe`); a player's hand names its hotbar slot.

use acacia_proto::packets::{Interact, InteractActionId, MobEquipment};
use acacia_proto::types::{ItemV4, WindowID, WindowIDVarint};
use bevy_ecs::prelude::*;
use bytes::Bytes;

use super::{HOTBAR, Held, Inventory, UI, stack, wire};
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

    /// One of the player's own windows as its client is sent it.
    fn window(&self, window: WindowIDVarint) -> Vec<ItemV4> {
        match window {
            WindowIDVarint::Inventory => self.main.iter().map(|slot| wire(slot, true)).collect(),
            // The fifth is the body slot, which players leave empty.
            WindowIDVarint::Armor => self.armor.iter().map(|slot| wire(slot, true)).chain([sapling_data::no_item()]).collect(),
            WindowIDVarint::Offhand => vec![wire(&self.offhand, true)],
            _ => {
                let shown = |slot: usize| if slot == 0 { Some(&self.cursor) } else { self.grid.get(slot) };
                (0..UI).map(|slot| shown(slot).map_or_else(sapling_data::no_item, |held| wire(held, true))).collect()
            }
        }
    }

    /// Sends the client every window and then its held slot, selecting it or not, as BDS does at
    /// join, after `/give` and wherever it shows them all (research/captures/bds-chest-moves.txt).
    pub fn send_all(&self, out: &mut Outgoing, select_slot: bool) {
        for window in [WindowIDVarint::Inventory, WindowIDVarint::Armor, WindowIDVarint::Ui, WindowIDVarint::Offhand] {
            out.push(&sapling_data::inventory_content(window, self.window(window)));
        }
        out.push(&sapling_data::player_hotbar(self.selected, select_slot));
    }

    /// Sends the client one of its windows alone, as BDS shows an offhand item that wore or went.
    pub fn send_window(&self, out: &mut Outgoing, window: WindowIDVarint) {
        out.push(&sapling_data::inventory_content(window, self.window(window)));
    }
}

/// The item in a player's hand, as AddPlayer and MobEquipment carry it: without a stack id.
pub(crate) fn in_hand(inventory: &Inventory) -> ItemV4 {
    wire(inventory.in_hand(), false)
}

fn hand(id: u64, inventory: &Inventory) -> MobEquipment {
    MobEquipment { runtime_entity_id: id, item: in_hand(inventory), slot: inventory.selected, selected_slot: inventory.selected, window_id: WindowID::Inventory }
}

/// Shows a player's viewers its hand again, as BDS did after a place in creative mode
/// (research/captures/bds-place-faces.txt).
pub(crate) fn show_hand(world: &mut World, player: Entity) {
    let Some(packet) = world.get::<NetId>(player).zip(world.get::<Inventory>(player)).map(|(id, inventory)| hand(id.0, inventory)) else { return };
    world.resource_mut::<ToViewers>().push(player, &packet);
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
pub(crate) fn select(mut packets: ResMut<PacketQueues>, mut inputs: ResMut<crate::player_events::Inputs>, mut players: Query<&mut Inventory>) {
    for (entity, packet) in packets.take::<MobEquipment>() {
        let Ok(mut inventory) = players.get_mut(entity) else { continue };
        if packet.window_id == WindowID::Inventory && usize::from(packet.selected_slot) < HOTBAR && inventory.selected != packet.selected_slot {
            inputs.held(entity, inventory.selected, packet.selected_slot);
            (inventory.selected, inventory.unsaved) = (packet.selected_slot, true);
        }
    }
}

/// Opens the inventory screen for a client that asks, which it waits for (as it does for the
/// close: `containers::dispatch_closes`). The screen names no entity, as from BDS
/// (`ServerPlayer::openInventory` passes -1). A rider's asks go to riding: to leave, and to open
/// what it rides when that carries goods.
pub(crate) fn screens(
    mut packets: ResMut<PacketQueues>,
    mut leaving: ResMut<crate::riding::Leaving>,
    mut cargo: ResMut<crate::vehicles::CargoAsked>,
    carriers: Query<(), With<crate::vehicles::Cargo>>,
    mut players: Query<(&Position, &mut Inventory, &mut Outgoing, Option<&crate::riding::Riding>)>,
) {
    for (entity, interact) in packets.take::<Interact>() {
        let Ok((position, mut inventory, mut out, riding)) = players.get_mut(entity) else { continue };
        let carrier = riding.map(|riding| riding.vehicle).filter(|vehicle| carriers.contains(*vehicle));
        if interact.action_id == InteractActionId::LeaveVehicle && riding.is_some_and(|riding| riding.id == interact.target_entity_id) {
            leaving.0.push(entity);
        } else if let (InteractActionId::OpenInventory, Some(vehicle)) = (interact.action_id, carrier) {
            cargo.0.push((entity, vehicle));
        } else if interact.action_id == InteractActionId::OpenInventory {
            let (window, block) = (inventory.next_window(), position.feet.map(|c| c.floor() as i32));
            out.push(&sapling_data::inventory_screen(WindowID::from_raw(window.into()), block, NO_ENTITY));
        }
    }
}
