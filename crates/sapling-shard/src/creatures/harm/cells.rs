//! What the cells about a mob hold, for harm.rs: blocks its box touches, sun and rain on it.

use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use crate::body::Body;
use crate::components::Layer;

/// Whether the body's box reaches into a cell holding one of `names`, however full.
pub(crate) fn touches(level: &Level, body: &Body, names: &[&str]) -> bool {
    const EDGE: f32 = 1e-4;
    let (low, high) = body.bounds();
    let cells = |axis: usize| low[axis].floor() as i32..=(high[axis] - EDGE).floor() as i32;
    cells(0).any(|x| cells(1).any(|y| cells(2).any(|z| holds(level, [x, y, z], names))))
}

/// Whether the cell holds one of `names`, as a block or as the liquid in one.
pub(crate) fn holds(level: &Level, [x, y, z]: [i32; 3], names: &[&str]) -> bool {
    let registry = level.registry();
    is_within(level, [x, y, z]) && [level.block(x, y, z), level.liquid(x, y, z)].into_iter().any(|id| registry.get(id).is_some_and(|state| names.contains(&state.name)))
}

fn is_within(level: &Level, [x, y, z]: [i32; 3]) -> bool {
    level.in_height(y) && level.is_loaded(sapling_world::ChunkPos::of_block(x, z))
}

/// It is day while the sky is darkened by no more than this (`Actor::isInDaylight`): BDS's zombie
/// burnt at 0, 12000 and 23500 and not at 12600 or 23000.
const DAY_DARKEN: u8 = 3;

/// Whether the sun reaches the cell `at` with the sky darkened by `darken`
/// (`crate::light::darken`): it is day, and the cell has the sky's full light. A stone roof and
/// leaves held it back on BDS, glass did not (research/captures/bds-sun.txt).
pub(super) fn is_sunlit(level: &Level, [x, y, z]: [i32; 3], darken: u8) -> bool {
    darken <= DAY_DARKEN && level.light_at(x, y, z).is_some_and(|(sky, _)| sky == sapling_world::FULL_LIGHT)
}

/// [`is_sunlit`] in the world `layer` now.
pub(crate) fn sun_on(world: &World, layer: Entity, at: [i32; 3]) -> bool {
    let darken = crate::light::darken(world, layer);
    world.get::<Layer>(layer).is_some_and(|layer| is_sunlit(&layer.world, at, darken))
}

/// Whether rain falls on the cell `at` while it `rains` (`weather::rains`): the sky is over it and the biome is not
/// cold enough for snow, which does not put a burning mob out (`Actor::isInWaterOrRain`).
pub(crate) fn is_rained_on(level: &Level, [x, y, z]: [i32; 3], rains: bool) -> bool {
    rains && level.light_at(x, y, z).is_some_and(|(sky, _)| sky == sapling_world::FULL_LIGHT) && !level.is_cold(x, y, z)
}

/// Whether rain falls on the feet or the head of a body standing at `feet`, `height` tall
/// (`Actor::isInRain`, the rain's level as `weather::rains`).
pub(crate) fn in_rain(world: &World, layer: Entity, feet: [f32; 3], height: f32) -> bool {
    let rains = crate::weather::rains(world, layer);
    let cells = [feet, [feet[0], feet[1] + height, feet[2]]].map(|at| at.map(|c| c.floor() as i32));
    world.get::<Layer>(layer).is_some_and(|layer| cells.iter().any(|cell| is_rained_on(&layer.world, *cell, rains)))
}

pub(crate) fn is_full_block(level: &Level, [x, y, z]: [i32; 3]) -> bool {
    let smothers = |name: &str| !name.ends_with("glass") && !name.ends_with("leaves");
    is_within(level, [x, y, z]) && level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_full_cube() && smothers(state.name))
}
