//! Frost Walker: boots with it freeze still water around a player on the ground into frosted
//! ice, which ages and melts back (docs/research/enchantments.md, section 6).
// TODO: the melt is the Java game's (BDS's was not read) and asks for no light, where the game
// wants more than 11 less the age.

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Face};

use super::liquid::{self, Liquid};
use super::{Dice, level, send};
use crate::components::Tick;

const FROSTED_ICE: &str = "minecraft:frosted_ice";
const OLDEST: u32 = 3;
/// Ticks from freezing to the first look at a melt, and the throw added (60 to 120).
const FIRST: [u32; 2] = [60, 61];
/// Ticks between looks after that, and the throw added (20 to 40).
const AGAIN: [u32; 2] = [20, 21];
/// Fewer frosted neighbours than this always melt a little.
const HUDDLED: usize = 4;
/// One in this many looks ages ice that has neighbours enough.
const AGES_ONE_IN: u32 = 3;

type Level = sapling_world::World;

/// Frosted ice that looks again at melting: when, and where.
#[derive(Resource, Default)]
pub(crate) struct Frost(Vec<(u64, Entity, BlockPos)>);

fn age(level: &Level, [x, y, z]: BlockPos) -> Option<u32> {
    let state = level.registry().get(level.block(x, y, z)).filter(|state| state.name == FROSTED_ICE)?;
    state.properties.split(',').find_map(|pair| pair.strip_prefix("age=")).and_then(|age| age.parse().ok())
}

fn is_still_water(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_water() && state.liquid_depth == 0)
}

fn is_air(level: &Level, [x, y, z]: BlockPos) -> bool {
    level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_air())
}

fn later(world: &mut World, layer: Entity, pos: BlockPos, [least, more]: [u32; 2]) {
    let when = world.resource::<Tick>().0 + u64::from(least + world.get_resource_or_insert_with(Dice::default).roll(more));
    world.get_resource_or_insert_with(Frost::default).0.push((when, layer, pos));
}

/// A player that moved across to `feet` with boots of Frost Walker at level `walker` (0: none)
/// freezes the still water under the air in a square around it (BDS `FrostWalkSystem`).
pub(crate) fn frost_walk(world: &mut World, layer: Entity, feet: [f32; 3], walker: u8) {
    let reach = sapling_data::enchant::frost_reach(walker);
    if reach <= 0 {
        return;
    }
    let Some(registry) = level(world, layer).map(|level| level.registry()) else { return };
    let Some(young) = registry.find(FROSTED_ICE, "age=0") else { return };
    let below = feet[1].floor() as i32 - 1;
    let [x, z] = [feet[0].floor() as i32, feet[2].floor() as i32];
    let mut frozen = Vec::new();
    for (dx, dz) in (-reach..=reach).flat_map(|dx| (-reach..=reach).map(move |dz| (dx, dz))) {
        let pos = [x + dx, below, z + dz];
        let fits = |level: &Level| is_still_water(level, pos) && is_air(level, [pos[0], below + 1, pos[2]]);
        if level(world, layer).is_some_and(fits) {
            frozen.push(pos);
        }
    }
    for pos in frozen {
        if send::set(world, layer, pos, young) {
            later(world, layer, pos, FIRST);
        }
    }
}

/// Ages the frosted ice at `pos` a step, or melts it back to water; whether it melted.
fn melt_a_little(world: &mut World, layer: Entity, pos: BlockPos) -> bool {
    let Some(age) = level(world, layer).and_then(|level| age(level, pos)) else { return false };
    if age >= OLDEST {
        return liquid::pour(world, layer, pos, Liquid::Water, false);
    }
    let older = level(world, layer).and_then(|level| level.registry().find(FROSTED_ICE, &format!("age={}", age + 1)));
    if let Some(older) = older {
        send::set(world, layer, pos, older);
    }
    false
}

/// Looks at the frosted ice that is due: it ages, the frosted ice beside it with it once it melts,
/// or waits for the next look.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let Some(mut frost) = world.get_resource_mut::<Frost>().filter(|frost| !frost.0.is_empty()) else { return };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut frost.0).into_iter().partition(|(when, ..)| *when <= now);
    frost.0 = waiting;
    for (_, layer, pos) in due {
        let Some(here) = level(world, layer).filter(|here| age(here, pos).is_some()) else { continue };
        let huddled = Face::ALL.iter().filter(|face| age(here, face.neighbour(pos)).is_some()).count();
        let ages = huddled < HUDDLED || world.get_resource_or_insert_with(Dice::default).roll(AGES_ONE_IN) == 0;
        if !ages || !melt_a_little(world, layer, pos) {
            later(world, layer, pos, AGAIN);
            continue;
        }
        for near in Face::ALL.iter().map(|face| face.neighbour(pos)) {
            let is_frosted = level(world, layer).is_some_and(|level| age(level, near).is_some());
            if is_frosted && !melt_a_little(world, layer, near) {
                later(world, layer, near, AGAIN);
            }
        }
    }
}
