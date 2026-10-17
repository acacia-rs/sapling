//! Daylight detectors: each gives the signal the sky over it is worth, read anew every second,
//! and a click turns one inside out (docs/research/redstone.md, "Daylight detectors").
// TODO: only detectors written while the shard runs are read: one a world folder holds stays as
// it was stored. The sky counts as open or shut, with no light let through leaves or water.

use std::collections::HashSet;
use std::f32::consts::PI;

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Weather};

use super::{level, send};
use crate::components::Tick;

const DETECTOR: &str = "minecraft:daylight_detector";
const INVERTED: &str = "minecraft:daylight_detector_inverted";
const SIGNAL: &str = "redstone_signal";
/// Ticks between two readings.
const EVERY: u64 = 20;
const DAY: f32 = 24000.0;
const FULL: i32 = 15;
/// What the sky loses of its light at most by night, and what rain and thunder each take of it.
const DARKEST: f32 = 11.0;
const CLOUDED: f32 = 5.0 / 16.0;

/// Where detectors are, by world.
#[derive(Resource, Default)]
pub(crate) struct Detectors(HashSet<(Entity, BlockPos)>);

/// The signal of a detector under an open sky at `time`: the Java game's reckoning, which gave
/// each of the twenty readings BDS showed but one (research/captures/bds-daylight.txt).
pub(super) fn signal(time: i64, weather: Weather) -> u8 {
    let day = (time.rem_euclid(DAY as i64) as f32 / DAY - 0.25).rem_euclid(1.0);
    let turn = day + (0.5 - (day * PI).cos() / 2.0 - day) / 3.0;
    let (rain, thunder) = (f32::from(u8::from(weather != Weather::Clear)), f32::from(u8::from(weather == Weather::Thunder)));
    let light = (1.0 - (1.0 - ((turn * 2.0 * PI).cos() * 2.0 + 0.5)).clamp(0.0, 1.0)) * (1.0 - rain * CLOUDED) * (1.0 - thunder * CLOUDED);
    let bright = FULL - ((1.0 - light) * DARKEST) as i32;
    let sun = turn * 2.0 * PI;
    let towards = if sun < PI { 0.0 } else { 2.0 * PI };
    (bright as f32 * (sun + (towards - sun) * 0.2).cos()).round().clamp(0.0, FULL as f32) as u8
}

fn is_detector(name: &str) -> bool {
    name == DETECTOR || name == INVERTED
}

/// A cell was written: a detector there is read from now on.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos) {
    let is_one = level(world, layer).and_then(|level| level.registry().get(level.block(pos[0], pos[1], pos[2]))).is_some_and(|state| is_detector(state.name));
    if is_one {
        world.get_resource_or_insert_with(Detectors::default).0.insert((layer, pos));
    }
}

/// A hand turns a detector inside out, as it stands.
pub(super) fn turn(world: &mut World, layer: Entity, [x, y, z]: BlockPos) {
    let Some(level) = level(world, layer) else { return };
    let registry = level.registry();
    let Some(state) = registry.get(level.block(x, y, z)) else { return };
    let other = if state.name == DETECTOR { INVERTED } else { DETECTOR };
    let Some(turned) = registry.find(other, state.properties) else { return };
    read(world, layer, [x, y, z], Some(turned));
}

/// Reads one detector, as the state `turned` if it is being turned: false where there is none
/// any more.
fn read(world: &mut World, layer: Entity, [x, y, z]: BlockPos, turned: Option<u32>) -> bool {
    let weather = crate::weather::of(world, layer);
    let Some(level) = level(world, layer) else { return false };
    let registry = level.registry();
    let id = turned.unwrap_or_else(|| level.block(x, y, z));
    let Some(state) = registry.get(id).filter(|state| is_detector(state.name)) else { return false };
    let shut = (y + 1..).take_while(|y| level.in_height(*y)).any(|y| registry.get(level.block(x, y, z)).is_some_and(|above| above.is_solid() && !above.name.contains("glass")));
    let sky = if shut { 0 } else { signal(level.level().time, weather) };
    let shown = if state.name == INVERTED { FULL as u8 - sky } else { sky };
    if let Some(now) = super::with_property(registry, id, SIGNAL, &shown.to_string()) {
        send::set(world, layer, [x, y, z], now);
    }
    true
}

/// Reads every detector each second.
pub(crate) fn tick(world: &mut World) {
    if world.resource::<Tick>().0 % EVERY != 0 {
        return;
    }
    let detectors: Vec<(Entity, BlockPos)> = world.get_resource::<Detectors>().map(|detectors| detectors.0.iter().copied().collect()).unwrap_or_default();
    for (layer, pos) in detectors {
        if !read(world, layer, pos, None) {
            world.resource_mut::<Detectors>().0.remove(&(layer, pos));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sky_is_worth_what_bds_s_detector_gave_through_a_day() {
        let read: Vec<u8> = [0, 1000, 2000, 3000, 4000, 5000, 6000, 8000, 10000, 11000, 12000, 12500, 13000, 13500, 14000, 18000, 22000, 22500, 23500].map(|time| signal(time, Weather::Clear)).into();
        assert_eq!(read, [7, 10, 12, 13, 14, 15, 15, 14, 12, 10, 7, 4, 2, 1, 0, 0, 0, 1, 4]);
        assert_eq!(signal(6000, Weather::Thunder), 10, "noon under thunder");
    }
}
