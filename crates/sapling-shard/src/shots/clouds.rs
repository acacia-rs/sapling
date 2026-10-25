//! The cloud a lingering potion leaves where it breaks (`spawn_aoe_cloud` in the pack: 30 seconds,
//! BDS's ticks of it 600, radius 3, 0.5 less each time it gives, the same one again 40 ticks on),
//! shown as the dragon's are (dragon/cloud.rs). Every fifth tick from its tenth it gives the potion
//! a quarter of its time to each player and mob standing in it; under half a block wide it is gone.
// TODO: the wait, the pulse, the quarter share and the shrinking over its life are the Java game's;
// BDS copies the effects unscaled into the cloud and where it shares them out was not found.

use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::{InLayer, Mode, Position, Tick, in_world};
use crate::dragon::cloud::{self, Shown};
use crate::{entities, mobs};

const TICKS: u64 = 600;
const RADIUS: f32 = 3.0;
const ON_USE: f32 = -0.5;
const AGAIN: u64 = 40;
const WAITS: u64 = 10;
const PULSE: u64 = 5;
const SHARE: f32 = 0.25;
const SMALLEST: f32 = 0.5;
/// How far above and below it a body's feet may be and stand in it.
const DEPTH: [f32; 2] = [-1.8, 0.5];

#[derive(Component)]
struct Lingering {
    potion: i16,
    radius: f32,
    born: u64,
    given: HashMap<Entity, u64>,
}

/// A cloud of the potion of data value `potion` at `at`.
pub(crate) fn spawn(world: &mut World, layer: Entity, at: [f32; 3], potion: i16) -> Option<Entity> {
    let colour = sapling_data::shots::potion_colour(potion);
    let shown = Shown { radius: RADIUS, ticks: TICKS, colour, potion, shrinks: -RADIUS / TICKS as f32 };
    let lingering = entities::spawn(world, layer, cloud::KIND, Body::new(at, [0.0; 3], cloud::SIZE), at, |id| Some(cloud::announcement(id, at, 0.0, shown)))?;
    let born = world.resource::<Tick>().0;
    world.entity_mut(lingering).insert(Lingering { potion, radius: RADIUS, born, given: HashMap::new() });
    Some(lingering)
}

/// Who stands in a cloud at `at` of `radius` in `layer`: players that may be harmed, then mobs.
fn within(world: &mut World, layer: Entity, at: [f32; 3], radius: f32) -> Vec<(Entity, bool)> {
    let inside = |feet: [f32; 3]| {
        let rise = feet[1] - at[1];
        (feet[0] - at[0]).powi(2) + (feet[2] - at[2]).powi(2) <= radius * radius && (DEPTH[0]..=DEPTH[1]).contains(&rise)
    };
    let players = in_world(world).into_iter().filter(|player| {
        world.get::<InLayer>(*player).is_some_and(|on| on.0 == layer)
            && world.get::<Mode>(*player).is_some_and(|mode| matches!(mode.0, sapling_api::GameMode::Survival | sapling_api::GameMode::Adventure))
            && world.get::<Position>(*player).is_some_and(|position| inside(position.feet))
    });
    let players: Vec<(Entity, bool)> = players.map(|player| (player, true)).collect();
    let mut mobs = world.query_filtered::<(Entity, &InLayer, &Body), With<crate::creatures::Creature>>();
    let mobs: Vec<(Entity, bool)> = mobs.iter(world).filter(|(_, on, body)| on.0 == layer && inside(body.at)).map(|(mob, ..)| (mob, false)).collect();
    players.into_iter().chain(mobs).collect()
}

fn pulse(world: &mut World, lingering: Entity, layer: Entity, now: u64) -> Option<()> {
    let at = world.get::<Body>(lingering)?.at;
    let kept = world.get::<Lingering>(lingering)?;
    let (potion, age) = (kept.potion, now.saturating_sub(kept.born));
    let radius = RADIUS - RADIUS * age as f32 / TICKS as f32 + (kept.radius - RADIUS);
    if age > TICKS || radius < SMALLEST {
        mobs::remove(world, lingering);
        return None;
    }
    if age < WAITS || age % PULSE != 0 {
        return Some(());
    }
    world.get_mut::<Lingering>(lingering)?.given.retain(|_, until| *until > now);
    for (victim, is_player) in within(world, layer, at, radius) {
        if world.get::<Lingering>(lingering)?.given.contains_key(&victim) {
            continue;
        }
        let mut kept = world.get_mut::<Lingering>(lingering)?;
        kept.given.insert(victim, now + AGAIN);
        kept.radius += ON_USE;
        match is_player {
            true => crate::vitals::splashed(world, victim, potion, SHARE, at),
            false => crate::creatures::splashed(world, victim, potion, SHARE),
        }
    }
    Some(())
}

/// A tick of every lingering cloud.
pub(crate) fn tick(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let clouds: Vec<(Entity, Entity)> = world.query_filtered::<(Entity, &InLayer), With<Lingering>>().iter(world).map(|(lingering, layer)| (lingering, layer.0)).collect();
    for (lingering, layer) in clouds {
        pulse(world, lingering, layer, now);
    }
}
