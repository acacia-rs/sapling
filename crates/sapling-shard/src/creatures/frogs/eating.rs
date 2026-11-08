//! A frog eating a small slime or magma cube (`behavior.nearest_attackable_target`, `behavior.eat_mob`), as
//! BDS's ate both in research/captures/bds-g14frog.txt: it takes one it sees within 16 (`TargetEid`, then
//! `mob.warning` a tick on) and walks at it at 0.2 (0.0864 a tick); within 1.75 it shows its tongue
//! (`FlagsExtended` bit 38) and the prey is thrown 0.75 to it across, heard `tongue` a tick on; 8 ticks after the
//! tongue it is hidden, the tick after the prey is gone, leaving the frog's loot where the tongue met it
//! (`loot_tables/entities/frog.json`: a slime ball, or a froglight by the frog's variant), and the next tick
//! the frog's `TargetEid` is cleared.

use bevy_ecs::prelude::*;
use sapling_api::Item;

use super::super::genes::Gene;
use super::super::{hits, hunting, legs};
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId, Tick};
use crate::entities::{self, Kind};
use crate::{late_sounds, mobs};

const PREY: [&str; 2] = ["minecraft:slime", "minecraft:magma_cube"];
/// The prey's size (`is_variant` 1) and how far off the frog takes it; ticks between its looks.
const SMALL: i32 = 1;
const WITHIN: f32 = 16.0;
const LOOKS: u64 = 10;
/// `eat_mob`'s `run_speed` 2 times the frog's movement 0.1.
const PACE: f32 = 0.2;
const REACH: f32 = 1.75;
const PULL: f32 = 0.75;
const TONGUE: i64 = 1 << 38;
/// Ticks from the tongue to its hiding, and to the prey's going.
const SHOWN: u64 = 8;
const SWALLOWED: u64 = 9;
/// The loot comes this far over the prey's feet (BDS's item at 1.125 up, its box's middle at 1.25).
const LOOT_UP: f32 = 1.25;
const FROGLIGHTS: [&str; 3] = ["minecraft:ochre_froglight", "minecraft:verdant_froglight", "minecraft:pearlescent_froglight"];

#[derive(Component, Clone, Copy)]
pub(super) struct Meal {
    prey: Entity,
    /// The tick of its tongue and where the prey stood then; the tick the prey went.
    tongue: Option<(u64, [f32; 3])>,
    eaten: Option<u64>,
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}

/// The nearest small slime or magma cube within [`WITHIN`] of `body` that it sees.
fn prey_near(world: &mut World, layer: Entity, body: &Body) -> Option<Entity> {
    let near: Vec<(Entity, Body)> = world
        .query::<(Entity, &InLayer, &Kind, &Body, &Gene)>()
        .iter(world)
        .filter(|(_, in_layer, kind, prey, gene)| in_layer.0 == layer && PREY.contains(&kind.0.as_str()) && gene.0 == SMALL && distance(body.at, prey.at) <= WITHIN)
        .map(|(prey, _, _, prey_body, _)| (prey, *prey_body))
        .collect();
    let level = &world.get::<Layer>(layer)?.world;
    let eye = [body.at[0], body.at[1] + hunting::HEAD * body.size[1], body.at[2]];
    let seen = |prey: &Body| hunting::sees(level, eye, [prey.at[0], prey.at[1] + prey.size[1] / 2.0, prey.at[2]]);
    near.into_iter().filter(|(prey, prey_body)| super::super::is_alive(world, *prey) && seen(prey_body)).min_by(|a, b| distance(body.at, a.1.at).total_cmp(&distance(body.at, b.1.at))).map(|(prey, _)| prey)
}

/// Before a sitting frog's move: it looks for prey, walks at it, tongues and swallows it. False when
/// it has none and leaves the frog to sit and leap.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity, id: u64, body: &Body) -> bool {
    let now = world.resource::<Tick>().0;
    let cell = body.at.map(|c| c.floor() as i32);
    let Some(meal) = world.get::<Meal>(entity).copied() else {
        let Some(prey) = (now % LOOKS == 0).then(|| prey_near(world, layer, body)).flatten() else { return false };
        world.entity_mut(entity).insert(Meal { prey, tongue: None, eaten: None });
        let target = world.get::<NetId>(prey).map_or(0, |prey| prey.0 as i64);
        entities::tell_viewers(world, entity, layer, cell, vec![hunting::targets(id, target)]);
        late_sounds::play_later(world, 1, layer, cell, hunting::sound("mob.warning", id, super::KIND, body, hunting::HEAD));
        legs::rest(world, entity);
        return true;
    };
    if meal.eaten.is_some() {
        world.entity_mut(entity).remove::<Meal>();
        entities::tell_viewers(world, entity, layer, cell, vec![hunting::targets(id, 0)]);
        return true;
    }
    match meal.tongue {
        Some((at, spot)) if now >= at + SWALLOWED => swallow(world, entity, layer, meal.prey, spot, now),
        Some((at, _)) => {
            if now == at + SHOWN {
                let packet = super::flags(world, entity, id, 0);
                entities::tell_viewers(world, entity, layer, cell, vec![packet]);
            }
            legs::rest(world, entity);
        }
        None => approach(world, entity, layer, id, body, meal.prey, now),
    }
    true
}

/// It walks at its prey, or tongues it once within reach; a prey gone or dead is given up.
fn approach(world: &mut World, entity: Entity, layer: Entity, id: u64, body: &Body, prey: Entity, now: u64) {
    let cell = body.at.map(|c| c.floor() as i32);
    let Some(prey_at) = world.get::<Body>(prey).map(|prey| prey.at).filter(|_| super::super::is_alive(world, prey)) else {
        world.entity_mut(entity).remove::<Meal>();
        entities::tell_viewers(world, entity, layer, cell, vec![hunting::targets(id, 0)]);
        return;
    };
    let far = distance(body.at, prey_at);
    if far > REACH {
        return legs::walk(world, entity, layer, body, [prey_at[0], prey_at[2]], PACE, 0.0);
    }
    legs::rest(world, entity);
    let across = (body.at[0] - prey_at[0]).hypot(body.at[2] - prey_at[2]).max(1e-4);
    hits::throw_at_once(world, prey, layer, [(body.at[0] - prey_at[0]) / across * PULL, 0.0, (body.at[2] - prey_at[2]) / across * PULL]);
    if let Some(mut meal) = world.get_mut::<Meal>(entity) {
        meal.tongue = Some((now, prey_at));
    }
    let packet = super::flags(world, entity, id, TONGUE);
    entities::tell_viewers(world, entity, layer, cell, vec![packet]);
    late_sounds::play_later(world, 1, layer, cell, hunting::sound("tongue", id, super::KIND, body, hunting::HEAD));
}

/// The prey goes, with no death, and the frog's loot is left where the tongue met it.
fn swallow(world: &mut World, entity: Entity, layer: Entity, prey: Entity, spot: [f32; 3], now: u64) {
    legs::rest(world, entity);
    if let Some(mut meal) = world.get_mut::<Meal>(entity) {
        meal.eaten = Some(now);
    }
    let Some(kind) = world.get::<Kind>(prey).map(|kind| kind.0.clone()).filter(|_| super::super::is_alive(world, prey)) else { return };
    mobs::remove(world, prey);
    let variant = world.get::<Gene>(entity).map_or(0, |gene| gene.0);
    let loot = match kind.as_str() {
        "minecraft:slime" => "minecraft:slime_ball",
        _ => FROGLIGHTS.get(variant as usize).copied().unwrap_or(FROGLIGHTS[0]),
    };
    if !super::super::leaves_loot(world, layer) {
        return;
    }
    let Some(stack) = world.get::<Layer>(layer).and_then(|level| crate::inventory::stored(&Item::new(loot, 1), level.world.registry())) else { return };
    crate::drops::toss(world, layer, stack, [spot[0], spot[1] + LOOT_UP, spot[2]]);
}
