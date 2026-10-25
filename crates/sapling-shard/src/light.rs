//! Light for the shard's rules. Each world keeps its own (`sapling_world::World::light_at`); the
//! shard has it worked out at the start of every tick, after the columns that came in, and again
//! before random ticks, and dims the sky's share by the time of day and the weather.

use std::f32::consts::PI;

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Light, LightBackend, Weather};

use crate::components::Layer;
use crate::handlers::Ctx;
use crate::worlds;

const DAY: f32 = 24000.0;
/// What the sky loses of its light at most by night, and what rain and thunder each take of it.
const DARKEST: f32 = 11.0;
const CLOUDED: f32 = 5.0 / 16.0;

/// Works out the light of every world of the shard anew where blocks were written or columns came in.
pub(crate) fn relight(world: &mut World) {
    for mut layer in world.query::<&mut Layer>().iter_mut(world).filter(|layer| layer.world.needs_relight()) {
        layer.world.relight();
    }
}

/// How far round the sun is at `time`, 0 to 1 from noon (the Java game's `getTimeOfDay`).
pub(crate) fn sun_turn(time: i64) -> f32 {
    let day = (time.rem_euclid(DAY as i64) as f32 / DAY - 0.25).rem_euclid(1.0);
    day + (0.5 - (day * PI).cos() / 2.0 - day) / 3.0
}

/// What the night and the weather take from sky light (BDS's `skyDarken`), 0 by a clear day to
/// 11 at midnight: the Java game's reckoning, by which daylight detectors gave all but one of the
/// twenty readings BDS showed (research/captures/bds-daylight.txt).
pub(crate) fn sky_darken(time: i64, weather: Weather) -> u8 {
    let (rain, thunder) = (f32::from(u8::from(weather != Weather::Clear)), f32::from(u8::from(weather == Weather::Thunder)));
    let light = (1.0 - (1.0 - ((sun_turn(time) * 2.0 * PI).cos() * 2.0 + 0.5)).clamp(0.0, 1.0)) * (1.0 - rain * CLOUDED) * (1.0 - thunder * CLOUDED);
    ((1.0 - light) * DARKEST) as u8
}

/// The raw brightness of a cell of a world (`World::brightness_at`) at its time and weather now.
pub(crate) fn brightness(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<u8> {
    let weather = crate::weather::of(world, layer);
    let level = &world.get::<Layer>(layer)?.world;
    level.brightness_at(x, y, z, sky_darken(level.level().time, weather))
}

impl LightBackend for Ctx<'_> {
    fn light_at(&self, [x, y, z]: BlockPos) -> Option<Light> {
        let level = &self.world.get::<Layer>(worlds::current(self.world))?.world;
        level.light_at(x, y, z).map(|(sky, block)| Light { sky, block })
    }

    fn brightness_at(&self, pos: BlockPos) -> Option<u8> {
        brightness(self.world, worlds::current(self.world), pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sky_darkens_by_night_and_under_rain() {
        assert_eq!((sky_darken(6000, Weather::Clear), sky_darken(18000, Weather::Clear), sky_darken(6000, Weather::Rain), sky_darken(6000, Weather::Thunder)), (0, 11, 3, 5));
    }
}
