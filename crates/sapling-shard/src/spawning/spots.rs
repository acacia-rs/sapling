//! Where in a block column mobs may appear, as BDS's `findNextSpawnBlockUnder` walks it from the
//! top down: the first spot, then one more at every lower level (each floor of air, each body of
//! water or lava), and what the spawn rules test of each (`sapling_data::Spot`).

use bevy_ecs::prelude::*;
use sapling_data::Medium;
use sapling_world::World as Level;

use crate::components::Layer;

/// The sky's dimming BDS reckons brightness by for `adjust_for_weather` in a thunderstorm.
const STORM_DARKEN: u8 = 10;
/// Where in its cell a mob appears across (`_spawnMobInCluster`, worlds of 1.21 on).
const INSET: f32 = 0.49;

/// A cell a mob could stand in, as the rules see it.
#[derive(Clone, Copy, Debug)]
pub(super) struct Found {
    pub cell: [i32; 3],
    pub biome: u32,
    pub surface: bool,
    pub medium: Medium,
    pub brightness: u8,
    pub weathered: u8,
    pub below: &'static str,
}

impl Found {
    pub fn feet(&self) -> [f32; 3] {
        let [x, y, z] = self.cell;
        [x as f32 + INSET, y as f32, z as f32 + INSET]
    }

    pub fn spot(&self, difficulty: u8, distance: f32) -> sapling_data::Spot<'static> {
        sapling_data::Spot {
            biome: self.biome,
            surface: self.surface,
            medium: self.medium,
            brightness: self.brightness,
            weathered: self.weathered,
            difficulty,
            y: self.cell[1],
            distance,
            below: self.below,
        }
    }
}

/// Every spot of block column `(x, z)` of `layer`, top first.
pub(super) fn levels(world: &World, layer: Entity, x: i32, z: i32) -> Vec<Found> {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return Vec::new() };
    let Some(top) = level.top_solid(x, z) else { return Vec::new() };
    let heights = level.heights();
    let (darken, storms) = (crate::light::darken(world, layer), crate::weather::storms(world, layer));
    let mut found = Vec::new();
    let mut above = None;
    for y in (heights.start + 1..heights.end).rev() {
        let here = medium(level, [x, y, z]);
        if let Some(medium) = here.filter(|medium| above != Some(*medium)) {
            found.extend(describe(level, [x, y, z], medium, top, darken, storms));
        }
        above = here;
    }
    found
}

/// The first spot from `cell` down in its block column of `layer`, a water one only if `water`.
pub(super) fn under(world: &World, layer: Entity, [x, y, z]: [i32; 3], water: bool) -> Option<Found> {
    let level = &world.get::<Layer>(layer)?.world;
    let top = level.top_solid(x, z)?;
    let (darken, storms) = (crate::light::darken(world, layer), crate::weather::storms(world, layer));
    let y = (level.heights().start + 1..=y.min(level.heights().end - 1)).rev().find(|&y| medium(level, [x, y, z]).is_some_and(|m| water || m != Medium::Water))?;
    describe(level, [x, y, z], medium(level, [x, y, z])?, top, darken, storms)
}

fn describe(level: &Level, [x, y, z]: [i32; 3], medium: Medium, top: i32, darken: u8, storms: bool) -> Option<Found> {
    let brightness = level.brightness_at(x, y, z, darken)?;
    let weathered = if storms { level.brightness_at(x, y, z, STORM_DARKEN)? } else { brightness };
    let below = level.registry().get(level.block(x, y - 1, z))?.name;
    Some(Found { cell: [x, y, z], biome: level.biome(x, y, z)?, surface: y > top, medium, brightness, weathered, below })
}

/// What a mob in `cell` would be in: water, or air over lava or over a full block; `None` for a
/// cell nothing can appear in.
fn medium(level: &Level, [x, y, z]: [i32; 3]) -> Option<Medium> {
    let registry = level.registry();
    let feet = registry.get(level.block(x, y, z))?;
    if feet.is_water() || registry.get(level.liquid(x, y, z)).is_some_and(|liquid| liquid.is_water()) {
        return Some(Medium::Water);
    }
    if feet.is_solid() || feet.is_liquid() {
        return None;
    }
    let below = registry.get(level.block(x, y - 1, z))?;
    if below.is_lava() {
        return Some(Medium::Lava);
    }
    below.is_full_cube().then_some(Medium::Air)
}

/// Whether `cell` of `layer` is a bubble column, where a `disallow_spawns_in_bubble` kind does not appear.
pub(super) fn in_bubble(world: &World, layer: Entity, [x, y, z]: [i32; 3]) -> bool {
    world.get::<Layer>(layer).is_some_and(|layer| layer.world.registry().get(layer.world.block(x, y, z)).is_some_and(|state| state.name == "minecraft:bubble_column"))
}

/// Whether a mob `height` tall standing in `cell` has room: nothing with collision where its body is.
pub(super) fn has_room(level: &Level, [x, y, z]: [i32; 3], height: f32) -> bool {
    let registry = level.registry();
    (y..y + height.ceil().max(1.0) as i32).all(|y| registry.get(level.block(x, y, z)).is_some_and(|state| !state.is_solid()))
}
