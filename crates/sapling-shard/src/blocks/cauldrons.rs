//! A cauldron's potions, dyed water and washing, as BDS ran them (research/captures/bds-cauldrons.txt,
//! docs/research/block-uses.md "Cauldrons"): the potion and the colour are in the cauldron's entity
//! (`PotionId`, `PotionType`, `CustomColor`) over `cauldron_liquid=water`. Water by bucket and bottle
//! is tools.rs's, powder snow snow.rs's.
//! A bucket takes a full cauldron of dyed water with its colour and does nothing to a potion; a
//! banner loses its last pattern a level at a time (bds-cauldrons2.txt). A potion poured into
//! water, plain or dyed, or onto another potion bursts it, heard `bottle.empty` but for a water
//! bottle (bds-cauldrons3.txt).

mod snow;

pub(super) use snow::fallen_in;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::{BlockRegistry, ItemStack, new_block_entity};

use super::send::keep_entity;
use super::{fields, level, send, with_property};
use crate::inventory::{self, Inventory, Place};

const CAULDRON: &str = "minecraft:cauldron";
const ENTITY: &str = "Cauldron";
const FULL: u8 = 6;
const BOTTLEFUL: u8 = 2;
/// Arrows a level of a potion tips.
const TIPPED: u8 = 16;
/// The event colour of plain water.
const WATER: i32 = -1_087_090_343;
const BOTTLES: [&str; 3] = ["minecraft:potion", "minecraft:splash_potion", "minecraft:lingering_potion"];
const SHULKER_BOX: &str = "minecraft:undyed_shulker_box";
const DYED_LEATHER: &str = "customColor";

/// What the cauldron holds: its level, a potion's id and kind, a dye's colour, and whether it is
/// powder snow.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Held {
    level: u8,
    potion: Option<(i16, i16)>,
    colour: Option<i32>,
    snow: bool,
}

/// What a click does: the cauldron after, the event told at a level's surface with its data, the
/// sound after, and what the hand becomes.
struct Outcome {
    next: Held,
    told: Option<(LevelEventEvent, u8, i32)>,
    sound: Option<&'static str>,
    cost: Cost,
}

/// What the hand pays, outside creative.
enum Cost {
    Spend,
    Becomes(ItemStack),
    /// This many of the stack in hand turn into the stack.
    Changed(ItemStack),
}

fn short(value: Option<Value>) -> Option<i16> {
    match value? {
        Value::Short(value) => Some(value),
        _ => None,
    }
}

fn held_at(world: &World, layer: Entity, pos: BlockPos) -> Option<Held> {
    let level = level(world, layer)?;
    let state = level.registry().get(level.block(pos[0], pos[1], pos[2])).filter(|state| state.name == CAULDRON)?;
    let snow = match state.property("cauldron_liquid")? {
        "water" => false,
        "powder_snow" => true,
        _ => return None,
    };
    let filled: u8 = state.property("fill_level")?.parse().ok()?;
    let id = short(fields::get(world, layer, pos, "PotionId")).filter(|id| *id >= 0);
    let kind = short(fields::get(world, layer, pos, "PotionType")).unwrap_or(0).max(0);
    let colour = match fields::get(world, layer, pos, "CustomColor") {
        Some(Value::Int(colour)) => Some(colour),
        _ => None,
    };
    Some(Held { level: filled, potion: id.map(|id| (id, kind)), colour, snow })
}

/// The cauldron state `id` holding `level` of water, or of powder snow.
fn with_contents(registry: &BlockRegistry, id: u32, level: u8, snow: bool) -> Option<u32> {
    let filled = with_property(registry, id, "fill_level", &level.to_string())?;
    with_property(registry, filled, "cauldron_liquid", if snow { "powder_snow" } else { "water" })
}

/// The cauldron's surface at `level`, where its events are told.
fn surface(pos: BlockPos, level: u8) -> Vec3f {
    Vec3f { x: pos[0] as f32 + 0.5, y: pos[1] as f32 + 0.375 + f32::from(level) / 8.0, z: pos[2] as f32 + 0.5 }
}

/// Tells the event, writes the level into the block and the rest into its entity, then tells the
/// sound at the block's middle, in BDS's order. A burst leaves `PotionType` 0, where a cauldron
/// that never held one has -1.
fn write(world: &mut World, layer: Entity, pos: BlockPos, outcome: &Outcome) {
    if let Some((event, at, data)) = outcome.told {
        send::to_viewers(world, layer, pos, &[sapling_net::encode(&LevelEvent { event, position: surface(pos, at), data })]);
    }
    let next = outcome.next;
    let Some(level) = level(world, layer) else { return };
    let filled = with_contents(level.registry(), level.block(pos[0], pos[1], pos[2]), next.level, next.snow);
    let entity = level.block_entity(pos).map_or_else(|| new_block_entity(ENTITY, pos), |entity| entity.nbt.clone());
    if let Some(id) = filled {
        send::set(world, layer, pos, id);
    }
    let Value::Compound(mut kept) = entity else { return };
    let old = |key: &str| kept.iter().find(|(name, _)| name == key).and_then(|(_, value)| short(Some(value.clone())));
    let had_potion = old("PotionId").is_some_and(|id| id >= 0);
    let (id, kind) = next.potion.unwrap_or((-1, if had_potion { 0 } else { old("PotionType").unwrap_or(-1) }));
    kept.retain(|(name, _)| !matches!(name.as_str(), "PotionId" | "PotionType" | "CustomColor"));
    kept.extend([("PotionId".into(), Value::Short(id)), ("PotionType".into(), Value::Short(kind))]);
    kept.extend(next.colour.map(|colour| ("CustomColor".into(), Value::Int(colour))));
    kept.sort_by(|a, b| a.0.cmp(&b.0));
    keep_entity(world, layer, pos, &Value::Compound(kept));
    if let Some(sound) = outcome.sound {
        let [x, y, z] = pos.map(|c| c as f32 + 0.5);
        send::to_viewers(world, layer, pos, &[send::noise(sound, Vec3f { x, y, z }, -1)]);
    }
}

fn stack(name: &str, damage: i16) -> ItemStack {
    ItemStack { name: name.to_owned(), count: 1, damage, block: None, tag: None }
}

fn with_tag(hand: &ItemStack, key: &str, value: Option<Value>) -> ItemStack {
    let mut fields = match &hand.tag {
        Some(Value::Compound(fields)) => fields.clone(),
        _ => Vec::new(),
    };
    fields.retain(|(name, _)| name != key);
    fields.extend(value.map(|value| (key.into(), value)));
    ItemStack { count: 1, tag: (!fields.is_empty()).then_some(Value::Compound(fields)), ..hand.clone() }
}

/// Two colours mixed half and half.
fn blend(old: i32, rgb: u32) -> i32 {
    let mix = |shift: u32| (((old as u32 >> shift) & 0xFF) + ((rgb >> shift) & 0xFF)) / 2;
    (0xFF00_0000 | mix(16) << 16 | mix(8) << 8 | mix(0)) as i32
}

/// A patterned banner with its last pattern washed off; `None` for a plain or an ominous one.
fn washed_banner(hand: &ItemStack) -> Option<ItemStack> {
    let tag = hand.tag.as_ref()?;
    (!matches!(tag.get("Type"), Some(Value::Int(1)))).then_some(())?;
    let Some(Value::List(patterns)) = tag.get("Patterns") else { return None };
    let (_, rest) = patterns.items.split_last()?;
    let left = (!rest.is_empty()).then(|| Value::List(acacia_proto::nbt::List { tag: patterns.tag, items: rest.to_vec() }));
    Some(with_tag(hand, "Patterns", left))
}

/// The cauldron emptied with `CauldronExplode` at its floor, coloured as what it held.
fn burst(held: Held, sound: Option<&'static str>, cost: Cost) -> Outcome {
    let next = Held { level: 0, potion: None, colour: None, snow: false };
    Outcome { next, told: Some((LevelEventEvent::CauldronExplode, 0, held.colour.unwrap_or(WATER))), sound, cost }
}

/// A potion poured in: like on like (or into an empty cauldron) two levels more and the last
/// kind; into anything else it bursts.
fn poured(hand: &ItemStack, kind: i16, held: Held) -> Option<Outcome> {
    let fits = held.level == 0 || held.colour.is_none() && held.potion.is_some_and(|(id, _)| id == hand.damage);
    let cost = Cost::Becomes(stack("minecraft:glass_bottle", 0));
    if !fits {
        let is_water = hand.name == BOTTLES[0] && hand.damage == 0;
        return Some(burst(held, (!is_water).then_some("bottle.empty"), cost));
    }
    (held.level < FULL).then_some(())?;
    let level = (held.level + BOTTLEFUL).min(FULL);
    let next = Held { level, potion: Some((hand.damage, kind)), colour: None, snow: false };
    Some(Outcome { next, told: Some((LevelEventEvent::CauldronFillPotion, level, sapling_data::shots::potion_colour(hand.damage))), sound: Some("bottle.empty"), cost })
}

/// Leather dyed in coloured water, or washed in plain water, for a level.
fn leather(hand: &ItemStack, held: Held) -> Option<Outcome> {
    let next = Held { level: held.level.checked_sub(1)?, ..held };
    let (event, data, colour) = match held.colour {
        Some(colour) => (LevelEventEvent::CauldronDyeArmor, colour, Some(Value::Int(colour))),
        None => {
            hand.tag.as_ref()?.get(DYED_LEATHER)?;
            (LevelEventEvent::CauldronCleanArmor, WATER, None)
        }
    };
    Some(Outcome { next, told: Some((event, next.level, data)), sound: None, cost: Cost::Changed(with_tag(hand, DYED_LEATHER, colour)) })
}

/// What a click with `hand` does to the cauldron holding `held`.
fn change(hand: &ItemStack, held: Held) -> Option<Outcome> {
    let name = hand.name.as_str();
    if held.snow || name == snow::BUCKET {
        return snow::change(hand, held);
    }
    let is_water = hand.damage == 0 && name == BOTTLES[0];
    if let Some(kind) = BOTTLES.iter().position(|bottle| *bottle == name).filter(|_| !is_water || held.potion.is_some()) {
        return poured(hand, kind as i16, held);
    }
    let plain = held.potion.is_none() && held.colour.is_none();
    match name {
        "minecraft:glass_bottle" => {
            let (id, kind) = held.potion.filter(|_| held.level >= BOTTLEFUL)?;
            let level = held.level - BOTTLEFUL;
            let next = Held { level, potion: (level > 0).then_some((id, kind)), ..held };
            let filled = stack(BOTTLES[usize::try_from(kind).ok()?.min(2)], id);
            Some(Outcome { next, told: Some((LevelEventEvent::CauldronTakePotion, held.level, sapling_data::shots::potion_colour(id))), sound: Some("bottle.fill"), cost: Cost::Becomes(filled) })
        }
        // Tipping tells nothing but the block.
        "minecraft:arrow" => {
            let (id, _) = held.potion.filter(|_| held.level > 0)?;
            let next = Held { level: held.level - 1, potion: (held.level > 1).then_some((id, held.potion?.1)), ..held };
            let tipped = ItemStack { count: hand.count.min(TIPPED), ..stack("minecraft:arrow", id + 1) };
            Some(Outcome { next, told: None, sound: None, cost: Cost::Changed(tipped) })
        }
        _ if name.starts_with("minecraft:leather_") && held.level > 0 && held.potion.is_none() => leather(hand, held),
        "minecraft:bucket" if held.level == FULL => {
            let colour = held.colour?;
            let next = Held { level: 0, potion: None, colour: None, snow: false };
            Some(Outcome { next, told: Some((LevelEventEvent::CauldronTakeWater, FULL, colour)), sound: None, cost: Cost::Becomes(stack("minecraft:water_bucket", 0)) })
        }
        "minecraft:banner" if held.level > 0 && plain => {
            let washed = washed_banner(hand)?;
            let next = Held { level: held.level - 1, ..held };
            Some(Outcome { next, told: Some((LevelEventEvent::CauldronCleanBanner, next.level, WATER)), sound: None, cost: Cost::Changed(washed) })
        }
        _ if name.ends_with("_shulker_box") && name != SHULKER_BOX && held.level > 0 && plain => {
            let next = Held { level: held.level - 1, ..held };
            let washed = ItemStack { name: SHULKER_BOX.to_owned(), block: None, ..hand.clone() };
            Some(Outcome { next, told: Some((LevelEventEvent::CauldronCleanArmor, next.level, WATER)), sound: None, cost: Cost::Changed(ItemStack { count: 1, ..washed }) })
        }
        _ => {
            // The older dye items (bone meal, ink sac) were not drilled; bone meal is bonemeal.rs's.
            let rgb = crate::fireworks::dye_rgb(name).filter(|_| name.ends_with("_dye") && held.level > 0 && held.potion.is_none())?;
            let colour = held.colour.map_or((0xFF00_0000 | rgb) as i32, |old| blend(old, rgb));
            Some(Outcome { next: Held { colour: Some(colour), ..held }, told: Some((LevelEventEvent::CauldronAddDye, held.level, colour)), sound: None, cost: Cost::Spend })
        }
    }
}

/// A click with what the player holds on the cauldron at `pos`: false when it does nothing here.
pub(super) fn use_on(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, creative: bool) -> bool {
    let Some(held) = held_at(world, layer, pos) else { return false };
    let Some(hand) = world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).map(|held| held.stack.clone()) else { return false };
    // BDS answered a bucket on a potion with nothing at all (bds-cauldrons2.txt).
    if hand.name == "minecraft:bucket" && held.potion.is_some() {
        return true;
    }
    let Some(outcome) = change(&hand, held) else { return false };
    write(world, layer, pos, &outcome);
    if creative {
        return true;
    }
    match outcome.cost {
        Cost::Spend => drop(inventory::use_up_held(world, player)),
        Cost::Becomes(into) => inventory::swap_held_for(world, player, into),
        Cost::Changed(into) => {
            inventory::change(world, player, |inventory| {
                let slot = Place::Main(inventory.selected.into());
                if inventory.take(slot, into.count.into()).is_ok() {
                    if inventory.slot(slot).is_some_and(Option::is_none) {
                        inventory.put(slot, Some(into.clone()));
                    } else {
                        inventory.add(&into);
                    }
                }
            });
        }
    }
    true
}

#[cfg(test)]
mod tests;
