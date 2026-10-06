//! A player's items between visits: asked of the first world's storage at login, handed back
//! when the player leaves, every few seconds while they changed, and when the shard stops.

use bevy_ecs::prelude::*;
use sapling_world::{PlayerData, PlayerLoad};

use super::{ARMOR, ENDER, Held, Inventory, MAIN, Place};
use crate::chunks::UNLOAD_INTERVAL;
use crate::components::{DefaultLayer, Identity, Layer, NetId, Outgoing, Stage, Tick};
use crate::experience::{self, Experience};
use crate::handoff::{Peers, ShardMsg};

fn fill(inventory: &mut Inventory, data: PlayerData) {
    for (slot, stack) in data.main.into_iter().take(MAIN).enumerate() {
        inventory.put(Place::Main(slot), stack);
    }
    for (slot, stack) in data.armor.into_iter().take(ARMOR).enumerate() {
        inventory.put(Place::Armor(slot), stack);
    }
    inventory.put(Place::Offhand, data.offhand);
    for (slot, stack) in data.ender.into_iter().take(ENDER).enumerate() {
        inventory.ender[slot] = stack.map(|stack| Held { stack, id: inventory.new_id() });
    }
    inventory.experience = Experience::new(data.level, data.progress);
    (inventory.selected, inventory.unsaved) = (data.selected, false);
}

/// What storage keeps; stacks on the cursor and in the crafting grid go back among the others.
fn kept(inventory: &Inventory) -> PlayerData {
    let mut inventory = inventory.clone();
    for held in inventory.cursor.take().into_iter().chain(std::mem::take(&mut inventory.grid).into_iter().flatten()) {
        inventory.add(&held.stack);
    }
    let stacks = |slots: Vec<Option<Held>>| slots.into_iter().map(|slot| slot.map(|held| held.stack)).collect();
    let (level, progress) = (inventory.experience.level, inventory.experience.progress);
    let (main, armor, ender, offhand) = (stacks(inventory.main), stacks(inventory.armor), stacks(inventory.ender), inventory.offhand.map(|held| held.stack));
    PlayerData { main, armor, offhand, selected: inventory.selected, ender, level, progress }
}

/// Gives the players storage answered for their items.
pub(crate) fn load(mut layers: Query<&mut Layer>, mut players: Query<(&Identity, &mut Inventory)>) {
    for mut layer in &mut layers {
        for (id, answer) in layer.world.take_players() {
            let Some((_, mut inventory)) = players.iter_mut().find(|(identity, inventory)| !inventory.loaded && identity.0.uuid.to_string() == id) else { continue };
            (inventory.loaded, inventory.keep) = (true, answer != PlayerLoad::Unreadable);
            if let PlayerLoad::Stored(data) = answer {
                fill(&mut inventory, data);
            }
        }
    }
}

/// Sends each joining player its windows once it has items to show, in BDS's order: inventory,
/// armour, the screen's own slots, offhand, then the slot in hand.
pub(crate) fn send_own(mut players: Query<(&NetId, &Stage, &mut Inventory, &mut Outgoing)>) {
    for (id, stage, mut inventory, mut out) in &mut players {
        if *stage >= Stage::StartGame && inventory.loaded && !inventory.sent {
            inventory.send_all(&mut out);
            out.push(&sapling_data::player_hotbar(inventory.selected, true));
            if inventory.experience != Experience::default() {
                out.push(&experience::attributes(id, &inventory.experience, true));
            }
            inventory.sent = true;
        }
    }
}

/// Has a player's items kept if they changed since they last were: by this shard's storage, or
/// by the entry shard's when this is another (`Peers::entry`).
pub(crate) fn save(world: &mut World, player: Entity) {
    let Some(id) = world.get::<Identity>(player).map(|identity| identity.0.uuid.to_string()) else { return };
    let Some(mut inventory) = world.get_mut::<Inventory>(player).filter(|inventory| inventory.keep && inventory.unsaved) else { return };
    inventory.unsaved = false;
    let data = kept(&inventory);
    let (index, entry) = (world.resource::<Peers>().index, world.resource::<Peers>().entry);
    if index == entry {
        return keep_here(world, &id, data);
    }
    if world.resource::<Peers>().tell(entry, ShardMsg::KeepPlayer { id, data: Box::new(data) }).is_err() {
        tracing::warn!("the shard that keeps players is gone: a player's items are not saved");
    }
}

/// Hands a player's items to the storage of this shard's first world.
pub(crate) fn keep_here(world: &mut World, id: &str, data: PlayerData) {
    let layer = world.resource::<DefaultLayer>().0;
    if let Some(mut layer) = world.get_mut::<Layer>(layer) {
        layer.world.save_player(id, data);
    }
}

pub(crate) fn save_all(world: &mut World) {
    let players: Vec<Entity> = world.query_filtered::<Entity, With<Inventory>>().iter(world).collect();
    players.into_iter().for_each(|player| save(world, player));
}

/// On the ticks columns are saved, saves the players whose items changed.
pub(crate) fn save_changed(world: &mut World) {
    if world.resource::<Tick>().0 % UNLOAD_INTERVAL == 0 {
        save_all(world);
    }
}
