//! Armour stands take what players hand them and change pose, as BDS's do: `ArmorStand::getInteraction`
//! (1429b5e20) picks the slot, its swap (1429ba2c0) trades the stand's item for the player's, and a
//! sneaking click moves the stand to its next pose (142a044f0). Evidence: docs/research/mob-kinds.md
//! "Armour stands". Hits knock them over (hits.rs).

mod hits;

pub(super) use hits::{struck, tick};

use acacia_proto::packets::{MobArmorEquipment, MobEquipment, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, WindowID};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::GameMode;
use sapling_world::ItemStack;

use crate::body::Body;
use crate::components::{InLayer, Mode, NetId};
use crate::entities::{self, Kind, Stored};
use crate::inventory::{self, Inventory, Place};
use crate::movement::Motion;
use crate::mobs;

const STAND: &str = "minecraft:armor_stand";
const POSES: i32 = 13;
const MAIN: usize = 0;
const OFF: usize = 1;
const HEAD: usize = 2;
const CHEST: usize = 3;
const LEGS: usize = 4;
const FEET: usize = 5;
/// Its eyes, where an armour piece put on it is heard from, as a share of its height.
const EYES: f32 = 0.85;

/// What a stand holds by BDS's equipment slots (main hand, offhand, head, chest, legs, feet) and its pose.
#[derive(Component, Clone, Default)]
pub(super) struct Gear {
    slots: [Option<ItemStack>; 6],
    pose: i32,
}

/// The slot a click `height` above the stand's feet is on (`getInteraction`); `None` below its feet's.
fn clicked(height: f32) -> Option<usize> {
    match height {
        h if (0.1..0.55).contains(&h) => Some(FEET),
        h if (0.9..1.599_999_9).contains(&h) => Some(CHEST),
        h if (0.4..1.2).contains(&h) => Some(LEGS),
        h if h >= 1.6 => Some(HEAD),
        _ => None,
    }
}

/// The stand's slot an item is worn in.
fn worn_in(name: &str) -> Option<usize> {
    inventory::worn_in(name).map(|piece| HEAD + piece)
}

fn gear_of(world: &World, stand: Entity) -> Gear {
    if let Some(gear) = world.get::<Gear>(stand) {
        return gear.clone();
    }
    let Some(compound) = world.get::<Stored>(stand).map(|stored| &stored.compound) else { return Gear::default() };
    let Some(actor) = sapling_world::actors(compound).ok().and_then(|actors| actors.into_iter().next()) else { return Gear::default() };
    let worn = |slot: usize| actor.armor.get(slot).cloned().flatten();
    Gear { slots: [actor.mainhand, actor.offhand, worn(0), worn(1), worn(2), worn(3)], pose: sapling_world::actor_pose(compound).unwrap_or(0) }
}

fn pose_data(id: u64, pose: i32) -> SetEntityData {
    let item = MetadataDictionaryItem { key: Key::ArmorStandPoseIndex, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(pose)) };
    SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

/// What shows the stand's gear: both hands, then its armour, as BDS sent them.
fn gear_packets(id: u64, gear: &Gear) -> Vec<Bytes> {
    let stack = |slot: usize| gear.slots[slot].as_ref().map(inventory::stack);
    let item = |slot: usize| stack(slot).and_then(|stack| sapling_data::mob_hand(id, &stack, slot == OFF)).map_or_else(sapling_data::no_item, |held| held.item);
    let main = MobEquipment { runtime_entity_id: id, item: item(MAIN), slot: 0, selected_slot: 0, window_id: WindowID::Inventory };
    let off = MobEquipment { runtime_entity_id: id, item: item(OFF), slot: 1, selected_slot: 1, window_id: WindowID::Offhand };
    let armour: MobArmorEquipment = sapling_data::mob_armor(id, &[stack(HEAD), stack(CHEST), stack(LEGS), stack(FEET)]);
    vec![sapling_net::encode(&main), sapling_net::encode(&off), sapling_net::encode(&armour)]
}

/// Keeps the stand's gear, tells its viewers `told`, and has players who come later shown it.
fn show(world: &mut World, stand: Entity, gear: Gear, told: Vec<Bytes>) {
    let (Some(id), Some(layer), Some(body)) = (world.get::<NetId>(stand).map(|id| id.0), world.get::<InLayer>(stand).map(|layer| layer.0), world.get::<Body>(stand).copied()) else { return };
    let [head, chest, legs, feet] = [HEAD, CHEST, LEGS, FEET].map(|slot| gear.slots[slot].clone());
    let hands = [gear.slots[MAIN].clone(), gear.slots[OFF].clone()];
    entities::restore(world, stand, |compound, registry| sapling_world::geared_actor(compound, &[head, chest, legs, feet], &hands, gear.pose, registry).ok());
    if let Some(added) = mobs::announced(world, stand).and_then(|announced| announced.into_iter().next()) {
        let mut announcement = vec![added];
        announcement.extend(gear_packets(id, &gear));
        announcement.push(sapling_net::encode(&pose_data(id, gear.pose)));
        mobs::reannounce(world, stand, announcement);
    }
    world.entity_mut(stand).insert(gear);
    entities::tell_viewers(world, stand, layer, body.at.map(|c| c.floor() as i32), told);
}

/// `player` clicked `stand` at `click` (the world point hit): false unless it is an armour stand
/// that took or gave something or changed pose.
pub(crate) fn use_on(world: &mut World, player: Entity, stand: Entity, click: [f32; 3]) -> bool {
    if world.get::<Kind>(stand).is_none_or(|kind| kind.0 != STAND) {
        return false;
    }
    let mode = world.get::<Mode>(player).map_or(GameMode::Survival, |mode| mode.0);
    let (Some(id), Some(body)) = (world.get::<NetId>(stand).map(|id| id.0), world.get::<Body>(stand).copied()) else { return false };
    if mode == GameMode::Spectator {
        return false;
    }
    let mut gear = gear_of(world, stand);
    if world.get::<Motion>(player).is_some_and(|motion| motion.pose.sneaking) {
        gear.pose = (gear.pose + 1) % POSES;
        let told = vec![sapling_net::encode(&pose_data(id, gear.pose))];
        show(world, stand, gear, told);
        return true;
    }
    let Some(on) = clicked(click[1] - body.at[1]) else { return false };
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref().map(|held| held.stack.clone()));
    let slot = match &held {
        Some(stack) => worn_in(&stack.name).unwrap_or(MAIN),
        None if on == CHEST && gear.slots[MAIN].is_some() => MAIN,
        None => on,
    };
    if !swap(world, player, &mut gear, slot, held, mode == GameMode::Creative) {
        return false;
    }
    let mut told = gear_packets(id, &gear);
    if let Some(sound) = gear.slots[slot].as_ref().filter(|_| slot >= HEAD).and_then(|piece| inventory::equip_sound_name(&piece.name)) {
        told.push(super::hunting::sound(&sound, id, STAND, &body, EYES));
    }
    told.push(super::flight::sound("mob.armor_stand.place", body.at.map(|c| c + 0.5), ""));
    show(world, stand, gear, told);
    true
}

/// BDS's swap of the stand's `slot` with what the player holds: nothing for the same stack or two
/// empty hands; a creative player's item is copied onto an empty slot; one of a stack of two or
/// more goes on and the stand's item goes into the inventory; otherwise the two trade places.
fn swap(world: &mut World, player: Entity, gear: &mut Gear, slot: usize, held: Option<ItemStack>, creative: bool) -> bool {
    let on = gear.slots[slot].clone();
    if held == on {
        return false;
    }
    let Some(stack) = held else {
        gear.slots[slot] = None;
        return inventory::change(world, player, |inventory| inventory.put(Place::Main(inventory.selected.into()), on)).unwrap_or(false);
    };
    let one = ItemStack { count: 1, ..stack.clone() };
    if creative && on.is_none() {
        gear.slots[slot] = Some(one);
        return true;
    }
    if stack.count >= 2 {
        let fits = on.as_ref().is_none_or(|on| world.get::<Inventory>(player).is_some_and(|inventory| inventory.has_room_for(on)));
        if !fits {
            return false;
        }
        gear.slots[slot] = Some(one);
        inventory::change(world, player, |inventory| {
            if !creative {
                let _ = inventory.take(Place::Main(inventory.selected.into()), 1);
            }
            if let Some(on) = &on {
                inventory.add(on);
            }
        });
        return true;
    }
    gear.slots[slot] = Some(stack);
    inventory::change(world, player, |inventory| inventory.put(Place::Main(inventory.selected.into()), on)).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_picks_the_slot_at_its_height() {
        assert_eq!([0.05, 0.3, 0.6, 1.0, 1.65, 1.9].map(clicked), [None, Some(FEET), Some(LEGS), Some(CHEST), Some(HEAD), Some(HEAD)]);
        assert_eq!(["minecraft:diamond_helmet", "minecraft:iron_boots", "minecraft:elytra", "minecraft:iron_sword"].map(worn_in), [Some(HEAD), Some(FEET), Some(CHEST), None]);
    }
}
