//! A mount's tack and goods put by `/replaceitem entity` (research/captures/bds-fixreplace.txt):
//! `slot.saddle` on the horse family, heard as `saddle` at its feet; `slot.armor` on a horse
//! (horse armour or a carpet) or a llama (a carpet; anything else taken unseen), heard as `armor`
//! mid-body; `slot.chest` into a carried chest's goods; `slot.inventory` over the slots the mob has,
//! tack first: a horse's saddle and armour, a donkey's saddle, a llama's carpet, then the chest.
// TODO: a chested mount's `slot.chest` and `slot.inventory` past the tack were not drilled.

use bevy_ecs::prelude::*;
use sapling_api::{MobSlot, MobSlotRefusal};
use sapling_world::ItemStack;

use super::packs;
use crate::body::Body;
use crate::components::NetId;
use crate::containers;
use crate::creatures::Raised;
use crate::entities::Kind;

const SADDLED: [&str; 5] = ["minecraft:horse", "minecraft:donkey", "minecraft:mule", "minecraft:skeleton_horse", "minecraft:zombie_horse"];
const LLAMAS: [&str; 2] = ["minecraft:llama", "minecraft:trader_llama"];
const HORSE: &str = "minecraft:horse";

/// Where a mount's tack goes.
enum Tacked {
    Saddle,
    Body,
}

/// The tack slots `kind` has, in `slot.inventory`'s order.
fn tack_slots(kind: &str) -> Vec<Tacked> {
    let saddle = SADDLED.contains(&kind).then_some(Tacked::Saddle);
    let body = (kind == HORSE || LLAMAS.contains(&kind)).then_some(Tacked::Body);
    saddle.into_iter().chain(body).collect()
}

/// Whether `mob` is one this module dresses.
pub(crate) fn is_mount(world: &World, mob: Entity) -> bool {
    world.get::<Kind>(mob).is_some_and(|kind| !tack_slots(&kind.0).is_empty())
}

/// Puts `item` in a mount's slot as `/replaceitem entity` does; [`MobSlotRefusal::NoSlot`] for a
/// slot it lacks, and a bad slot past the goods of the chest it carries (none: size 0).
pub(crate) fn set(world: &mut World, mob: Entity, slot: MobSlot, item: Option<ItemStack>) -> Result<(), MobSlotRefusal> {
    let kind = world.get::<Kind>(mob).map(|kind| kind.0.clone()).ok_or(MobSlotRefusal::NoSlot)?;
    let tack = tack_slots(&kind);
    let goods = packs::goods_size(world, mob, &kind);
    match slot {
        MobSlot::Saddle if SADDLED.contains(&kind.as_str()) => put_tack(world, mob, &kind, Tacked::Saddle, item, true),
        MobSlot::Body if kind == HORSE || LLAMAS.contains(&kind.as_str()) => {
            let is_carpet = item.as_ref().is_some_and(|item| item.name.ends_with("_carpet"));
            // A llama took horse armour and showed nothing (bds-fixreplace.txt).
            if LLAMAS.contains(&kind.as_str()) && item.is_some() && !is_carpet {
                return Ok(());
            }
            put_tack(world, mob, &kind, Tacked::Body, item, true)
        }
        MobSlot::Inventory(index) if index < tack.len() => put_tack(world, mob, &kind, tack.into_iter().nth(index).expect("in range"), item, false),
        MobSlot::Inventory(index) if index - tack.len() < goods => put_goods(world, mob, index - tack.len(), item),
        MobSlot::Inventory(_) => Err(MobSlotRefusal::BadSlot { size: tack.len() + goods }),
        MobSlot::Chest(index) if packs::carries_chest(&kind) => match index < goods {
            true => put_goods(world, mob, index, item),
            false => Err(MobSlotRefusal::BadSlot { size: goods }),
        },
        _ => Err(MobSlotRefusal::NoSlot),
    }
}

/// Saddles or unsaddles, or dresses the body; heard as BDS's command was (`saddle` only by
/// `slot.saddle`, `armor` for any body change).
fn put_tack(world: &mut World, mob: Entity, kind: &str, tacked: Tacked, item: Option<ItemStack>, by_name: bool) -> Result<(), MobSlotRefusal> {
    let (Some(id), Some(body)) = (world.get::<NetId>(mob).map(|id| id.0), world.get::<Body>(mob).copied()) else { return Err(MobSlotRefusal::NoSlot) };
    // A saddle shows only in the flags, sent only when they change (bds-fixreplace.txt).
    let (told, changed) = match tacked {
        Tacked::Saddle => {
            let saddled = item.as_ref().is_some_and(|item| item.name == "minecraft:saddle");
            let was = world.get_mut::<Raised>(mob).map(|mut raised| std::mem::replace(&mut raised.saddled, saddled));
            let changed = was != Some(saddled);
            let word = changed.then(|| crate::entities::mob_flags(world, mob, kind, acacia_proto::types::MetadataFlags1::SADDLED, saddled));
            let sound = (by_name && saddled).then(|| super::packets::sound("saddle", id, kind, body.at));
            (sound.into_iter().chain(word.map(|word| super::packets::flags(id, word))).collect::<Vec<_>>(), changed)
        }
        Tacked::Body => {
            let worn = item.map(|item| ItemStack { count: 1, ..item });
            let slots = [None, None, None, None, worn.as_ref().map(crate::inventory::stack)];
            let mut told = vec![sapling_net::encode(&sapling_data::mob_armor(id, &slots))];
            packs::wear(world, mob, worn);
            told.push(super::packets::sound("armor", id, kind, [body.at[0], body.at[1] + body.size[1] * 0.5, body.at[2]]));
            (told, true)
        }
    };
    super::tell(world, mob, mob, told);
    packs::tack_changed(world, mob);
    if changed {
        super::retell(world, mob);
    }
    Ok(())
}

/// Writes one slot of the goods of the chest `mob` carries.
fn put_goods(world: &mut World, mob: Entity, index: usize, item: Option<ItemStack>) -> Result<(), MobSlotRefusal> {
    let mut slots = packs::opened(world, mob).ok_or(MobSlotRefusal::NoSlot)?;
    let slot = packs::TACK + index;
    let held = item.map(|stack| containers::new_held(world, stack));
    *slots.get_mut(slot).ok_or(MobSlotRefusal::NoSlot)? = held;
    packs::load(world, mob, slots);
    Ok(())
}
