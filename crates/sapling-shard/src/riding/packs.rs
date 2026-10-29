//! A tamed mount's inventory (docs/research/mob-life.md "Mount inventories"): slot 0 its saddle,
//! 1 a horse's armour or a llama's carpet, and from 2 the goods of the chest a donkey, mule or
//! llama was given (15 slots, a llama 3 for each point of strength). It opens on the horse screen
//! (`UpdateEquipment` naming the mob and saying what each tack slot takes, no `ContainerOpen`)
//! to a sneaking player's click and to its rider's inventory key; what is in it is kept in
//! the mob's `ChestItems` by slot, and slots 0 and 1 are the tack it shows. It dies spilling the
//! goods ([`spilled`]).
// TODO: a donkey's chest does not switch its component groups.

use acacia_proto::nbt::{self, List, Nbt, Network, Raw, Value};
use acacia_proto::packets::UpdateEquipment;
use acacia_proto::types::{WindowID, WindowType};
use bevy_ecs::prelude::*;
use bytes::BytesMut;
use sapling_world::ItemStack;

use super::mounts::Tack;
use crate::body::Body;
use crate::components::{InLayer, NetId};
use crate::creatures::Raised;
use crate::entities::{self, Kind, Stored};
use crate::inventory::{Held, Inventory};
use crate::{blocks, containers, creatures};

const SADDLE: &str = "minecraft:saddle";
const CHEST: &str = "minecraft:chest";
/// Kinds whose inventory has a saddle slot, and those that take a chest.
const SADDLED: [&str; 5] = ["minecraft:horse", "minecraft:donkey", "minecraft:mule", "minecraft:skeleton_horse", "minecraft:zombie_horse"];
const CHESTED: [&str; 4] = ["minecraft:donkey", "minecraft:mule", "minecraft:llama", "minecraft:trader_llama"];
const LLAMAS: [&str; 2] = ["minecraft:llama", "minecraft:trader_llama"];
const HORSE: &str = "minecraft:horse";
const TACK: usize = 2;
const DONKEY_CHEST: usize = 15;
const PER_STRENGTH: usize = 3;
const HORSE_ARMOR: [&str; 6] = ["leather", "copper", "iron", "golden", "diamond", "netherite"];
const CARPETS: [&str; 18] = [
    "white", "orange", "magenta", "light_blue", "yellow", "lime", "pink", "gray", "light_gray", "cyan", "purple", "blue", "brown", "green", "red", "black", "moss", "pale_moss",
];

/// The slots of a mount's inventory while it has been opened since it came.
#[derive(Component, Clone)]
pub(crate) struct Pack(Vec<Option<Held>>);

fn tamed(world: &World, mob: Entity) -> bool {
    world.get::<Raised>(mob).is_some_and(Raised::is_tamed)
}

fn chested(world: &World, mob: Entity) -> bool {
    world.get::<Raised>(mob).is_some_and(|raised| raised.chested)
}

fn has_pack(kind: &str) -> bool {
    SADDLED.contains(&kind) || LLAMAS.contains(&kind)
}

/// How many slots `mob` of `kind` has: its tack's two and its chest's.
fn size(world: &World, mob: Entity, kind: &str) -> usize {
    let chest = match kind {
        _ if !chested(world, mob) => 0,
        _ if LLAMAS.contains(&kind) => super::strength::of(world, mob).unwrap_or(0) as usize * PER_STRENGTH,
        _ if CHESTED.contains(&kind) => DONKEY_CHEST,
        _ => 0,
    };
    TACK + chest
}

fn stack(name: &str) -> ItemStack {
    ItemStack { name: name.into(), count: 1, damage: 0, block: None, tag: None }
}

/// What the slot `slot` takes: a saddle, a horse's armour or a llama's carpet, else anything.
pub(super) fn takes(kind: &str, slot: usize, item: &str) -> bool {
    let bare = item.strip_prefix("minecraft:").unwrap_or(item);
    match slot {
        0 => item == SADDLE,
        1 if LLAMAS.contains(&kind) => bare.strip_suffix("_carpet").is_some_and(|colour| CARPETS.contains(&colour)),
        1 => kind == HORSE && bare.strip_suffix("_horse_armor").is_some_and(|metal| HORSE_ARMOR.contains(&metal)),
        _ => true,
    }
}

/// What a mount's inventory refuses in a slot: in its tack slots what no mount wears there.
// TODO: by kind, so a llama's slot 1 refuses horse armour (its screen offers no such slot).
fn refuses(slot: usize, stack: &ItemStack) -> bool {
    slot < TACK && !takes(HORSE, slot, &stack.name) && !takes(LLAMAS[0], slot, &stack.name)
}

/// What the inventory of `mob` refuses, if it is a mount's that was opened.
pub(crate) fn refusal(world: &World, mob: Entity) -> Option<fn(usize, &ItemStack) -> bool> {
    world.get::<Pack>(mob).map(|_| refuses as fn(usize, &ItemStack) -> bool)
}

/// The slots of `mob`'s inventory, once it was opened.
pub(crate) fn slots(world: &World, mob: Entity) -> Option<Vec<Option<Held>>> {
    world.get::<Pack>(mob).map(|pack| pack.0.clone())
}

/// Puts `armor` on `mob`, or takes it off.
fn wear(world: &mut World, mob: Entity, armor: Option<ItemStack>) {
    match world.get_mut::<Tack>(mob) {
        Some(mut tack) => tack.armor = armor,
        None => {
            let mut tack = Tack::default();
            tack.armor = armor;
            world.entity_mut(mob).insert(tack);
        }
    }
}

/// What `mob`'s goods are kept as: its pack's slots once opened, else its tack and the chest's
/// goods its stored compound holds.
fn goods(world: &World, mob: Entity) -> Vec<(u8, ItemStack)> {
    if let Some(Pack(slots)) = world.get::<Pack>(mob) {
        return slots.iter().enumerate().filter_map(|(slot, held)| Some((slot as u8, held.as_ref()?.stack.clone()))).collect();
    }
    let saddle = world.get::<Raised>(mob).filter(|raised| raised.saddled).map(|_| (0, stack(SADDLE)));
    let armor = world.get::<Tack>(mob).and_then(|tack| tack.armor.clone()).map(|armor| (1, armor));
    let stored = world.get::<Stored>(mob).map(|stored| sapling_world::actor_chest(&stored.compound)).unwrap_or_default();
    saddle.into_iter().chain(armor).chain(stored.into_iter().filter(|(slot, _)| usize::from(*slot) >= TACK)).collect()
}

/// What a dying mount lets go of, `doMobLoot` or not: its goods and the chest they were in; its
/// saddle, armour and carpet stay with it (research/captures/bds-tack.txt).
// TODO: the chest's own drop is Java's; BDS's chested donkey was not seen.
pub(crate) fn spilled(world: &World, mob: Entity) -> Vec<ItemStack> {
    if !world.get::<Kind>(mob).is_some_and(|kind| has_pack(&kind.0)) {
        return Vec::new();
    }
    let mut spilled: Vec<ItemStack> = goods(world, mob).into_iter().filter(|(slot, _)| usize::from(*slot) >= TACK).map(|(_, stack)| stack).collect();
    spilled.extend(chested(world, mob).then(|| stack(CHEST)));
    spilled
}

/// Writes `mob`'s tack and goods into what a world folder keeps of it.
pub(super) fn keep(world: &mut World, mob: Entity) {
    let items = goods(world, mob);
    let (saddled, chested) = world.get::<Raised>(mob).map_or((false, false), |raised| (raised.saddled, raised.chested));
    entities::restore(world, mob, |compound, registry| {
        let flagged = sapling_world::flagged_actor(compound, "Saddled", saddled).ok()?;
        let flagged = sapling_world::flagged_actor(&flagged, "Chested", chested).ok()?;
        sapling_world::chested_actor(&flagged, &items, registry).ok()
    });
}

/// A mob a world folder kept: the armour or carpet in its slot 1 is worn again.
pub(super) fn adopt(world: &mut World, mob: Entity) {
    let Some(kind) = world.get::<Kind>(mob).map(|kind| kind.0.clone()).filter(|kind| has_pack(kind)) else { return };
    let stored = world.get::<Stored>(mob).map(|stored| sapling_world::actor_chest(&stored.compound)).unwrap_or_default();
    if let Some((_, armor)) = stored.into_iter().find(|(slot, stack)| *slot == 1 && takes(&kind, 1, &stack.name)) {
        wear(world, mob, Some(armor));
    }
}

/// The slots of `mob`'s inventory, made from what it wears and keeps when first asked for.
fn pack(world: &mut World, mob: Entity) -> Option<Vec<Option<Held>>> {
    let kind = world.get::<Kind>(mob)?.0.clone();
    let size = size(world, mob, &kind);
    if let Some(Pack(slots)) = world.get::<Pack>(mob).filter(|pack| pack.0.len() == size) {
        return Some(slots.clone());
    }
    let mut slots = world.get::<Pack>(mob).map(|pack| pack.0.clone()).unwrap_or_else(|| {
        let goods = goods(world, mob);
        let mut slots = vec![None; size];
        for (slot, stack) in goods.into_iter().filter(|(slot, _)| usize::from(*slot) < size) {
            slots[usize::from(slot)] = Some(containers::new_held(world, stack));
        }
        slots
    });
    slots.resize(size, None);
    world.entity_mut(mob).insert(Pack(slots.clone()));
    Some(slots)
}

/// Makes `slots` what `mob`'s inventory holds: those with it open see each changed slot, the tack
/// in slots 0 and 1 is what it wears, and its stored compound keeps them.
pub(crate) fn load(world: &mut World, mob: Entity, slots: Vec<Option<Held>>) {
    let Some(Pack(before)) = world.get::<Pack>(mob).cloned() else { return };
    let changed: Vec<usize> = (0..slots.len().min(before.len())).filter(|slot| before[*slot] != slots[*slot]).collect();
    if changed.is_empty() {
        return;
    }
    world.entity_mut(mob).insert(Pack(slots.clone()));
    let viewers = containers::carried_viewers(world, mob);
    containers::tell_changed(world, viewers, &changed, &slots);
    if changed.iter().any(|slot| *slot < TACK) {
        let saddled = slots[0].is_some();
        let armor = slots.get(1).cloned().flatten().map(|held| held.stack).filter(|stack| world.get::<Kind>(mob).is_some_and(|kind| takes(&kind.0, 1, &stack.name)));
        if let Some(mut raised) = world.get_mut::<Raised>(mob) {
            raised.saddled = saddled;
        }
        wear(world, mob, armor);
        let worn = super::mounts::worn(world, mob);
        super::tell(world, mob, mob, worn);
        super::retell(world, mob);
    }
    keep(world, mob);
}

/// The tack `mob` wears changed by hand: its inventory holds it, and those with it open see it.
pub(super) fn tack_changed(world: &mut World, mob: Entity) {
    if let Some(Pack(mut slots)) = world.get::<Pack>(mob).cloned() {
        let saddled = world.get::<Raised>(mob).is_some_and(|raised| raised.saddled);
        let armor = world.get::<Tack>(mob).and_then(|tack| tack.armor.clone());
        if saddled != slots[0].is_some() {
            slots[0] = saddled.then(|| containers::new_held(world, stack(SADDLE)));
        }
        if armor.as_ref() != slots[1].as_ref().map(|held| &held.stack) {
            slots[1] = armor.map(|armor| containers::new_held(world, armor));
        }
        world.entity_mut(mob).insert(Pack(slots.clone()));
        let viewers = containers::carried_viewers(world, mob);
        containers::tell_changed(world, viewers, &[0, 1], &slots);
    }
    keep(world, mob);
}

/// The window's description of each tack slot: what it takes and the outline it shows empty.
fn equipment(kind: &str) -> Option<Raw<Network>> {
    let item = |name: &str| Value::Compound(vec![("Count".into(), Value::Byte(1)), ("Damage".into(), Value::Short(0)), ("Name".into(), Value::String(name.into())), ("WasPickedUp".into(), Value::Byte(0))]);
    let slot = |number: i32, outline: &str, takes: Vec<String>| {
        let accepted = takes.iter().map(|name| Value::Compound(vec![("slotItem".into(), item(name))])).collect();
        Value::Compound(vec![("acceptedItems".into(), Value::List(List { tag: 10, items: accepted })), ("item".into(), item(outline)), ("slotNumber".into(), Value::Int(number))])
    };
    let mut slots = Vec::new();
    if SADDLED.contains(&kind) {
        slots.push(slot(0, "minecraft:horsesaddle", vec![SADDLE.into()]));
    }
    if kind == HORSE {
        slots.push(slot(1, "minecraft:horsearmor", HORSE_ARMOR.iter().map(|metal| format!("minecraft:{metal}_horse_armor")).collect()));
    } else if LLAMAS.contains(&kind) {
        slots.push(slot(1, "minecraft:carpet", CARPETS.iter().map(|colour| format!("minecraft:{colour}_carpet")).collect()));
    }
    let mut out = BytesMut::new();
    nbt::write::<Network>(&mut out, &Nbt { name: String::new(), value: Value::Compound(vec![("slots".into(), Value::List(List { tag: 10, items: slots }))]) });
    Raw::<Network>::read(&mut &out[..]).ok()
}

/// Opens `mob`'s inventory for `player`; false for a mob that has none or is not tamed.
pub(crate) fn open(world: &mut World, player: Entity, mob: Entity) -> bool {
    let Some(kind) = world.get::<Kind>(mob).map(|kind| kind.0.clone()).filter(|kind| has_pack(kind) && tamed(world, mob)) else { return false };
    let (Some(slots), Some(id), Some(layer), Some(body)) = (pack(world, mob), world.get::<NetId>(mob).map(|id| id.0), world.get::<InLayer>(mob).map(|layer| layer.0), world.get::<Body>(mob).copied()) else { return false };
    if equipment(&kind).is_none() {
        return false;
    }
    // BDS tells a size of 0 whatever the slots (research/captures/bds-tack.txt).
    let told = |window: u8| UpdateEquipment { window_id: WindowID::from_raw(window.into()), window_type: WindowType::Horse, size: 0, entity_id: id as i64, inventory: equipment(&kind).expect("made above") };
    containers::show_carried(world, player, layer, body.at.map(|c| c.floor() as i32), (mob, id), WindowType::Horse, &slots, Some(&told))
}

/// A player's click on `mob` with what it holds: a tamed grown donkey, mule or llama takes the
/// chest held, a tamed mount opens to a sneaking player. False when the click does neither.
pub(super) fn clicked(world: &mut World, player: Entity, mob: Entity, kind: &str, sneaking: bool) -> bool {
    let held = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref().map(|held| held.stack.name.clone()));
    let tame_grown = tamed(world, mob) && !creatures::is_baby(world, mob);
    // A sneaking click opens even holding a chest (research/captures/bds-tack.txt).
    if !sneaking && held.as_deref() == Some(CHEST) && tame_grown && CHESTED.contains(&kind) && !chested(world, mob) {
        if let Some(mut raised) = world.get_mut::<Raised>(mob) {
            raised.chested = true;
        }
        blocks::spend_held(world, player);
        let (Some(id), Some(body)) = (world.get::<NetId>(mob).map(|id| id.0), world.get::<Body>(mob).copied()) else { return true };
        let shown = entities::mob_flags(world, mob, kind, acacia_proto::types::MetadataFlags1::CHESTED, true);
        super::tell(world, mob, mob, vec![super::packets::flags(id, shown), super::packets::sound("add.chest", id, kind, body.at)]);
        keep(world, mob);
        return true;
    }
    sneaking && has_pack(kind) && open(world, player, mob)
}
