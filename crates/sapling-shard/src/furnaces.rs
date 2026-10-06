//! Furnaces, blast furnaces and smokers at work: each tick a watched one (ticking.rs) burns its
//! fuel down, lights the next piece when it has something to smelt, smelts, and crackles while
//! lit. The order within a tick is BDS's `FurnaceBlockActor::tick` (docs/research/furnaces.md;
//! containers-crafting.md, section D). Its slots are a container's (containers.rs): 0 what is
//! smelted, 1 the fuel, 2 the result. Taking results pays experience (xp.rs).

mod fuel;
mod xp;

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{ContainerSetData, LevelSoundEvent};
use acacia_proto::types::{Vec3f, WindowID};
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};
use sapling_data::Station;
use sapling_world::ItemStack;

use crate::blocks::{self, Dice};
use crate::components::{Layer, Outgoing};
use crate::containers;
use crate::inventory::Held;

const INPUT: usize = 0;
const FUEL: usize = 1;
const OUTPUT: usize = 2;
/// The `ContainerSetData` properties of a furnace's screen: ticks cooked, ticks the fire has
/// left, ticks the burning piece of fuel gave.
const PROPERTIES: [i32; 3] = [0, 1, 2];
/// The fields of the block entity that hold the same three.
const FIELDS: [&str; 3] = ["CookTime", "BurnTime", "BurnDuration"];
/// A lit furnace crackles every this many ticks and up to [`CRACKLE_SPREAD`] more.
const CRACKLE_EVERY: u32 = 20;
const CRACKLE_SPREAD: u32 = 80;

/// A kind of smelter: its recipes, the ticks an item takes, its block unlit and lit, and its sound.
struct Smelter {
    station: Station,
    interval: u16,
    blocks: [&'static str; 2],
    crackle: &'static str,
}

fn smelter(block: &str) -> Option<Smelter> {
    let kinds = [
        Smelter { station: Station::Furnace, interval: 200, blocks: ["minecraft:furnace", "minecraft:lit_furnace"], crackle: "block.furnace.lit" },
        Smelter { station: Station::BlastFurnace, interval: 100, blocks: ["minecraft:blast_furnace", "minecraft:lit_blast_furnace"], crackle: "block.blastfurnace.fire_crackle" },
        Smelter { station: Station::Smoker, interval: 100, blocks: ["minecraft:smoker", "minecraft:lit_smoker"], crackle: "block.smoker.smoke" },
    ];
    kinds.into_iter().find(|kind| kind.blocks.contains(&block))
}

/// A watched furnace's fire.
#[derive(Default)]
struct Fire {
    cook: u16,
    lit: u16,
    lit_duration: u16,
    /// What the screens were last told of the three.
    shown: Option<[u16; 3]>,
    /// Ticks since it last crackled, and until it does again; neither is kept.
    since_crackle: u32,
    next_crackle: u32,
}

impl Fire {
    fn times(&self) -> [u16; 3] {
        [self.cook, self.lit, self.lit_duration]
    }
}

/// The furnaces that tick, by layer and block.
#[derive(Resource, Default)]
pub(crate) struct Furnaces(HashMap<(Entity, BlockPos), Fire>);

fn entity(world: &World, layer: Entity, pos: BlockPos) -> Option<Value> {
    world.get::<Layer>(layer)?.world.block_entity(pos).map(|entity| entity.nbt.clone())
}

fn keep(world: &mut World, layer: Entity, pos: BlockPos, entity: &Value) {
    if let Some(mut level) = world.get_mut::<Layer>(layer) {
        level.world.set_block_entity(pos, entity);
    }
}

fn time(entity: &Value, field: &str) -> u16 {
    if let Some(Value::Short(ticks)) = entity.get(field) { (*ticks).max(0) as u16 } else { 0 }
}

/// Whether a furnace's block entity says it was burning or cooking when it was kept.
pub(crate) fn was_at_work(entity: &Value) -> bool {
    FIELDS.iter().take(2).any(|field| time(entity, field) > 0)
}

/// Starts ticking the furnace at `pos` from the times its block entity kept, holding its slots;
/// one that ticks already tells its screens all its times again.
pub(crate) fn watch(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(kept) = entity(world, layer, pos) else { return };
    if !containers::pin(world, layer, pos) {
        return;
    }
    let fire = Fire { cook: time(&kept, FIELDS[0]), lit: time(&kept, FIELDS[1]), lit_duration: time(&kept, FIELDS[2]), ..Fire::default() };
    world.get_resource_or_insert_with(Furnaces::default).0.entry((layer, pos)).or_insert(fire).shown = None;
}

/// The furnaces that tick.
pub(crate) fn watched(world: &World) -> Vec<(Entity, BlockPos)> {
    world.get_resource::<Furnaces>().map(|furnaces| furnaces.0.keys().copied().collect()).unwrap_or_default()
}

/// Stops ticking the furnace at `pos`, its times kept in its block entity.
pub(crate) fn forget(world: &mut World, layer: Entity, pos: BlockPos) {
    let Some(fire) = world.get_resource_mut::<Furnaces>().and_then(|mut furnaces| furnaces.0.remove(&(layer, pos))) else { return };
    keep_times(world, layer, pos, &fire);
    containers::unpin(world, layer, pos);
}

/// What a player may not put into a furnace: anything into the result, what does not burn into
/// the fuel (BDS `isItemAllowedInFuelSlot`, which also takes a bucket).
pub(crate) fn refuses(slot: usize, item: &ItemStack) -> bool {
    match slot {
        FUEL => fuel::burn_ticks(&item.name, 1).is_none() && item.name != fuel::BUCKET,
        OUTPUT => true,
        _ => false,
    }
}

/// A hopper above fills the input with anything; one beside, the fuel with what burns.
pub(crate) fn takes_from_hopper(slot: usize, face: Face, item: &ItemStack) -> bool {
    match slot {
        INPUT => face == Face::Up,
        FUEL => face != Face::Up && fuel::burn_ticks(&item.name, 1).is_some(),
        _ => false,
    }
}

/// A hopper below empties the result, and the fuel slot of an empty bucket.
pub(crate) fn gives_to_hopper(slot: usize, face: Face, item: &ItemStack) -> bool {
    face == Face::Down && (slot == OUTPUT || slot == FUEL && item.name == fuel::BUCKET)
}

/// Changes what the furnace at `pos` owes the next player who takes its results, in hundredths.
fn owe(world: &mut World, layer: Entity, pos: BlockPos, change: impl FnOnce(i32) -> i32) -> i32 {
    let Some(Value::Compound(mut fields)) = entity(world, layer, pos) else { return 0 };
    let owed = xp::stored(&Value::Compound(fields.clone()));
    fields.retain(|(key, _)| key != xp::STORED);
    fields.push((xp::STORED.to_owned(), Value::Int(change(owed))));
    keep(world, layer, pos, &Value::Compound(fields));
    owed
}

/// A hopper took `item` out of the container at `pos`: from a furnace's result, its
/// experience is owed to the next player.
pub(crate) fn taken_by_hopper(world: &mut World, layer: Entity, pos: BlockPos, slot: usize, item: &ItemStack) {
    let is_result = slot == OUTPUT && containers::block_name(world, layer, pos).and_then(smelter).is_some();
    if is_result {
        owe(world, layer, pos, |owed| owed.saturating_add(xp::owed_for(item)));
    }
}

/// A player's request took results out of the furnace at `pos` (`before` to `after`): it is
/// paid their experience and what the furnace owed.
pub(crate) fn taken_by_player(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, before: &[Option<Held>], after: &[Option<Held>]) {
    let (Some(Some(was)), Some(now)) = (before.get(OUTPUT), after.get(OUTPUT)) else { return };
    let left = now.as_ref().filter(|held| held.stack.name == was.stack.name).map_or(0, |held| held.stack.count);
    let Some(taken) = was.stack.count.checked_sub(left).filter(|taken| *taken > 0) else { return };
    let owed = owe(world, layer, pos, |_| 0);
    let points = xp::reward(world, &was.stack, taken, owed);
    xp::pay_player(world, player, points);
}

/// The furnace at `pos` is being broken with `slots`: its result's and owed experience lies there.
pub(crate) fn broken(world: &mut World, layer: Entity, pos: BlockPos, slots: &[Option<Held>]) {
    let Some(result) = slots.get(OUTPUT).cloned().flatten().filter(|_| slots.len() == 3) else { return };
    let owed = owe(world, layer, pos, |_| 0);
    let points = xp::reward(world, &result.stack, result.stack.count, owed);
    xp::pay_at(world, layer, pos, points);
}

/// Writes a fire's times into its furnace's block entity.
fn keep_times(world: &mut World, layer: Entity, pos: BlockPos, fire: &Fire) {
    let Some(Value::Compound(mut fields)) = entity(world, layer, pos) else { return };
    fields.retain(|(key, _)| !FIELDS.contains(&key.as_str()));
    fields.extend(FIELDS.iter().zip(fire.times()).map(|(field, ticks)| ((*field).to_owned(), Value::Short(ticks as i16))));
    keep(world, layer, pos, &Value::Compound(fields));
}

/// Swaps the furnace's block for its lit or unlit one, facing as it did. The block entity goes
/// with the old block, so it is given to the new one and shown again.
fn relight(world: &mut World, layer: Entity, pos: BlockPos, smelter: &Smelter, lit: bool, fire: &Fire) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let registry = level.registry();
    let turned = registry.get(level.block(pos[0], pos[1], pos[2])).and_then(|state| registry.find(smelter.blocks[usize::from(lit)], state.properties));
    let kept = entity(world, layer, pos);
    if let (Some(kept), true) = (kept, turned.is_some_and(|id| blocks::set(world, layer, pos, id))) {
        blocks::keep_entity(world, layer, pos, &kept);
        keep_times(world, layer, pos, fire);
    }
}

/// While lit, plays the smelter's sound every 20 to 99 ticks, the first on its first lit tick.
fn crackle(world: &mut World, layer: Entity, pos: BlockPos, smelter: &Smelter, fire: &mut Fire) {
    if fire.since_crackle >= fire.next_crackle {
        fire.next_crackle = CRACKLE_EVERY + world.get_resource_or_insert_with(Dice::default).roll(CRACKLE_SPREAD);
        fire.since_crackle = 0;
        let [x, y, z] = pos.map(|c| c as f32 + 0.5);
        let sound = LevelSoundEvent { sound_id: smelter.crackle.into(), position: Vec3f { x, y, z }, extra_data: -1, entity_type: String::new(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
        blocks::to_viewers(world, layer, pos, &[sapling_net::encode(&sound)]);
    }
    fire.since_crackle += 1;
}

fn take_one(slot: &mut Option<Held>) {
    if let Some(held) = slot.as_mut().filter(|held| held.stack.count > 1) {
        held.stack.count -= 1;
    } else {
        *slot = None;
    }
}

/// One tick of the furnace at `pos`; false once it is gone or has nothing left to do and
/// nobody looking.
fn step(world: &mut World, layer: Entity, pos: BlockPos, fire: &mut Fire) -> bool {
    let Some(smelter) = containers::block_name(world, layer, pos).and_then(smelter) else { return false };
    let Some(mut slots) = containers::slots_at(world, layer, pos).filter(|slots| slots.len() > OUTPUT) else { return false };
    let made = slots[INPUT].as_ref().and_then(|held| sapling_data::smelted(smelter.station, &held.stack.name, held.stack.damage));
    let has_room = |(item, count): &&(sapling_data::CreativeItem, u8)| match &slots[OUTPUT] {
        None => true,
        Some(held) => (held.stack.name.as_str(), held.stack.damage) == (item.name, item.damage) && held.stack.count + count <= sapling_data::max_stack(item.name),
    };
    let made = made.filter(has_room);
    let (was_lit, before) = (fire.lit > 0, slots.clone());
    if was_lit {
        crackle(world, layer, pos, &smelter, fire);
    }
    fire.lit = fire.lit.saturating_sub(1);
    if let (0, Some(_), Some(ticks)) = (fire.lit, made, slots[FUEL].as_ref().and_then(|held| fuel::burn_ticks(&held.stack.name, smelter.interval))) {
        (fire.lit, fire.lit_duration) = (ticks, ticks);
        let leaves_bucket = slots[FUEL].as_ref().is_some_and(|held| held.stack.name == fuel::LAVA_BUCKET);
        take_one(&mut slots[FUEL]);
        if leaves_bucket {
            slots[FUEL] = Some(containers::new_held(world, ItemStack { name: fuel::BUCKET.to_owned(), count: 1, damage: 0, block: None, tag: None }));
        }
    }
    match made.filter(|_| fire.lit > 0) {
        Some((item, count)) if fire.cook + 1 >= smelter.interval => {
            fire.cook = 0;
            match slots[OUTPUT].as_mut() {
                Some(held) => held.stack.count += count,
                None => slots[OUTPUT] = Some(containers::new_held(world, ItemStack { name: item.name.to_owned(), count: *count, damage: item.damage, block: item.block, tag: item.tag.clone() })),
            }
            // The result is shown before the input goes, the game's order: a client that waits
            // for the input to empty has the whole result by then.
            containers::set_slots(world, layer, pos, slots.clone(), None);
            take_one(&mut slots[INPUT]);
        }
        Some(_) => fire.cook += 1,
        None if fire.lit > 0 => fire.cook = 0,
        None => fire.cook = fire.cook.saturating_sub(2),
    }
    if slots != before {
        containers::set_slots(world, layer, pos, slots, None);
    }
    if (fire.lit > 0) != was_lit {
        relight(world, layer, pos, &smelter, fire.lit > 0, fire);
    }
    let viewers = containers::viewers(world, layer, pos);
    show(world, &viewers, fire);
    let is_idle = fire.lit == 0 && fire.cook == 0;
    if is_idle && viewers.is_empty() {
        keep_times(world, layer, pos, fire);
        containers::unpin(world, layer, pos);
    }
    !(is_idle && viewers.is_empty())
}

/// Tells the screens of a furnace the times that changed since they were last told.
fn show(world: &mut World, viewers: &[(Entity, u8)], fire: &mut Fire) {
    let (now, told) = (fire.times(), fire.shown);
    if viewers.is_empty() {
        fire.shown = None;
        return;
    }
    fire.shown = Some(now);
    for (viewer, window) in viewers {
        let Some(mut out) = world.get_mut::<Outgoing>(*viewer) else { continue };
        for index in (0..PROPERTIES.len()).filter(|index| told.is_none_or(|told| told[*index] != now[*index])) {
            out.push(&ContainerSetData { window_id: WindowID::from_raw((*window).into()), property: PROPERTIES[index], value: now[index].into() });
        }
    }
}

/// Lets every watched furnace burn and smelt for a tick.
pub(crate) fn tick(world: &mut World) {
    let Some(mut furnaces) = world.get_resource_mut::<Furnaces>().filter(|furnaces| !furnaces.0.is_empty()) else { return };
    let mut fires = std::mem::take(&mut furnaces.0);
    fires.retain(|(layer, pos), fire| step(world, *layer, *pos, fire));
    // A furnace opened during the steps is in the resource already; one here is older.
    world.resource_mut::<Furnaces>().0.extend(fires);
}
