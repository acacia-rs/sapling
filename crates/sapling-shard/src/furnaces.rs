//! Furnaces, blast furnaces and smokers at work: each tick a watched one burns its fuel down,
//! lights the next piece when it has something to smelt, and smelts. The order within a tick is
//! BDS's `FurnaceBlockActor::tick` (docs/research/containers-crafting.md, section D). Its slots
//! are a container's (containers.rs): 0 what is smelted, 1 the fuel, 2 the result.
// TODO: a furnace is watched from the first time someone opens it until it has nothing left to
// do; one that was burning when its column was saved stands still until it is opened. No
// experience for the results, no crackling sound.

mod fuel;

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use acacia_proto::packets::ContainerSetData;
use acacia_proto::types::WindowID;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_data::Station;
use sapling_world::ItemStack;

use crate::components::{Layer, Outgoing};
use crate::inventory::Held;
use crate::{blocks, containers};

const INPUT: usize = 0;
const FUEL: usize = 1;
const OUTPUT: usize = 2;
/// The `ContainerSetData` properties of a furnace's screen: ticks cooked, ticks the fire has
/// left, ticks the burning piece of fuel gave.
const PROPERTIES: [i32; 3] = [0, 1, 2];
/// The fields of the block entity that hold the same three.
const FIELDS: [&str; 3] = ["CookTime", "BurnTime", "BurnDuration"];

/// A kind of smelter: its recipes, the ticks an item takes, and its block unlit and lit.
struct Smelter {
    station: Station,
    interval: u16,
    blocks: [&'static str; 2],
}

fn smelter(block: &str) -> Option<Smelter> {
    let kinds = [
        Smelter { station: Station::Furnace, interval: 200, blocks: ["minecraft:furnace", "minecraft:lit_furnace"] },
        Smelter { station: Station::BlastFurnace, interval: 100, blocks: ["minecraft:blast_furnace", "minecraft:lit_blast_furnace"] },
        Smelter { station: Station::Smoker, interval: 100, blocks: ["minecraft:smoker", "minecraft:lit_smoker"] },
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

/// Starts ticking the furnace at `pos`, whose slots the containers hold, from the times its
/// block entity kept; one that ticks already tells its screens all its times again.
pub(crate) fn watch(world: &mut World, layer: Entity, pos: BlockPos) {
    let kept = entity(world, layer, pos);
    let time = |field: &str| if let Some(Value::Short(ticks)) = kept.as_ref().and_then(|entity| entity.get(field)) { (*ticks).max(0) as u16 } else { 0 };
    let fire = Fire { cook: time(FIELDS[0]), lit: time(FIELDS[1]), lit_duration: time(FIELDS[2]), shown: None };
    world.get_resource_or_insert_with(Furnaces::default).0.entry((layer, pos)).or_insert(fire).shown = None;
}

pub(crate) fn is_watched(world: &World, layer: Entity, pos: BlockPos) -> bool {
    world.get_resource::<Furnaces>().is_some_and(|furnaces| furnaces.0.contains_key(&(layer, pos)))
}

/// Writes a fire's times into its furnace's block entity.
fn keep_times(world: &mut World, layer: Entity, pos: BlockPos, fire: &Fire) {
    let Some(Value::Compound(mut fields)) = entity(world, layer, pos) else { return };
    fields.retain(|(key, _)| !FIELDS.contains(&key.as_str()));
    fields.extend(FIELDS.iter().zip(fire.times()).map(|(field, ticks)| ((*field).to_owned(), Value::Short(ticks as i16))));
    if let Some(mut level) = world.get_mut::<Layer>(layer) {
        level.world.set_block_entity(pos, &Value::Compound(fields));
    }
}

/// Swaps the furnace's block for its lit or unlit one, facing as it did. The block entity goes
/// with the old block, so it is written again.
fn relight(world: &mut World, layer: Entity, pos: BlockPos, smelter: &Smelter, lit: bool, fire: &Fire) {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return };
    let registry = level.registry();
    let turned = registry.get(level.block(pos[0], pos[1], pos[2])).and_then(|state| registry.find(smelter.blocks[usize::from(lit)], state.properties));
    if turned.is_some_and(|id| blocks::set(world, layer, pos, id)) {
        containers::rewrite(world, layer, pos);
        keep_times(world, layer, pos, fire);
    }
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
        containers::release(world, layer, pos);
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
