//! Each world's weather, and what its players are told of it: the events BDS sent for
//! `/weather` (research/captures/bds-weather.txt). A player is told whenever what it was last
//! told differs from its world's sky, so one who joins or changes worlds is too.
// TODO: weather comes by command or plugin alone, never of itself; rain falls on no block and
// puts no fire out; lightning does not strike.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{Weather, WeatherBackend};

use crate::components::{InLayer, Layer, Outgoing, Tick, in_world};
use crate::handlers::Ctx;
use crate::worlds;

/// What BDS sent with the start of rain and of thunder.
const FULL: i32 = 65535;

/// A world's weather and the tick it clears in, if it does of itself.
#[derive(Component, Clone, Copy, Default)]
pub(crate) struct Sky {
    weather: Weather,
    until: Option<u64>,
}

/// The weather a player was last told of.
#[derive(Component, Clone, Copy, Default)]
struct Told(Weather);

pub(crate) fn of(world: &World, layer: Entity) -> Weather {
    world.get::<Sky>(layer).map_or(Weather::Clear, |sky| sky.weather)
}

pub(crate) fn set(world: &mut World, layer: Entity, weather: Weather, ticks: Option<u32>) {
    let until = ticks.map(|ticks| world.resource::<Tick>().0 + u64::from(ticks));
    if world.get::<Layer>(layer).is_some() {
        world.entity_mut(layer).insert(Sky { weather, until });
        // At once, as BDS told it before it answered the command.
        tell(world);
    }
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

/// Clears the skies whose time is up; players are told of any change.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let over: Vec<Entity> = world.query::<(Entity, &Sky)>().iter(world).filter(|(_, sky)| sky.until.is_some_and(|until| now >= until)).map(|(layer, _)| layer).collect();
    for layer in over {
        world.entity_mut(layer).insert(Sky::default());
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
}
