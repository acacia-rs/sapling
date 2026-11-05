//! Armour and hand items `/replaceitem entity` puts on a mob, as BDS showed them on a zombie
//! (research/captures/bds-fixreplace.txt): a piece is worn as [`super::worn`] keeps it, shown in
//! MobArmorEquipment and heard as put on, at the head for a helmet and mid-body for the rest;
//! something no slot wears (stone on the feet) is taken and shown to no one; the main hand is
//! shown silently, the offhand heard. The hands are kept in the mob's `Mainhand` and `Offhand`.

use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{MobSlot, MobSlotRefusal};
use sapling_world::ItemStack;

use super::Creature;
use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::entities::{self, Kind};
use crate::inventory::{equip_sound_name, stack, worn_in};

const HEAD_UP: f32 = 0.85;
const GENERIC: &str = "armor.equip_generic";

/// What commands have put in a mob's hands.
#[derive(Component, Default, Clone)]
pub(crate) struct Gear {
    main: Option<ItemStack>,
    off: Option<ItemStack>,
}

fn hand_packet(id: u64, item: Option<&ItemStack>, offhand: bool) -> Bytes {
    let held = item.and_then(|item| sapling_data::mob_hand(id, &stack(item), offhand));
    let window_id = if offhand { acacia_proto::types::WindowID::Offhand } else { acacia_proto::types::WindowID::Inventory };
    let empty = acacia_proto::packets::MobEquipment { runtime_entity_id: id, item: sapling_data::no_item(), slot: 0, selected_slot: 0, window_id };
    sapling_net::encode(&held.unwrap_or(empty))
}

/// What the mob holds and wears, by [`super::turning::GEAR`]'s slots: its piglin hands, else
/// what commands or its stored look gave it.
pub(super) fn gear_of(world: &World, mob: Entity) -> Vec<(MobSlot, Option<ItemStack>)> {
    let stored = world.get::<Creature>(mob).and_then(|creature| creature.look.as_ref()).map(|look| Gear { main: look.mainhand.clone(), off: look.offhand.clone() });
    let gear = world.get::<Gear>(mob).cloned().or(stored).unwrap_or_default();
    let [main, off] = super::piglins::hands(world, mob).unwrap_or([gear.main, gear.off]);
    let worn = super::worn::pieces(world, mob);
    super::turning::GEAR.into_iter().zip([main, off].into_iter().chain(worn)).filter(|(_, item)| item.is_some()).collect()
}

/// [`set`] shown with no sound: a hand in `MobEquipment`, armour in `MobArmorEquipment`.
pub(super) fn set_quietly(world: &mut World, mob: Entity, slot: MobSlot, item: Option<ItemStack>) -> Option<()> {
    let MobSlot::Armor(piece) = slot else { return set(world, mob, slot, item).ok() };
    super::worn::put_on(world, mob, usize::from(piece), item).then_some(())
}

/// Puts `item` in a new mob's main hand, kept and shown to those who come to see it, telling no one now.
pub(super) fn hold_from_birth(world: &mut World, mob: Entity, item: ItemStack) {
    let mut gear = world.get::<Gear>(mob).cloned().unwrap_or_default();
    gear.main = Some(item);
    let hands = [gear.main.clone(), gear.off.clone()];
    world.entity_mut(mob).insert(gear);
    entities::restore(world, mob, |compound, registry| sapling_world::handed_actor(compound, &hands, registry).ok());
    super::shown::reshow(world, mob);
}

/// Puts `item` in a mob's armour or hand slot; [`MobSlotRefusal::NoSlot`] for any other slot.
pub(crate) fn set(world: &mut World, mob: Entity, slot: MobSlot, item: Option<ItemStack>) -> Result<(), MobSlotRefusal> {
    if world.get::<Creature>(mob).is_none() {
        return Err(MobSlotRefusal::NoSlot);
    }
    let (Some(id), Some(body), Some(kind)) = (world.get::<NetId>(mob).map(|id| id.0), world.get::<Body>(mob).copied(), world.get::<Kind>(mob).map(|kind| kind.0.clone())) else {
        return Err(MobSlotRefusal::NoSlot);
    };
    let mid = [body.at[0], body.at[1] + body.size[1] * 0.5, body.at[2]];
    let stored = world.get::<Creature>(mob).and_then(|creature| creature.look.as_ref()).map(|look| Gear { main: look.mainhand.clone(), off: look.offhand.clone() });
    let mut gear = world.get::<Gear>(mob).cloned().or(stored).unwrap_or_default();
    let mut told = Vec::new();
    let is_hand = matches!(slot, MobSlot::MainHand | MobSlot::OffHand);
    match slot {
        MobSlot::Armor(piece) => {
            let piece = usize::from(piece.min(3));
            let worn = item.as_ref().is_some_and(|new| worn_in(&new.name) == Some(piece));
            let sound = item.as_ref().filter(|_| worn).and_then(|new| equip_sound_name(&new.name));
            let (shown, off) = super::worn::wear(world, mob, piece, item).ok_or(MobSlotRefusal::NoSlot)?;
            let was_shown = off.is_some_and(|old| worn_in(&old.name) == Some(piece));
            told.extend(shown.filter(|_| worn || was_shown));
            let at = if piece == 0 { [body.at[0], body.at[1] + body.size[1] * HEAD_UP, body.at[2]] } else { mid };
            told.extend(sound.map(|sound| crate::riding::sound_packet(&sound, id, &kind, at)));
        }
        MobSlot::MainHand => {
            told.push(hand_packet(id, item.as_ref(), false));
            gear.main = item;
        }
        MobSlot::OffHand => {
            told.push(hand_packet(id, item.as_ref(), true));
            let sound = item.as_ref().map(|new| equip_sound_name(&new.name).unwrap_or_else(|| GENERIC.to_owned()));
            told.extend(sound.map(|sound| crate::riding::sound_packet(&sound, id, &kind, mid)));
            gear.off = item;
        }
        _ => return Err(MobSlotRefusal::NoSlot),
    }
    let hands = [gear.main.clone(), gear.off.clone()];
    world.entity_mut(mob).insert(gear);
    if is_hand {
        entities::restore(world, mob, |compound, registry| sapling_world::handed_actor(compound, &hands, registry).ok());
    }
    if let Some(layer) = world.get::<InLayer>(mob).map(|layer| layer.0) {
        entities::tell_viewers(world, mob, layer, body.at.map(|c| c.floor() as i32), told);
    }
    Ok(())
}

/// What shows a mob's command-held items to one who comes to see it.
pub(super) fn shown(world: &World, mob: Entity, id: u64) -> Vec<Bytes> {
    let Some(gear) = world.get::<Gear>(mob) else { return Vec::new() };
    let main = gear.main.as_ref().map(|main| hand_packet(id, Some(main), false));
    let off = gear.off.as_ref().map(|off| hand_packet(id, Some(off), true));
    main.into_iter().chain(off).collect()
}
