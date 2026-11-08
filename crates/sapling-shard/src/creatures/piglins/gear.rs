//! Weapons and armour a piglin takes up, by its pack's `shareables` priorities (lower is better): it
//! takes one into an empty hand or slot, or over a worse one, as BDS's did (research/captures/
//! bds-q1piglin.txt): a crossbow over an iron sword, an iron sword into an empty hand, a diamond helmet
//! over an iron one, leather boots onto bare feet; not an iron sword over a crossbow, a crossbow over
//! a golden sword (both 2) nor an iron helmet over a diamond one. What it takes is shown a tick on
//! (armour as `MobEquipment` naming its equipment slot, 2 the head to 5 the feet), what it put down
//! is dropped a tick after that, and what it took drops at its death. Every piece offered to a bare
//! piglin was taken (bds-tt1piglin.txt, the iron helmet four times of four; bds-q1piglin.txt's one
//! left lying did not come again).

use acacia_proto::packets::MobEquipment;
use acacia_proto::types::WindowID;
use bevy_ecs::prelude::*;
use sapling_world::ItemStack;

use super::{Hands, data};
use crate::body::Body;
use crate::components::{InLayer, Tick};
use crate::inventory::worn_in;
use crate::{drops, late_sounds};

/// The admired golden items' priority, which no weapon or piece beats but a crossbow equals.
const GOLDEN: u8 = 2;
/// The turtle shell is the pack's 9 with the heads and carved pumpkin (bds-tt1piglin.txt: one took it onto a bare head).
const MATERIALS: [(&str, u8); 7] = [("netherite", 3), ("diamond", 4), ("iron", 5), ("chainmail", 6), ("copper", 7), ("leather", 8), ("turtle", 9)];
const PIECES: [&str; 4] = ["helmet", "chestplate", "leggings", "boots"];
/// A sword's priority by material: one worse than the same armour from copper on, as the pack lists.
const SWORDS: [(&str, u8); 6] = [("netherite", 3), ("diamond", 4), ("iron", 5), ("copper", 6), ("stone", 7), ("wooden", 8)];
/// What it put down falls from this far over its feet.
const DROPPED_UP: f32 = 0.125;

/// What the piglin put down for what it took, each dropped at its tick.
#[derive(Component, Default)]
pub(super) struct Shedding(Vec<(u64, ItemStack)>);

/// The armour slots it filled itself, to drop at its death.
#[derive(Component, Default)]
pub(super) struct PickedWorn([bool; 4]);

fn plain(name: &str) -> &str {
    name.strip_prefix("minecraft:").unwrap_or(name)
}

/// A main-hand item's priority: a crossbow 2, swords and spears by material, golden things 2.
fn weapon(name: &str) -> Option<u8> {
    let plain = plain(name);
    if plain == "crossbow" || plain.starts_with("golden_") {
        return Some(GOLDEN);
    }
    let (material, kind) = plain.rsplit_once('_')?;
    SWORDS.iter().find(|(named, _)| *named == material).filter(|_| kind == "sword" || kind == "spear").map(|(_, priority)| *priority)
}

/// An armour piece's slot and priority; golden pieces 2.
fn armour(name: &str) -> Option<(usize, u8)> {
    let (material, piece) = plain(name).rsplit_once('_')?;
    let slot = PIECES.iter().position(|named| *named == piece)?;
    let priority = if material == "golden" { GOLDEN } else { MATERIALS.iter().find(|(named, _)| *named == material)?.1 };
    Some((slot, priority))
}

fn better(new: u8, old: Option<&ItemStack>, of: impl Fn(&str) -> Option<u8>) -> bool {
    old.is_none_or(|old| of(&old.name).is_some_and(|old| new < old))
}

/// Whether the piglin would take `name` up as a weapon or a piece of armour.
pub(super) fn wants(world: &World, piglin: Entity, name: &str) -> bool {
    if plain(name).starts_with("golden_") {
        return false;
    }
    if let Some(new) = weapon(name) {
        let held = world.get::<Hands>(piglin).and_then(|hands| hands.main.as_ref());
        return better(new, held, weapon);
    }
    let Some((slot, new)) = armour(name) else { return false };
    let worn = crate::creatures::worn::pieces(world, piglin);
    better(new, worn[slot].as_ref(), |name| armour(name).map(|(_, priority)| priority))
}

/// The piglin `id` took `taken` up as a weapon or armour (`wants` said so): it holds or wears it,
/// shown a tick on, and puts down what it had there a tick after that.
pub(super) fn take(world: &mut World, piglin: Entity, id: u64, taken: ItemStack) {
    let Some((layer, body)) = world.get::<InLayer>(piglin).map(|layer| layer.0).zip(world.get::<Body>(piglin).copied()) else { return };
    let now = world.resource::<Tick>().0;
    let cell = body.at.map(|c| c.floor() as i32);
    let (slot, shown, old) = match worn_in(&taken.name).filter(|_| weapon(&taken.name).is_none()) {
        Some(piece) => {
            let Some((_, old)) = crate::creatures::worn::wear(world, piglin, piece, Some(taken.clone())) else { return };
            let mut picked = world.get::<PickedWorn>(piglin).map_or([false; 4], |picked| picked.0);
            picked[piece] = true;
            world.entity_mut(piglin).insert(PickedWorn(picked));
            (2 + piece as u8, taken, old)
        }
        None => {
            let off = world.get_mut::<Hands>(piglin).and_then(|mut hands| hands.off.take());
            let old = world.get::<Hands>(piglin).and_then(|hands| hands.main.clone());
            world.entity_mut(piglin).insert((Hands { main: Some(taken.clone()), off }, super::pickup::PickedMain));
            crate::creatures::reshow(world, piglin);
            (0, taken, old)
        }
    };
    if let Some(item) = sapling_data::mob_hand(id, &data(&shown), false).map(|held| held.item) {
        let told = MobEquipment { runtime_entity_id: id, item, slot, selected_slot: if slot == 0 { 0 } else { 120 }, window_id: WindowID::Inventory };
        late_sounds::play_later(world, 1, layer, cell, sapling_net::encode(&told));
    }
    if let Some(old) = old {
        match world.get_mut::<Shedding>(piglin) {
            Some(mut shedding) => shedding.0.push((now + 2, old)),
            None => drop(world.entity_mut(piglin).insert(Shedding(vec![(now + 2, old)]))),
        }
    }
}

/// The armour the dying piglin took up, taken off it.
pub(super) fn picked_worn(world: &mut World, piglin: Entity) -> Vec<ItemStack> {
    let Some(PickedWorn(picked)) = world.entity_mut(piglin).take::<PickedWorn>() else { return Vec::new() };
    (0..4).filter(|piece| picked[*piece]).filter_map(|piece| crate::creatures::worn::take_off(world, piglin, piece)).collect()
}

/// Drops what each piglin put down whose tick has come.
pub(super) fn shed(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let due: Vec<(Entity, Entity, [f32; 3], Vec<ItemStack>)> = world
        .query::<(Entity, &mut Shedding, &InLayer, &Body)>()
        .iter_mut(world)
        .filter_map(|(piglin, mut shedding, layer, body)| {
            let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut shedding.0).into_iter().partition(|(when, _)| *when <= now);
            shedding.0 = waiting;
            (!due.is_empty()).then(|| (piglin, layer.0, body.at, due.into_iter().map(|(_, item)| item).collect()))
        })
        .collect();
    for (_, layer, [x, y, z], items) in due {
        for item in items {
            drops::let_go(world, layer, item, [x, y + DROPPED_UP, z]);
        }
    }
}
