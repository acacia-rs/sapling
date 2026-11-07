//! A farmer farming in working hours (its pack's `behavior.harvest_farm_block`, defaults:
//! `search_range` 16, `search_height` 1, `goal_radius` 1.5, 0.5 s after each reap or sow, up to
//! 8 s before searching again when none was found): it walks to a ripe crop and breaks it, then
//! sows the farmland under it from its pantry, or sows bare farmland. BDS (bds-an1farm.txt) broke
//! a ripe wheat every 0.7 to 1.5 s, heard `place` at the farmland's middle two ticks on, and took
//! the drops up at once (pantry.rs).

use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use super::pantry::Pantry;
use super::Trader;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, Tick};

const FARMER: i32 = 1;
/// Each crop, what sows it, and its ripe `growth`.
const CROPS: [(&str, &str); 4] = [
    ("minecraft:wheat", "minecraft:wheat_seeds"),
    ("minecraft:carrots", "minecraft:carrot"),
    ("minecraft:potatoes", "minecraft:potato"),
    ("minecraft:beetroot", "minecraft:beetroot_seeds"),
];
const RIPE: &str = "7";
const FARMLAND: &str = "minecraft:farmland";
const ACROSS: i32 = 16;
const UP: i32 = 1;
const REACHED: f32 = 1.5;
/// `minecraft:movement` 0.5 times `speed_multiplier` 0.5.
const WALKS: f32 = 0.25;
const AFTER: u64 = 10;
const SEARCH_WAIT: u32 = 160;

/// The cell a farmer is bound for, and the tick it may next search.
#[derive(Component, Default)]
pub(super) struct Farming {
    cell: Option<[i32; 3]>,
    next: u64,
}

fn name_at(level: &Level, [x, y, z]: [i32; 3]) -> Option<(&'static str, Option<&'static str>)> {
    level.in_height(y).then(|| level.registry().get(level.block(x, y, z)).map(|state| (state.name, state.property("growth")))).flatten()
}

/// What sows `crop` of [`CROPS`].
fn seed_of(crop: &str) -> Option<&'static str> {
    CROPS.iter().find(|(name, _)| *name == crop).map(|(_, seed)| *seed)
}

/// The seed the farmer would sow, of those it holds.
fn sowing(pantry: Option<&Pantry>) -> Option<&'static str> {
    CROPS.iter().map(|(_, seed)| *seed).find(|seed| pantry.is_some_and(|pantry| pantry.count(seed) > 0))
}

/// Whether the farmer has work at `cell`: a ripe crop, or bare farmland under air it can sow.
fn wanted(level: &Level, cell: [i32; 3], sows: bool) -> bool {
    let below = name_at(level, [cell[0], cell[1] - 1, cell[2]]).map(|(name, _)| name);
    match name_at(level, cell) {
        Some((crop, Some(growth))) if seed_of(crop).is_some() => growth == RIPE,
        Some(("minecraft:air", _)) => sows && below == Some(FARMLAND),
        _ => false,
    }
}

fn nearest(level: &Level, feet: [i32; 3], sows: bool) -> Option<[i32; 3]> {
    let cells = (-UP..=UP).flat_map(|dy| (-ACROSS..=ACROSS).flat_map(move |dz| (-ACROSS..=ACROSS).map(move |dx| [feet[0] + dx, feet[1] + dy, feet[2] + dz])));
    let off = |cell: &[i32; 3]| (0..3).map(|axis| (cell[axis] - feet[axis]).pow(2)).sum::<i32>();
    cells.filter(|cell| wanted(level, *cell, sows)).min_by_key(off)
}

/// Before its move, in working hours: a grown farmer goes to farm. False with no farming to do.
pub(super) fn farm(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    if world.get::<Trader>(entity).is_none_or(|trader| trader.profession != FARMER) || crate::creatures::is_baby(world, entity) {
        return false;
    }
    let now = world.resource::<Tick>().0;
    let sows = sowing(world.get::<Pantry>(entity)).is_some();
    let feet = body.at.map(|c| c.floor() as i32);
    let (cell, next) = world.get::<Farming>(entity).map_or((None, now), |farming| (farming.cell, farming.next));
    let Some(level) = world.get::<Layer>(layer).map(|kept| &kept.world) else { return false };
    let kept = cell.filter(|cell| wanted(level, *cell, sows));
    let found = (kept.is_none() && now >= next).then(|| nearest(level, feet, sows));
    let cell = match (kept, found) {
        (Some(cell), _) => cell,
        (None, None) => return false,
        (None, Some(found)) => {
            let wait = if found.is_some() { 0 } else { u64::from(world.get_resource_or_insert_with(Dice::default).roll(SEARCH_WAIT)) };
            world.entity_mut(entity).insert(Farming { cell: found, next: now + wait });
            let Some(found) = found else { return false };
            found
        }
    };
    let centre = [cell[0] as f32 + 0.5, cell[2] as f32 + 0.5];
    if (centre[0] - body.at[0]).hypot(centre[1] - body.at[2]) > REACHED {
        crate::creatures::legs::walk(world, entity, layer, body, centre, WALKS, 0.0);
        return true;
    }
    reap_and_sow(world, entity, layer, cell);
    world.entity_mut(entity).insert(Farming { cell: None, next: now + AFTER });
    crate::creatures::legs::rest(world, entity);
    true
}

/// Breaks a ripe crop at `cell` as by hand, then sows the farmland under it.
fn reap_and_sow(world: &mut World, entity: Entity, layer: Entity, cell: [i32; 3]) {
    let reaped = world.get::<Layer>(layer).and_then(|kept| name_at(&kept.world, cell)).and_then(|(crop, _)| seed_of(crop).map(|_| crop));
    if reaped.is_some() {
        crate::blocks::break_block(world, layer, cell);
    }
    let seed = reaped.and_then(seed_of).filter(|seed| world.get::<Pantry>(entity).is_some_and(|pantry| pantry.count(seed) > 0)).or_else(|| sowing(world.get::<Pantry>(entity)));
    let Some(seed) = seed else { return };
    let crop = CROPS.iter().find(|(_, sows)| *sows == seed).map(|(crop, _)| *crop).unwrap_or_default();
    let Some((id, sound)) = world.get::<Layer>(layer).and_then(|kept| {
        let registry = kept.world.registry();
        let id = crate::blocks::with_property(registry, kept.world.block_id(crop)?, "growth", "0")?;
        Some((id, crate::blocks::sound("place", registry, [cell[0], cell[1] - 1, cell[2]], id)))
    }) else {
        return;
    };
    if crate::blocks::set(world, layer, cell, id) {
        if let Some(mut pantry) = world.get_mut::<Pantry>(entity) {
            pantry.take(seed, 1);
        }
        crate::blocks::to_viewers(world, layer, cell, &[sound]);
    }
}
