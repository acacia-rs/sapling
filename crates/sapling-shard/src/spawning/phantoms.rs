//! Insomnia and where phantoms appear: BDS counts each player's ticks since it last slept
//! (`InsomniaSystem`, `TimeSinceRest`), and a phantom the spawner puts by its pack rule is moved
//! above a player who has gone without (bds-blocks `1429ab450`; docs/research/mob-spawning.md
//! "Phantoms").

use bevy_ecs::prelude::*;
use sapling_api::GameMode;

use super::SpawnDice;
use super::spots::Found;
use crate::body::Body;
use crate::components::{InLayer, Layer, Mode, Position, in_world};
use crate::creatures::Creature;
use crate::entities::Kind;
use crate::sleep::Sleeping;

pub(super) const PHANTOM: &str = "minecraft:phantom";
const DO_INSOMNIA: &str = "doInsomnia";
/// The player component's `days_until_insomnia` (3) in ticks.
const SLEEPLESS: u32 = 72_000;
/// How near (squared) the spot the spawner found a player must be.
const NEAR: f32 = 10_000.0;
/// How far round the phantom's new place, on each axis, other phantoms count, and the most there.
const CROWD: f32 = 48.0;
const MOST: usize = 5;

/// A player's ticks since it last slept, saved as `TimeSinceRest`.
#[derive(Component, Clone, Copy, Default)]
pub(crate) struct Rested(pub i32);

/// BDS's insomnia tick: a sleeper's count goes back to 1, everyone else's rises while
/// `doInsomnia` is on and goes back to 1 while it is off; spectators' stays.
pub(super) fn count_sleepless(world: &mut World) {
    for player in in_world(world) {
        if world.get::<Mode>(player).is_some_and(|mode| mode.0 == GameMode::Spectator) {
            continue;
        }
        let sleepless = world.get::<Sleeping>(player).is_none() && world.get::<InLayer>(player).is_some_and(|layer| crate::rules::on(world, layer.0, DO_INSOMNIA));
        let rested = world.get::<Rested>(player).copied().unwrap_or_default();
        world.entity_mut(player).insert(Rested(if sleepless { rested.0.saturating_add(1) } else { 1 }));
    }
}

/// Where a phantom the spawner found `found` for appears: 20 to 34 above, and within 10 across, the
/// first player (in join order) within 100 of the spot with open sky over it whose sleeplessness
/// passes a roll, unless 5 phantoms are about that place already; `None` when no player takes it.
pub(super) fn place(world: &mut World, layer: Entity, found: &Found) -> Option<Found> {
    let spot = found.feet();
    for player in in_world(world) {
        let (Some(in_layer), Some(position), Some(rested)) = (world.get::<InLayer>(player), world.get::<Position>(player), world.get::<Rested>(player)) else { continue };
        let (feet, rested) = (position.feet, rested.0);
        if in_layer.0 != layer || (0..3).map(|axis| (feet[axis] - spot[axis]).powi(2)).sum::<f32>() >= NEAR || !under_sky(world, layer, feet) {
            continue;
        }
        let dice = &mut world.get_resource_or_insert_with(SpawnDice::default).0;
        if rested <= 0 || dice.roll(rested as u32) < SLEEPLESS {
            continue;
        }
        let [x, y, z] = [dice.roll(21) as f32 - 10.0, dice.roll(15) as f32 + 20.0, dice.roll(21) as f32 - 10.0];
        let cell = [feet[0] + x, feet[1] + y, feet[2] + z].map(|c| c.floor() as i32);
        if crowd(world, layer, cell.map(|c| c as f32)) < MOST {
            return Some(Found { cell, ..*found });
        }
    }
    None
}

fn under_sky(world: &World, layer: Entity, [x, y, z]: [f32; 3]) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    level.top_solid(x.floor() as i32, z.floor() as i32).is_none_or(|top| y.floor() as i32 > top)
}

fn crowd(world: &mut World, layer: Entity, at: [f32; 3]) -> usize {
    let mut phantoms = world.query_filtered::<(&InLayer, &Kind, &Body), With<Creature>>();
    phantoms.iter(world).filter(|(in_layer, kind, body)| in_layer.0 == layer && kind.0 == PHANTOM && (0..3).all(|axis| (body.at[axis] - at[axis]).abs() <= CROWD)).count()
}
