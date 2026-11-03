//! Armour and hand items `/replaceitem entity` puts on a mob, as BDS showed them on a zombie
//! (research/captures/bds-fixreplace.txt): a piece is shown in MobArmorEquipment and heard as
//! put on, at the head for a helmet and mid-body for the rest; something no slot wears (stone on
//! the feet) is taken and shown to no one; the main hand is shown silently, the offhand heard.
// TODO: kept in no world folder, and no mob's harm weighs this armour (mob armour is the mobs group's).

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

/// What commands have put on a mob.
#[derive(Component, Default, Clone)]
pub(crate) struct Gear {
    armor: [Option<ItemStack>; 4],
    main: Option<ItemStack>,
    off: Option<ItemStack>,
}

impl Gear {
    fn armor_packet(&self, id: u64) -> Bytes {
        let shown: Vec<_> = self.armor.iter().enumerate().map(|(slot, piece)| piece.as_ref().filter(|piece| worn_in(&piece.name) == Some(slot)).map(stack)).collect();
        sapling_net::encode(&sapling_data::mob_armor(id, &shown))
    }

    fn hand_packet(id: u64, item: Option<&ItemStack>, offhand: bool) -> Bytes {
        let held = item.and_then(|item| sapling_data::mob_hand(id, &stack(item), offhand));
        let window_id = if offhand { acacia_proto::types::WindowID::Offhand } else { acacia_proto::types::WindowID::Inventory };
        let empty = acacia_proto::packets::MobEquipment { runtime_entity_id: id, item: sapling_data::no_item(), slot: 0, selected_slot: 0, window_id };
        sapling_net::encode(&held.unwrap_or(empty))
    }
}

/// Puts `item` in a mob's armour or hand slot; [`MobSlotRefusal::NoSlot`] for any other slot.
pub(crate) fn set(world: &mut World, mob: Entity, slot: MobSlot, item: Option<ItemStack>) -> Result<(), MobSlotRefusal> {
    if world.get::<Creature>(mob).is_none() {
        return Err(MobSlotRefusal::NoSlot);
    }
    let mut gear = world.get::<Gear>(mob).cloned().unwrap_or_default();
    let (Some(id), Some(body), Some(kind)) = (world.get::<NetId>(mob).map(|id| id.0), world.get::<Body>(mob).copied(), world.get::<Kind>(mob).map(|kind| kind.0.clone())) else {
        return Err(MobSlotRefusal::NoSlot);
    };
    let mid = [body.at[0], body.at[1] + body.size[1] * 0.5, body.at[2]];
    let mut told = Vec::new();
    match slot {
        MobSlot::Armor(piece) => {
            let piece = usize::from(piece.min(3));
            let was_shown = gear.armor[piece].as_ref().is_some_and(|old| worn_in(&old.name) == Some(piece));
            let worn = item.as_ref().is_some_and(|new| worn_in(&new.name) == Some(piece));
            let sound = item.as_ref().filter(|_| worn).and_then(|new| equip_sound_name(&new.name));
            gear.armor[piece] = item;
            if worn || was_shown {
                told.push(gear.armor_packet(id));
            }
            let at = if piece == 0 { [body.at[0], body.at[1] + body.size[1] * HEAD_UP, body.at[2]] } else { mid };
            told.extend(sound.map(|sound| crate::riding::sound_packet(&sound, id, &kind, at)));
        }
        MobSlot::MainHand => {
            told.push(Gear::hand_packet(id, item.as_ref(), false));
            gear.main = item;
        }
        MobSlot::OffHand => {
            told.push(Gear::hand_packet(id, item.as_ref(), true));
            let sound = item.as_ref().map(|new| equip_sound_name(&new.name).unwrap_or_else(|| GENERIC.to_owned()));
            told.extend(sound.map(|sound| crate::riding::sound_packet(&sound, id, &kind, mid)));
            gear.off = item;
        }
        _ => return Err(MobSlotRefusal::NoSlot),
    }
    world.entity_mut(mob).insert(gear);
    if let Some(layer) = world.get::<InLayer>(mob).map(|layer| layer.0) {
        entities::tell_viewers(world, mob, layer, body.at.map(|c| c.floor() as i32), told);
    }
    Ok(())
}

/// What shows a mob's command gear to one who comes to see it.
pub(super) fn shown(world: &World, mob: Entity, id: u64) -> Vec<Bytes> {
    let Some(gear) = world.get::<Gear>(mob) else { return Vec::new() };
    let mut shown = Vec::new();
    if gear.armor.iter().any(Option::is_some) {
        shown.push(gear.armor_packet(id));
    }
    shown.extend(gear.main.as_ref().map(|main| Gear::hand_packet(id, Some(main), false)));
    shown.extend(gear.off.as_ref().map(|off| Gear::hand_packet(id, Some(off), true)));
    shown
}
