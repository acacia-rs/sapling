//! Bone meal on a plant, as BDS grew them for a creative player (docs/research/bone-meal.md).
// Saplings take theirs in saplings.rs.
// TODO: outside creative a crop's step is the Java
// game's 2 to 5, which BDS's steps of 2 to 4 fit; where grass and flowers come up, and which, is a guess at BDS's scatter; vines,
// cocoa, moss, sea grass and the rest take none.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{Block, BlockPos};

use super::{Dice, level, send, with_property};

const RIPE: u8 = 7;
const RIPE_BUSH: u8 = 3;
/// What a stem grows by a click: BDS's went 0, 3, 6, 7.
const STEM_STEP: u8 = 3;
const CANE_HEIGHT: i32 = 3;
const BAMBOO_HEIGHT: i32 = 12;
const CROPS: [&str; 4] = ["wheat", "carrots", "potatoes", "beetroot"];
const STEMS: [&str; 2] = ["melon_stem", "pumpkin_stem"];
const FLOWERS: [&str; 12] =
    ["poppy", "dandelion", "azure_bluet", "oxeye_daisy", "cornflower", "allium", "blue_orchid", "lily_of_the_valley", "red_tulip", "orange_tulip", "white_tulip", "pink_tulip"];
/// Tries at a cell beside a flower, and around a grass block; one sprout in this many is a flower.
const FLOWER_TRIES: usize = 6;
const SPROUT_TRIES: usize = 24;
const FLOWER_IN: f32 = 8.0;
const ROLLS: usize = 3 * SPROUT_TRIES + 1;
const MUSHROOM_ODDS: f32 = 0.4;

type Level = sapling_world::World;

fn name_at(level: &Level, [x, y, z]: BlockPos) -> Option<&str> {
    level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().map(|state| state.name.strip_prefix("minecraft:").unwrap_or(state.name))
}

fn is_air(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_air())
}

fn state(level: &Level, name: &str, properties: &str) -> Option<u32> {
    send::resolve(level.registry(), &Block { name: format!("minecraft:{name}"), properties: properties.to_owned() })
}

/// The top of the run of `name` through `pos`, and how many of it are under that.
fn top_of(level: &Level, [x, y, z]: BlockPos, name: &str) -> (i32, i32) {
    let is = |y: i32| name_at(level, [x, y, z]) == Some(name);
    let top = (y..).take_while(|y| is(*y)).last().unwrap_or(y);
    (top, (0..).take_while(|down| is(top - down)).count() as i32)
}

/// `plant` on the free cells over grass that `tries` picks beside `pos`, at most `reach` off.
fn scattered(level: &Level, [x, y, z]: BlockPos, reach: i32, tries: usize, roll: &mut dyn FnMut() -> f32, plant: &mut dyn FnMut(f32) -> Option<u32>) -> Vec<(BlockPos, u32)> {
    let mut grown: Vec<(BlockPos, u32)> = Vec::new();
    let span = (2 * reach + 1) as f32;
    for _ in 0..tries {
        let cell = [x + (roll() * span) as i32 - reach, y, z + (roll() * span) as i32 - reach];
        let which = roll();
        let is_free = is_air(level, cell) && name_at(level, [cell[0], y - 1, cell[2]]) == Some("grass_block") && !grown.iter().any(|(at, _)| *at == cell);
        if let Some(id) = plant(which).filter(|_| is_free) {
            grown.push((cell, id));
        }
    }
    grown
}

/// The blocks bone meal on the block at `pos` sets: `None` for what takes none, and nothing for
/// a plant that takes it and stays as it is.
fn grown(level: &Level, pos: BlockPos, creative: bool, roll: &mut dyn FnMut() -> f32) -> Option<Vec<(BlockPos, u32)>> {
    let [x, y, z] = pos;
    let id = level.block(x, y, z);
    let properties = level.registry().get(id)?.properties;
    let name = name_at(level, pos)?;
    let growth: Option<u8> = properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(key, _)| *key == "growth").and_then(|(_, value)| value.parse().ok());
    let grow_to = |to: u8| with_property(level.registry(), id, "growth", &to.to_string()).map(|id| vec![(pos, id)]);
    let above = [x, y + 1, z];
    match name {
        // A ripe crop still takes it; a ripe stem and a ripe bush, which is picked, do not.
        _ if CROPS.contains(&name) => match growth? {
            // A ripe crop took it in creative and not in survival.
            RIPE.. => creative.then(Vec::new),
            now => grow_to(if creative { RIPE } else { (now + 2 + (roll() * 4.0) as u8).min(RIPE) }),
        },
        _ if STEMS.contains(&name) => growth.filter(|now| *now < RIPE).and_then(|now| grow_to((now + STEM_STEP).min(RIPE))),
        "sweet_berry_bush" => growth.filter(|now| *now < RIPE_BUSH).and_then(|now| grow_to(if creative { RIPE_BUSH } else { now + 1 })),
        "short_grass" | "fern" if is_air(level, above) => {
            let tall = if name == "fern" { "large_fern" } else { "tall_grass" };
            Some(vec![(pos, state(level, tall, "upper_block_bit=0")?), (above, state(level, tall, "upper_block_bit=1")?)])
        }
        _ if FLOWERS.contains(&name) => Some(scattered(level, pos, 1, FLOWER_TRIES, roll, &mut |_| Some(id))),
        "grass_block" if is_air(level, above) => {
            let mut sprout = |which: f32| state(level, if which * FLOWER_IN < 1.0 { FLOWERS[(which * FLOWER_IN * 3.0) as usize % 3] } else { "short_grass" }, "");
            let mut sprouts = vec![(above, sprout(1.0)?)];
            let around = scattered(level, above, 2, SPROUT_TRIES, roll, &mut sprout);
            sprouts.extend(around.into_iter().filter(|(cell, _)| *cell != above));
            Some(sprouts)
        }
        "reeds" => {
            let (top, tall) = top_of(level, pos, name);
            Some((1..=CANE_HEIGHT - tall).map(|up| [x, top + up, z]).take_while(|cell| is_air(level, *cell)).map(|cell| (cell, id)).collect())
        }
        "bamboo_sapling" if is_air(level, above) => Some(vec![(pos, state(level, "bamboo", "bamboo_leaf_size=no_leaves")?), (above, state(level, "bamboo", "bamboo_leaf_size=small_leaves")?)]),
        "bamboo" => {
            let (top, tall) = top_of(level, pos, name);
            let over = [x, top + 1, z];
            Some(if tall < BAMBOO_HEIGHT && is_air(level, over) { vec![(over, state(level, "bamboo", "bamboo_leaf_size=small_leaves")?)] } else { Vec::new() })
        }
        _ => None,
    }
}

/// Bone meal on kelp, cave vines and mushrooms, by BDS's rules as decompiled (`None` for the rest):
/// the top of a stalk of kelp grows the next tick if it waited to (142fdcbe0); a cave vine without
/// berries gets them (148b219b0); a mushroom grows huge four times in ten, every time in creative
/// (1465ead90), and takes the meal either way.
fn wild(world: &mut World, layer: Entity, pos: BlockPos, creative: bool) -> Option<bool> {
    let level = level(world, layer)?;
    let [x, y, z] = pos;
    let id = level.block(x, y, z);
    let block = level.registry().get(id)?;
    let (name, properties) = (block.name.strip_prefix("minecraft:").unwrap_or(block.name), block.properties);
    match name {
        "kelp" => Some(super::growth::meal_kelp(world, layer, pos)),
        "cave_vines" => {
            let has_vine_under = name_at(level, [x, y - 1, z]).is_some_and(|under| under.starts_with("cave_vines"));
            let berries = state(level, if has_vine_under { "cave_vines_body_with_berries" } else { "cave_vines_head_with_berries" }, properties)?;
            Some(send::set(world, layer, pos, berries))
        }
        "cave_vines_head_with_berries" | "cave_vines_body_with_berries" => Some(false),
        "brown_mushroom" | "red_mushroom" => {
            let grows = creative || world.get_resource_or_insert_with(Dice::default).fraction() < MUSHROOM_ODDS;
            if grows {
                huge_mushroom(world, layer, pos, name);
            }
            Some(true)
        }
        _ => None,
    }
}

/// The huge mushroom of the mushroom `name` at `pos`, where it has room and soil: worldgen's
/// feature, drawn until it comes out of that colour, as BDS's bone meal grows its own kind.
fn huge_mushroom(world: &mut World, layer: Entity, pos: BlockPos, name: &str) {
    const DRAWS: usize = 8;
    let Some(air) = level(world, layer).map(|level| level.registry().air_id()) else { return };
    let mushroom = level(world, layer).map_or(air, |level| level.block(pos[0], pos[1], pos[2]));
    send::set_quietly(world, layer, pos, air);
    let colour = format!("{name}_block");
    for _ in 0..DRAWS {
        let seed = world.get_resource_or_insert_with(Dice::default).roll(u32::MAX);
        let Some(level) = level(world, layer) else { return };
        let Some(grown) = level.place_feature("minecraft:huge_mushroom_feature", pos, seed).filter(|grown| !grown.is_empty()) else { break };
        if grown.iter().any(|(_, id)| level.registry().get(*id).is_some_and(|state| state.name.ends_with(&colour))) {
            send::set_many(world, layer, &grown);
            return;
        }
    }
    send::set_quietly(world, layer, pos, mushroom);
}

/// Bone meal on the block at `pos`: false when it takes none.
pub(super) fn feed(world: &mut World, layer: Entity, pos: BlockPos, creative: bool) -> bool {
    if let Some(took) = wild(world, layer, pos, creative) {
        if took {
            particle(world, layer, pos);
        }
        return took;
    }
    if let Some(took) = super::saplings::fertilized(world, layer, pos, creative) {
        if took {
            particle(world, layer, pos);
        }
        return took;
    }
    let rolls: Vec<f32> = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        (0..ROLLS).map(|_| dice.fraction()).collect()
    };
    let mut rolls = rolls.into_iter();
    let Some(grown) = level(world, layer).and_then(|level| grown(level, pos, creative, &mut || rolls.next().unwrap_or(0.5))) else { return false };
    particle(world, layer, pos);
    for (cell, id) in grown {
        send::set(world, layer, cell, id);
    }
    true
}

/// Bone meal's sparkle, from the cell's corner as BDS sent it.
fn particle(world: &mut World, layer: Entity, pos: BlockPos) {
    let [x, y, z] = pos.map(|c| c as f32);
    send::to_viewers(world, layer, pos, &[sapling_net::encode(&LevelEvent { event: LevelEventEvent::ParticleCropGrowth, position: Vec3f { x, y, z }, data: 0 })]);
}
