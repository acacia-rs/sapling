//! Each world's weather as BDS keeps and turns it, and what its players are told of it: the
//! events BDS sent for `/weather` (research/captures/bds-weather.txt). The state is the level's
//! (`Level::weather`, saved in `level.dat`): rain and lightning each have a level and the ticks
//! until it turns, which BDS's `Weather::tick` (0x14119d290) counts down while `doWeatherCycle`
//! is on, turning each when its count runs out and drawing how long the new spell lasts. A player
//! is told whenever what it was last told differs from its world's sky, so one who joins or
//! changes worlds is too.
// TODO: BDS draws the spells from the level's random, here from a hash of the tick; `query`
// answers thunder where lightning is on and rain is not, here clear; snow falls on no block.
// Lightning: lightning.rs.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{Weather, WeatherBackend};
use sapling_world::WeatherRecord;

use crate::components::{InLayer, Layer, Outgoing, Tick, in_world};
use crate::handlers::Ctx;
use crate::{dimensions, worlds};

/// What BDS sent with the start of rain and of thunder.
const FULL: i32 = 65535;
const CYCLE: &str = "doWeatherCycle";
/// A new spell lasts its least plus `nextInt` of its spread: clear sky and rain by
/// `Weather::tick`, `stopWeather` and `/toggledownfall` (0x143a48410), thunder by `Weather::tick`.
const CLEAR_SPELL: [u64; 2] = [12_000, 168_000];
const RAIN_SPELL: [u64; 2] = [12_000, 12_000];
const THUNDER_SPELL: [u64; 2] = [3_600, 12_000];
/// `/weather` without a duration: `nextInt(600) * 20 + 6000` ticks (0x143a4f000).
const COMMAND_SPELL: (u64, u64, u64) = (6_000, 600, 20);
/// Counts at or below this turn the weather (`Weather::tick` compares with 2).
const TURNS_AT: i32 = 1;

fn sky(record: &WeatherRecord) -> Weather {
    match (record.rain_level > 0.0, record.lightning_level > 0.0) {
        (false, _) => Weather::Clear,
        (true, false) => Weather::Rain,
        (true, true) => Weather::Thunder,
    }
}

fn home_record(world: &World, layer: Entity) -> Option<WeatherRecord> {
    world.get::<Layer>(dimensions::home(world, layer)).map(|home| home.world.level().weather)
}

/// The sky of a world; another dimension has its overworld's.
pub(crate) fn of(world: &World, layer: Entity) -> Weather {
    home_record(world, layer).map_or(Weather::Clear, |record| sky(&record))
}

fn put(world: &mut World, layer: Entity, record: WeatherRecord) {
    let home = dimensions::home(world, layer);
    if let Some(mut home) = world.get_mut::<Layer>(home) {
        home.world.level_mut().weather = record;
    }
    // At once, as BDS told it before it answered the command.
    tell(world);
}

/// `WeatherManager::updateWeather` as `/weather` calls it: rain and lightning on or off, both
/// for `ticks` (BDS's own draw without them).
pub(crate) fn set(world: &mut World, layer: Entity, weather: Weather, ticks: Option<u32>) {
    let now = world.resource::<Tick>().0;
    let (least, sides, step) = COMMAND_SPELL;
    let ticks = ticks.map_or(least + chance(now, layer, 0) % sides * step, u64::from);
    let time = i32::try_from(ticks).unwrap_or(i32::MAX);
    let level = |on: bool| if on { 1.0 } else { 0.0 };
    let record = WeatherRecord { rain_level: level(weather != Weather::Clear), rain_time: time, lightning_level: level(weather == Weather::Thunder), lightning_time: time };
    put(world, layer, record);
}

/// `WeatherManager::stopWeather`, as a night slept through ends: where `doWeatherCycle` is on
/// and it rains or thunders, both stop for a clear spell each.
pub(crate) fn stop(world: &mut World, layer: Entity) {
    let Some(level) = world.get::<Layer>(dimensions::home(world, layer)).map(|home| home.world.level()) else { return };
    let wet = level.weather.rain_level != 0.0 || level.weather.lightning_level != 0.0;
    if !wet || !level.rules.on(CYCLE) {
        return;
    }
    let now = world.resource::<Tick>().0;
    let spell = |salt| spell(CLEAR_SPELL, now, layer, salt);
    put(world, layer, WeatherRecord { rain_level: 0.0, rain_time: spell(1), lightning_level: 0.0, lightning_time: spell(2) });
}

fn event(event: LevelEventEvent, data: i32) -> LevelEvent {
    LevelEvent { event, position: Vec3f { x: 0.0, y: 0.0, z: 0.0 }, data }
}

/// What tells a player the sky went from `was` to `now`.
fn turns(was: Weather, now: Weather) -> Vec<LevelEvent> {
    let (rained, rains) = (was != Weather::Clear, now != Weather::Clear);
    let (thundered, thunders) = (was == Weather::Thunder, now == Weather::Thunder);
    let mut told = Vec::new();
    if rains != rained {
        told.push(if rains { event(LevelEventEvent::StartRain, FULL) } else { event(LevelEventEvent::StopRain, 0) });
    }
    if thunders != thundered {
        told.push(if thunders { event(LevelEventEvent::StartThunder, FULL) } else { event(LevelEventEvent::StopThunder, 0) });
    }
    told
}

/// A number that differs by tick, world and draw: the shard's dice are not thrown for the weather.
fn chance(now: u64, layer: Entity, salt: u64) -> u64 {
    let mut mixed = (now ^ layer.to_bits() ^ salt.wrapping_mul(0xD6E8_FEB8_6659_FD93)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    mixed ^= mixed >> 29;
    mixed.wrapping_mul(0xBF58_476D_1CE4_E5B9) >> 16
}

fn spell([least, spread]: [u64; 2], now: u64, layer: Entity, salt: u64) -> i32 {
    (least + chance(now, layer, salt) % spread) as i32
}

/// One tick of `Weather::tick`: both counts run down; one that ran out turns its weather and
/// draws the new spell's length.
fn turned(record: WeatherRecord, now: u64, layer: Entity) -> WeatherRecord {
    let (rain_turns, lightning_turns) = (record.rain_time <= TURNS_AT, record.lightning_time <= TURNS_AT);
    let mut next = WeatherRecord { rain_time: record.rain_time - 1, lightning_time: record.lightning_time - 1, ..record };
    if rain_turns {
        let rains = record.rain_level <= 0.0;
        next.rain_level = if rains { 1.0 } else { 0.0 };
        next.rain_time = spell(if rains { RAIN_SPELL } else { CLEAR_SPELL }, now, layer, 1);
    }
    if lightning_turns {
        let thunders = record.lightning_level <= 0.0;
        next.lightning_level = if thunders { 1.0 } else { 0.0 };
        next.lightning_time = spell(if thunders { THUNDER_SPELL } else { CLEAR_SPELL }, now, layer, 2);
    }
    next
}

/// Turns the weather of every overworld where `doWeatherCycle` is on; players are told of any
/// change.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let cycling: Vec<Entity> = world.query::<(Entity, &Layer)>().iter(world).filter(|(_, layer)| layer.world.level().rules.on(CYCLE)).map(|(layer, _)| layer).collect();
    let homes: Vec<Entity> = cycling.into_iter().filter(|layer| dimensions::home(world, *layer) == *layer).collect();
    for layer in homes {
        if let Some(mut home) = world.get_mut::<Layer>(layer) {
            let level = home.world.level_mut();
            level.weather = turned(level.weather, now, layer);
        }
    }
    tell(world);
}

/// Tells each player what it has not heard of its world's sky.
fn tell(world: &mut World) {
    for player in in_world(world) {
        let Some(sky) = world.get::<InLayer>(player).map(|layer| of(world, layer.0)) else { continue };
        let was = world.get::<Told>(player).map_or(Weather::Clear, |told| told.0);
        if sky == was {
            continue;
        }
        world.entity_mut(player).insert(Told(sky));
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            turns(was, sky).iter().for_each(|told| out.push(told));
        }
    }
}

/// The weather a player was last told of.
#[derive(Component, Clone, Copy, Default)]
struct Told(Weather);

impl WeatherBackend for Ctx<'_> {
    fn weather(&self) -> Weather {
        of(self.world, worlds::current(self.world))
    }

    fn set_weather(&mut self, weather: Weather, ticks: Option<u32>) {
        let layer = worlds::current(self.world);
        set(self.world, layer, weather, ticks);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turn_of_the_sky_is_told_as_bds_told_it() {
        let names = |was, now| turns(was, now).iter().map(|told| (told.event, told.data)).collect::<Vec<_>>();
        assert_eq!(names(Weather::Clear, Weather::Rain), [(LevelEventEvent::StartRain, FULL)]);
        assert_eq!(names(Weather::Rain, Weather::Thunder), [(LevelEventEvent::StartThunder, FULL)]);
        assert_eq!(names(Weather::Thunder, Weather::Clear), [(LevelEventEvent::StopRain, 0), (LevelEventEvent::StopThunder, 0)]);
        assert_eq!(names(Weather::Rain, Weather::Rain), []);
    }

    #[test]
    fn the_chance_differs_from_tick_to_tick() {
        let drawn: std::collections::HashSet<u64> = (0..64).map(|now| chance(now, Entity::PLACEHOLDER, 1) % 1000).collect();
        assert!(drawn.len() > 48, "{}", drawn.len());
    }

    #[test]
    fn each_count_turns_its_own_weather_when_it_runs_out() {
        let rain = WeatherRecord { rain_level: 1.0, rain_time: 2, lightning_level: 0.0, lightning_time: 40 };
        let next = turned(rain, 0, Entity::PLACEHOLDER);
        assert_eq!((next.rain_level, next.rain_time, next.lightning_time), (1.0, 1, 39));
        let next = turned(next, 1, Entity::PLACEHOLDER);
        assert_eq!((next.rain_level, next.lightning_level, next.lightning_time), (0.0, 0.0, 38), "rain stopped, lightning still counting");
        assert!((12_000..180_000).contains(&next.rain_time), "a clear spell: {}", next.rain_time);
        let storm = turned(WeatherRecord { lightning_time: 1, ..rain }, 0, Entity::PLACEHOLDER);
        assert_eq!((storm.rain_level, storm.lightning_level, sky(&storm)), (1.0, 1.0, Weather::Thunder));
        assert!((3_600..15_600).contains(&storm.lightning_time), "a thunder spell: {}", storm.lightning_time);
    }
}
