//! How hard rain and lightning fall now, and what each rule of the sky reads of it. The levels the
//! sky turns to are 0 or 1 (`level.dat`); what falls follows each by 0.01 a tick (`Weather::tick`'s
//! 0x14119d160, rain +0x38 towards +0x3c, lightning +0x44 towards +0x48). Who reads what:
//! docs/research/mob-damage.md, "What the rain's level drives".

use bevy_ecs::prelude::*;
use sapling_world::WeatherRecord;

use crate::components::Layer;
use crate::dimensions;

const RAMP: f32 = 0.01;
/// `isPrecipitatingAt` (0x14119d6b0) rains on a block over this: 20 ticks after `/weather rain`,
/// as BDS's enderman was first hurt (research/captures/bds-enderrain.txt), 80 after it clears.
const RAINS_OVER: f32 = 0.2;
/// `LevelChunk::tick` (0x1409f98a0) draws bolts where rain times lightning is over this.
const STORMS_OVER: f32 = 0.9;

/// What falls on an overworld now, rain and lightning, 0 to 1.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct Falling {
    pub rain: f32,
    pub lightning: f32,
}

impl Falling {
    pub(super) fn of(record: &WeatherRecord) -> Self {
        Falling { rain: record.rain_level, lightning: record.lightning_level }
    }

    /// A tick nearer the levels `record` turned to.
    pub(super) fn towards(self, record: &WeatherRecord) -> Self {
        Falling { rain: ramped(self.rain, record.rain_level), lightning: ramped(self.lightning, record.lightning_level) }
    }

    /// Rain times lightning, what bolts wait on (the Java game's `getThunderLevel`); the sky's
    /// darkening takes the two apart (`light::sky_darken`).
    pub(crate) fn thunder(self) -> f32 {
        self.rain * self.lightning
    }
}

fn ramped(level: f32, towards: f32) -> f32 {
    (level + (towards - level).clamp(-RAMP, RAMP)).clamp(0.0, 1.0)
}

fn record(world: &World, layer: Entity) -> Option<WeatherRecord> {
    world.get::<Layer>(dimensions::home(world, layer)).map(|home| home.world.level().weather)
}

/// What falls now on a world (its overworld's): before its first tick, the levels it turned to.
pub(crate) fn falling(world: &World, layer: Entity) -> Falling {
    let home = dimensions::home(world, layer);
    world.get::<Falling>(home).copied().or_else(|| record(world, layer).as_ref().map(Falling::of)).unwrap_or(Falling { rain: 0.0, lightning: 0.0 })
}

/// Whether rain falls on a world's open blocks.
pub(crate) fn rains(world: &World, layer: Entity) -> bool {
    falling(world, layer).rain > RAINS_OVER
}

/// Whether any rain or snow falls, however little: what fills cauldrons and lays snow
/// (`LevelChunk::tick` weighs its odds by the level and draws from above 0).
pub(crate) fn falls(world: &World, layer: Entity) -> bool {
    falling(world, layer).rain > 0.0
}

/// Whether bolts come down: rain and lightning near their fullest together.
pub(crate) fn storms(world: &World, layer: Entity) -> bool {
    falling(world, layer).thunder() > STORMS_OVER
}

#[cfg(test)]
mod tests {
    use super::*;

    fn after(from: Falling, towards: (f32, f32), ticks: usize) -> Falling {
        let record = WeatherRecord { rain_level: towards.0, rain_time: 1, lightning_level: towards.1, lightning_time: 1 };
        (0..ticks).fold(from, |falling, _| falling.towards(&record))
    }

    #[test]
    fn rain_sets_in_over_20_ticks_and_clears_over_81() {
        let (dry, wet) = (Falling { rain: 0.0, lightning: 0.0 }, Falling { rain: 1.0, lightning: 0.0 });
        assert!(after(dry, (1.0, 0.0), 19).rain <= RAINS_OVER && after(dry, (1.0, 0.0), 20).rain > RAINS_OVER);
        assert!(after(wet, (0.0, 0.0), 80).rain > RAINS_OVER && after(wet, (0.0, 0.0), 81).rain <= RAINS_OVER);
        assert_eq!(after(dry, (1.0, 0.0), 200).rain, 1.0);
    }

    #[test]
    fn bolts_wait_for_rain_and_lightning_both_near_full() {
        let dry = Falling { rain: 0.0, lightning: 0.0 };
        let first = (1..200).find(|ticks| after(dry, (1.0, 1.0), *ticks).thunder() > STORMS_OVER);
        assert_eq!(first, Some(95));
        let raining = Falling { rain: 1.0, lightning: 0.0 };
        assert_eq!((1..200).find(|ticks| after(raining, (1.0, 1.0), *ticks).thunder() > STORMS_OVER), Some(91));
    }
}
