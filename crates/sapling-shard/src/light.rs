//! Light for the shard's rules. Each world keeps its own (`sapling_world::World::light_at`); the
//! shard has it worked out at the start of every tick, after the columns that came in, and again
//! before random ticks, and dims the sky's share by the time of day and the weather.

use std::f32::consts::PI;

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Light, LightBackend};

use crate::components::Layer;
use crate::weather::Falling;
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

/// What the night and the weather take from sky light (BDS's `skyDarken`, `Dimension` tick
/// 14187f9c0), 0 by a clear day to 11 at midnight, rain and lightning each by how hard they fall
/// now: the Java game's reckoning, by which daylight detectors gave all but one of the twenty
/// readings BDS showed (research/captures/bds-daylight.txt), and 14 then 13 a second and a half
/// apart as rain set in.
pub(crate) fn sky_darken(time: i64, falling: Falling) -> u8 {
    let light = (1.0 - (1.0 - ((sun_turn(time) * 2.0 * PI).cos() * 2.0 + 0.5)).clamp(0.0, 1.0)) * (1.0 - falling.rain * CLOUDED) * (1.0 - falling.lightning * CLOUDED);
    ((1.0 - light) * DARKEST) as u8
}

/// What the night and the weather take from a world's sky light now ([`sky_darken`]).
pub(crate) fn darken(world: &World, layer: Entity) -> u8 {
    let falling = crate::weather::falling(world, layer);
    world.get::<Layer>(layer).map_or(0, |level| sky_darken(level.world.level().time, falling))
}

/// The raw brightness of a cell of a world (`World::brightness_at`) at its time and weather now.
pub(crate) fn brightness(world: &World, layer: Entity, [x, y, z]: BlockPos) -> Option<u8> {
    let darken = darken(world, layer);
    world.get::<Layer>(layer)?.world.brightness_at(x, y, z, darken)
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

    fn falling(rain: f32, lightning: f32) -> Falling {
        Falling { rain, lightning }
    }

    #[test]
    fn the_sky_darkens_by_night_and_under_rain() {
        let (clear, rain, thunder) = (falling(0.0, 0.0), falling(1.0, 0.0), falling(1.0, 1.0));
        assert_eq!((sky_darken(6000, clear), sky_darken(18000, clear), sky_darken(6000, rain), sky_darken(6000, thunder)), (0, 11, 3, 5));
    }

    /// bds-daylight.txt: noon's 15 fell to 14 some 30 ticks into the rain and 13 some 60 in; 4 s
    /// into the thunder that followed, 10; 2 s after `/weather clear`, back to 12.
    #[test]
    fn the_sky_darkens_as_the_rain_and_lightning_fall_harder() {
        let shown = |rain: f32, lightning: f32| 15 - sky_darken(6000, falling(rain, lightning));
        assert_eq!([0.29, 0.31, 0.61].map(|rain| shown(rain, 0.0)), [15, 14, 13]);
        assert_eq!((shown(1.0, 0.67), shown(0.59, 0.39)), (10, 12));
    }
}
