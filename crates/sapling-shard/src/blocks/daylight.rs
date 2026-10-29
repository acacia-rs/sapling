//! Daylight detectors: each gives the signal the sky over it is worth, read anew every second,
//! and a click turns one inside out (docs/research/redstone.md, "Daylight detectors").
// TODO: only detectors written while the shard runs are read: one a world folder holds stays as
// it was stored.

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
const FULL: i32 = 15;

/// Where detectors are, by world, and those written since the last tick, read on the next.
#[derive(Resource, Default)]
pub(crate) struct Detectors(HashSet<(Entity, BlockPos)>, HashSet<(Entity, BlockPos)>);

/// The signal of a detector with `sky` light in its cell at `time`: the Java game's reckoning,
/// which under an open sky gave each of the twenty readings BDS showed but one
/// (research/captures/bds-daylight.txt).
pub(super) fn signal(time: i64, weather: Weather, sky: u8) -> u8 {
    let bright = bright(time, weather, sky);
    let sun = crate::light::sun_turn(time) * 2.0 * PI;
    let towards = if sun < PI { 0.0 } else { 2.0 * PI };
    (bright as f32 * (sun + (towards - sun) * 0.2).cos()).round().clamp(0.0, FULL as f32) as u8
}

/// The sky light in the cell, darkened for the time and weather; an inverted detector gives 15 less
/// this, without the sun's angle (research/captures/bds-rsplace.txt: 0, 6, 11, 6 at 6000, 13000,
/// 18000 and 23000).
fn bright(time: i64, weather: Weather, sky: u8) -> i32 {
    i32::from(sky) - i32::from(crate::light::sky_darken(time, weather))
}

fn is_detector(name: &str) -> bool {
    name == DETECTOR || name == INVERTED
}

/// A cell was written: a detector there is read on the next tick, as BDS read one put down
/// (bds-rsplace.txt), and every second from then on.
pub(super) fn written(world: &mut World, layer: Entity, pos: BlockPos) {
    let is_one = level(world, layer).and_then(|level| level.registry().get(level.block(pos[0], pos[1], pos[2]))).is_some_and(|state| is_detector(state.name));
    if is_one {
        let mut detectors = world.get_resource_or_insert_with(Detectors::default);
        detectors.0.insert((layer, pos));
        detectors.1.insert((layer, pos));
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
    let (time, sky) = (level.level().time, level.light_at(x, y, z).map_or(0, |(sky, _)| sky));
    let shown = match state.name == INVERTED {
        true => (FULL - bright(time, weather, sky)).clamp(0, FULL) as u8,
        false => signal(time, weather, sky),
    };
    if let Some(now) = super::with_property(registry, id, SIGNAL, &shown.to_string()) {
        send::set(world, layer, [x, y, z], now);
    }
    true
}

/// Reads every detector each second, and those written since the last tick.
pub(crate) fn tick(world: &mut World) {
    let every = world.resource::<Tick>().0 % EVERY == 0;
    let Some(mut detectors) = world.get_resource_mut::<Detectors>() else { return };
    let fresh = std::mem::take(&mut detectors.1);
    let detectors: Vec<(Entity, BlockPos)> = match every {
        true => detectors.0.iter().copied().collect(),
        false => fresh.into_iter().collect(),
    };
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
        let read: Vec<u8> = [0, 1000, 2000, 3000, 4000, 5000, 6000, 8000, 10000, 11000, 12000, 12500, 13000, 13500, 14000, 18000, 22000, 22500, 23500].map(|time| signal(time, Weather::Clear, FULL as u8)).into();
        assert_eq!(read, [7, 10, 12, 13, 14, 15, 15, 14, 12, 10, 7, 4, 2, 1, 0, 0, 0, 1, 4]);
        assert_eq!(signal(6000, Weather::Thunder, FULL as u8), 10, "noon under thunder");
        assert_eq!((signal(6000, Weather::Clear, 13), signal(6000, Weather::Clear, 0)), (13, 0), "under leaves, and under a roof");
    }
}
