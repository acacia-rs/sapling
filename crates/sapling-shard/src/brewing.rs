//! Brewing stands at work, by BDS's `BrewingStandBlockActor::tick` (141f22400): each tick a
//! watched one (ticking.rs) whose fuel is out spends what lies in its fuel slot (a blaze powder
//! gives twenty brews), starts a brew of 400 ticks when it has fuel and its ingredient makes
//! something of a bottle, and at the end turns each bottle by BDS's mixes
//! (`sapling_data::brewed`), spends the ingredient and one fuel. Its slots are a container's
//! (containers.rs): 0 the ingredient, 1 to 3 the bottles, 4 the fuel. Evidence:
//! docs/research/brewing.md.

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{ContainerSetData, LevelSoundEvent};
use acacia_proto::types::{Vec3f, WindowID};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_world::ItemStack;

use crate::components::{Layer, Outgoing};
use crate::{containers, late_sounds};
use crate::inventory::Held;

const INGREDIENT: usize = 0;
const BOTTLES: [usize; 3] = [1, 2, 3];
const FUEL: usize = 4;
const POWDER: &str = "minecraft:blaze_powder";
/// Brews one blaze powder lasts, and the ticks a brew takes.
const BREWS_PER_POWDER: u16 = 20;
const BREW_TICKS: u16 = 400;
/// The `ContainerSetData` properties of a stand's screen: brew ticks left, fuel left, fuel given.
const PROPERTIES: [i32; 3] = [0, 1, 2];
/// The fields of the block entity that hold the same three.
const FIELDS: [&str; 3] = ["CookTime", "FuelAmount", "FuelTotal"];
const BREWED: &str = "potion.brewed";
const JUST_BREWED: &str = "wasJustBrewed";

/// A watched stand's brew.
#[derive(Default)]
struct Brew {
    ticks: u16,
    fuel: u16,
    fuel_total: u16,
    /// The ingredient the brew began with: taking it out or swapping it for another item (its
    /// data unread) stops the brew.
    with: Option<String>,
    shown: Option<[u16; 3]>,
}

impl Brew {
    fn values(&self) -> [u16; 3] {
        [self.ticks, self.fuel, self.fuel_total]
    }
}

/// The brewing stands that tick, by layer and block.
#[derive(Resource, Default)]
pub(crate) struct Stands(HashMap<(Entity, BlockPos), Brew>);

fn entity(world: &World, layer: Entity, pos: BlockPos) -> Option<Value> {
    world.get::<Layer>(layer)?.world.block_entity(pos).map(|entity| entity.nbt.clone())
}

fn field(entity: &Value, name: &str) -> u16 {
    if let Some(Value::Short(value)) = entity.get(name) { (*value).max(0) as u16 } else { 0 }
}

/// Whether a stand's block entity says it was brewing, or holds anything that may start a brew
/// (a hopper's), when it was kept.
pub(crate) fn was_at_work(entity: &Value) -> bool {
    field(entity, FIELDS[0]) > 0 || matches!(entity.get("Items"), Some(Value::List(list)) if !list.items.is_empty())
}

/// Starts ticking the stand at `pos` from what its block entity kept, unless it ticks already.
pub(crate) fn watch(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(kept) = entity(world, layer, pos) else { return };
    if !containers::pin(world, layer, pos) {
        return;
    }
    let brew = Brew { ticks: field(&kept, FIELDS[0]), fuel: field(&kept, FIELDS[1]), fuel_total: field(&kept, FIELDS[2]), ..Brew::default() };
    world.get_resource_or_insert_with(Stands::default).0.entry((layer, pos)).or_insert(brew);
}

/// A screen of the stand at `pos` opened: it ticks, and its screens are told all three values.
pub(crate) fn opened(world: &mut World, layer: Entity, pos: BlockPos) {
    watch(world, layer, pos);
    if let Some(mut stands) = world.get_resource_mut::<Stands>()
        && let Some(brew) = stands.0.get_mut(&(layer, pos))
    {
        brew.shown = None;
    }
}

/// The stands that tick.
pub(crate) fn watched(world: &World) -> Vec<(Entity, BlockPos)> {
    world.get_resource::<Stands>().map(|stands| stands.0.keys().copied().collect()).unwrap_or_default()
}

/// Stops ticking the stand at `pos`, what it was doing kept in its block entity.
pub(crate) fn forget(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(brew) = world.get_resource_mut::<Stands>().and_then(|mut stands| stands.0.remove(&(layer, pos))) else { return };
    keep(world, layer, pos, &brew);
    containers::unpin(world, layer, pos);
}

/// What a stand takes into `slot`, by BDS's `canPushInItem` (141f219d0): a blaze powder as fuel,
/// a bottle (a glass one too) below, anything a mix takes on top.
fn takes(slot: usize, item: &ItemStack) -> bool {
    match slot {
        INGREDIENT => sapling_data::is_brewing_ingredient(&item.name, item.damage),
        FUEL => item.name == POWDER,
        _ => item.name == "minecraft:glass_bottle" || sapling_data::is_brewing_bottle(&item.name),
    }
}

/// What a player may not put into a stand's `slot`.
pub(crate) fn refuses(slot: usize, item: &ItemStack) -> bool {
    !takes(slot, item)
}

/// A hopper above fills the ingredient; one beside the fuel and the bottles.
// TODO: BDS fills only an empty bottle slot; here glass bottles stack onto one.
pub(crate) fn takes_from_hopper(slot: usize, face: Face, item: &ItemStack) -> bool {
    takes(slot, item) && (slot == INGREDIENT) == (face == Face::Up)
}

/// A hopper below empties the bottles, brewed or not (`canPullOutItem` 141f21ce0).
pub(crate) fn gives_to_hopper(slot: usize, face: Face) -> bool {
    face == Face::Down && BOTTLES.contains(&slot)
}

fn keep(world: &mut World, layer: Entity, pos: BlockPos, brew: &Brew) {
    let Some(Value::Compound(mut fields)) = entity(world, layer, pos) else { return };
    fields.retain(|(key, _)| !FIELDS.contains(&key.as_str()));
    fields.extend(FIELDS.iter().zip(brew.values()).map(|(name, value)| ((*name).into(), Value::Short(value as i16))));
    if let Some(mut level) = world.get_mut::<Layer>(layer) {
        level.world.set_block_entity(pos, &Value::Compound(fields));
    }
}

/// What each bottle of `slots` becomes with its ingredient; `None` where it stays.
fn made(slots: &[Option<Held>]) -> [Option<(&'static str, i16)>; 3] {
    let ingredient = slots[INGREDIENT].as_ref().map(|held| (held.stack.name.as_str(), held.stack.damage));
    BOTTLES.map(|slot| {
        let (name, data) = ingredient?;
        let bottle = slots[slot].as_ref()?;
        sapling_data::brewed(&bottle.stack.name, bottle.stack.damage, name, data)
    })
}

fn take_one(slot: &mut Option<Held>) {
    if let Some(held) = slot.as_mut().filter(|held| held.stack.count > 1) {
        held.stack.count -= 1;
    } else {
        *slot = None;
    }
}

/// The brew is done: every bottle that makes something is turned (tagged `wasJustBrewed`), the
/// ingredient spent, and those near hear it a tick later.
fn finish(world: &mut World, layer: Entity, pos: BlockPos, slots: &mut [Option<Held>]) {
    for (slot, made) in BOTTLES.into_iter().zip(made(slots)) {
        if let (Some((name, data)), Some(held)) = (made, slots[slot].as_mut()) {
            let tag = Value::Compound(vec![(JUST_BREWED.into(), Value::Byte(1))]);
            held.stack = ItemStack { name: name.to_owned(), count: 1, damage: data, block: None, tag: Some(tag) };
        }
    }
    take_one(&mut slots[INGREDIENT]);
    let [x, y, z] = pos.map(|c| c as f32 + 0.5);
    let sound = LevelSoundEvent { sound_id: BREWED.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: String::new(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
    late_sounds::play_later(world, 1, layer, pos, sapling_net::encode(&sound));
}

/// One tick of the stand at `pos`; false once it is gone or has nothing left to do and nobody
/// looking.
fn step(world: &mut World, layer: Entity, pos: BlockPos, brew: &mut Brew) -> bool {
    if containers::block_name(world, layer, pos) != Some("minecraft:brewing_stand") {
        return false;
    }
    let Some(mut slots) = containers::slots_at(world, layer, pos).filter(|slots| slots.len() > FUEL) else { return false };
    let before = slots.clone();
    if brew.fuel == 0 && let Some(fuel) = slots[FUEL].as_ref() {
        let gives = if fuel.stack.name == POWDER { BREWS_PER_POWDER } else { 0 };
        (brew.fuel, brew.fuel_total) = (gives, gives);
        take_one(&mut slots[FUEL]);
    }
    let brews = brew.fuel > 0 && made(&slots).iter().any(Option::is_some);
    let ingredient = slots[INGREDIENT].as_ref().map(|held| held.stack.name.clone());
    if brew.ticks == 0 {
        if brews {
            (brew.ticks, brew.with) = (BREW_TICKS, ingredient);
        }
    } else {
        brew.ticks -= 1;
        if brews && brew.ticks == 0 {
            finish(world, layer, pos, &mut slots);
            brew.fuel -= 1;
            // BDS shows the bottles before the spent ingredient.
            let mut bottled = slots.clone();
            bottled[INGREDIENT].clone_from(&before[INGREDIENT]);
            containers::set_slots(world, layer, pos, bottled);
        } else if !brews || ingredient != brew.with {
            brew.ticks = 0;
        }
    }
    if slots != before {
        containers::set_slots(world, layer, pos, slots);
    }
    let viewers = containers::viewers(world, layer, pos);
    show(world, &viewers, brew);
    let is_idle = brew.ticks == 0;
    if is_idle && viewers.is_empty() {
        keep(world, layer, pos, brew);
        containers::unpin(world, layer, pos);
    }
    !(is_idle && viewers.is_empty())
}

/// Tells the screens of a stand the values that changed since they were last told.
fn show(world: &mut World, viewers: &[(Entity, u8)], brew: &mut Brew) {
    let (now, told) = (brew.values(), brew.shown);
    if viewers.is_empty() {
        brew.shown = None;
        return;
    }
    brew.shown = Some(now);
    for (viewer, window) in viewers {
        let Some(mut out) = world.get_mut::<Outgoing>(*viewer) else { continue };
        for index in (0..PROPERTIES.len()).filter(|index| told.is_none_or(|told| told[*index] != now[*index])) {
            out.push(&ContainerSetData { window_id: WindowID::from_raw((*window).into()), property: PROPERTIES[index], value: now[index].into() });
        }
    }
}

/// Lets every watched stand brew for a tick.
pub(crate) fn tick(world: &mut World) {
    let Some(mut stands) = world.get_resource_mut::<Stands>().filter(|stands| !stands.0.is_empty()) else { return };
    let mut brews = std::mem::take(&mut stands.0);
    brews.retain(|(layer, pos), brew| step(world, *layer, *pos, brew));
    world.resource_mut::<Stands>().0.extend(brews);
}
