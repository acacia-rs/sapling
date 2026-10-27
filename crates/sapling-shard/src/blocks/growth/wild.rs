//! Random ticks of what grows wild in caves and woods: mushrooms spread, vines climb and hang,
//! cave vines grow down (BDS's rules as decompiled, docs/research/growth.md "Wild plants").

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::super::{level, send};
use super::Picks;

const MUSHROOMS: [&str; 2] = ["minecraft:brown_mushroom", "minecraft:red_mushroom"];
const MUSHROOM_ODDS: u32 = 25;
/// A mushroom or vine stops spreading with this many of its kind in the 9x3x9 around it.
const CROWDED: usize = 5;
/// A mushroom keeps to light under this, but on these grounds.
const MUSHROOM_LIGHT: u8 = 13;
const MUSHROOM_GROUND: [&str; 2] = ["minecraft:mycelium", "minecraft:podzol"];
const VINE: &str = "minecraft:vine";
const CAVE_VINES: [&str; 3] = ["minecraft:cave_vines", "minecraft:cave_vines_head_with_berries", "minecraft:cave_vines_body_with_berries"];
const CAVE_VINE_ODDS: f32 = 0.1;
const BERRY_ODDS: f32 = 0.11;
const OLDEST: u8 = 25;

type Level = sapling_world::World;

/// Whether `name` is one this module ticks.
pub(super) fn is_ticked(name: &str) -> bool {
    MUSHROOMS.contains(&name) || name == VINE || CAVE_VINES.contains(&name)
}

fn name_at(level: &Level, [x, y, z]: BlockPos) -> Option<&'static str> {
    level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().map(|state| state.name)
}

fn properties_at(level: &Level, [x, y, z]: BlockPos) -> &'static str {
    level.in_height(y).then(|| level.registry().get(level.block(x, y, z))).flatten().map_or("", |state| state.properties)
}

fn is_air(level: &Level, pos: BlockPos) -> bool {
    level.is_loaded(send::column(pos)) && name_at(level, pos) == Some("minecraft:air")
}

fn number(properties: &str, key: &str) -> Option<u8> {
    properties.split(',').filter_map(|pair| pair.split_once('=')).find(|(name, _)| *name == key).and_then(|(_, value)| value.parse().ok())
}

/// How many of `name` are in the 9x3x9 around `pos`, counting up to [`CROWDED`].
fn crowd(level: &Level, [x, y, z]: BlockPos, name: &str) -> usize {
    let cells = (-4..=4).flat_map(|dx| (-1..=1).flat_map(move |dy| (-4..=4).map(move |dz| [x + dx, y + dy, z + dz])));
    cells.filter(|cell| name_at(level, *cell) == Some(name)).take(CROWDED).count()
}

/// A random tick of the block `name` at `pos`.
pub(super) fn ticked(world: &mut World, layer: Entity, pos: BlockPos, name: &str, picks: &mut Picks) {
    if MUSHROOMS.contains(&name) {
        mushroom(world, layer, pos, name, picks);
    } else if name == VINE {
        super::vines::ticked(world, layer, pos, picks);
    } else {
        cave_vine(world, layer, pos, name, picks);
    }
}

/// Whether a mushroom could stand in the empty cell `pos` (`MushroomBlock::canSurvive` 1465eac10).
fn mushroom_stands(level: &Level, [x, y, z]: BlockPos) -> bool {
    let below = [x, y - 1, z];
    if level.liquid(x, y, z) != level.registry().air_id() {
        return false;
    }
    let ground = name_at(level, below);
    ground.is_some_and(|ground| MUSHROOM_GROUND.contains(&ground))
        || level.brightness_at(x, y, z, 0).is_some_and(|light| light < MUSHROOM_LIGHT)
            && level.in_height(y - 1)
            && level.registry().get(level.block(x, y - 1, z)).is_some_and(|state| state.is_solid())
}

/// One in 25, a mushroom not crowded walks four random steps through cells it could stand in and
/// puts one more where a fifth step lands (`MushroomBlock::randomTick` 1465ea600).
fn mushroom(world: &mut World, layer: Entity, pos: BlockPos, name: &str, picks: &mut Picks) {
    let Some(level) = level(world, layer) else { return };
    if picks.below(MUSHROOM_ODDS) != 0 || crowd(level, pos, name) >= CROWDED {
        return;
    }
    let step = |from: BlockPos, picks: &mut Picks| {
        let dx = picks.below(3) as i32 - 1;
        let dy = picks.below(2) as i32 - picks.below(2) as i32;
        let dz = picks.below(3) as i32 - 1;
        [from[0] + dx, from[1] + dy, from[2] + dz]
    };
    let takes = |cell: BlockPos| is_air(level, cell) && mushroom_stands(level, cell);
    let mut at = pos;
    for _ in 0..4 {
        let next = step(at, picks);
        if takes(next) {
            at = next;
        }
    }
    let last = step(at, picks);
    let Some(id) = level.registry().states_of(name).next().map(|(id, _)| id).filter(|_| takes(last)) else { return };
    send::set(world, layer, last, id);
}

/// A cave vine's tip grows down one in ten, a berry one in nine of those; one with berries shows
/// them as head or body by what is under it (`CaveVinesBlock::randomTick` 148b208d0).
fn cave_vine(world: &mut World, layer: Entity, pos: BlockPos, name: &str, picks: &mut Picks) {
    let Some(level) = level(world, layer) else { return };
    let below = Face::Down.neighbour(pos);
    let under = name_at(level, below);
    let age = number(properties_at(level, pos), "growing_plant_age").unwrap_or(0);
    let has_vine_under = under.is_some_and(|under| CAVE_VINES.contains(&under));
    let berries = match name {
        "minecraft:cave_vines_head_with_berries" if has_vine_under => Some("minecraft:cave_vines_body_with_berries"),
        "minecraft:cave_vines_body_with_berries" if !has_vine_under => Some("minecraft:cave_vines_head_with_berries"),
        _ => None,
    };
    let aged = |name: &str, age: u8| level.registry().find(name, &format!("growing_plant_age={age}"));
    if let Some(id) = berries.and_then(|berries| aged(berries, age)) {
        send::set_quietly(world, layer, pos, id);
        send::show(world, layer, pos);
        return;
    }
    if has_vine_under {
        return;
    }
    let (grows, berry) = (picks.below(1 << 24) as f32 / (1 << 24) as f32, picks.below(1 << 24) as f32 / (1 << 24) as f32);
    if grows >= CAVE_VINE_ODDS || age >= OLDEST || !is_air(level, below) {
        return;
    }
    let grown = if berry < BERRY_ODDS { "minecraft:cave_vines_head_with_berries" } else { "minecraft:cave_vines" };
    if let Some(id) = aged(grown, age + 1) {
        send::set(world, layer, below, id);
    }
}
