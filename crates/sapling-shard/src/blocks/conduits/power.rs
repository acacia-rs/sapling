//! What an active conduit does every 40 ticks, by the Java game's `ConduitBlockEntity`
//! (docs/research/effects-raids.md "Conduits"): conduit power to each player in water or rain
//! whose block is nearer than 16 for every 7 frame blocks; with all 42, 4 damage to one hostile
//! mob in water or rain within 8, kept as its `Target` until it dies or leaves.

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Effect, Weather};

use super::super::{Dice, send};
use crate::body::Body;
use crate::components::{InLayer, Layer, Position, in_world};
use crate::creatures::{Cause, Creature, Health};
use crate::entities::Kind;

const WATER: &[&str] = &["minecraft:water", "minecraft:flowing_water"];
const RANGE_PER: usize = 7;
const RANGE_STEP: i32 = 16;
const LASTS: u32 = 260;
const HUNTS_WITH: usize = 42;
const HUNTS: i32 = 8;
const BITES: f32 = 4.0;

/// The mob each hunting conduit has in its sights.
#[derive(Resource, Default)]
struct Prey(HashMap<(Entity, BlockPos), Entity>);

fn is_wet(world: &World, layer: Entity, feet: [f32; 3]) -> bool {
    let raining = crate::weather::of(world, layer) != Weather::Clear;
    raining || world.get::<Layer>(layer).is_some_and(|level| crate::creatures::holds(&level.world, feet.map(|c| c.floor() as i32), WATER))
}

/// Whether the block `feet` stand in is nearer `pos` than `reach`, as Java's `closerThan`.
fn is_near(pos: BlockPos, feet: [f32; 3], reach: i32) -> bool {
    let cell = feet.map(|c| c.floor() as i32);
    (0..3).map(|axis| i64::from(cell[axis] - pos[axis]).pow(2)).sum::<i64>() < i64::from(reach).pow(2)
}

pub(super) fn empower(world: &mut World, layer: Entity, pos: BlockPos, frame: usize) {
    let reach = (frame / RANGE_PER) as i32 * RANGE_STEP;
    let players: Vec<Entity> = in_world(world)
        .into_iter()
        .filter(|player| world.get::<InLayer>(*player).is_some_and(|on| on.0 == layer))
        .filter(|player| world.get::<Position>(*player).is_some_and(|at| is_near(pos, at.feet, reach) && is_wet(world, layer, at.feet)))
        .collect();
    for player in players {
        crate::vitals::give_ambient(world, player, Effect::ConduitPower, LASTS, 0);
    }
}

/// Live hostile mobs in water or rain whose box's foot is within the box 8 round `pos`.
fn hostiles(world: &mut World, layer: Entity, pos: BlockPos) -> Vec<(Entity, [f32; 3])> {
    let within = |at: [f32; 3]| (0..3).all(|axis| at[axis] >= (pos[axis] - HUNTS) as f32 && at[axis] <= (pos[axis] + 1 + HUNTS) as f32);
    let found: Vec<(Entity, [f32; 3])> = world
        .query_filtered::<(Entity, &InLayer, &Kind, &Body, &Health), With<Creature>>()
        .iter(world)
        .filter(|(_, on, kind, body, health)| on.0 == layer && health.left() > 0.0 && within(body.at) && sapling_data::has_family(&kind.0, "monster"))
        .map(|(mob, _, _, body, _)| (mob, body.at))
        .collect();
    found.into_iter().filter(|(_, at)| is_wet(world, layer, *at)).collect()
}

fn alive_at(world: &World, mob: Entity, layer: Entity) -> Option<[f32; 3]> {
    let on = world.get::<InLayer>(mob)?.0 == layer && world.get::<Health>(mob)?.left() > 0.0;
    on.then(|| world.get::<Body>(mob).map(|body| body.at)).flatten()
}

pub(super) fn hunt(world: &mut World, layer: Entity, pos: BlockPos, frame: usize) {
    let held = world.get_resource_or_insert_with(Prey::default).0.get(&(layer, pos)).copied();
    let target = if frame < HUNTS_WITH {
        None
    } else if let Some(mob) = held {
        alive_at(world, mob, layer).filter(|at| is_near(pos, *at, HUNTS)).map(|_| mob)
    } else {
        let found = hostiles(world, layer, pos);
        (!found.is_empty()).then(|| found[world.resource_mut::<Dice>().roll(found.len() as u32) as usize].0)
    };
    if target != held {
        let mut prey = world.resource_mut::<Prey>();
        match target {
            Some(mob) => prey.0.insert((layer, pos), mob),
            None => prey.0.remove(&(layer, pos)),
        };
        show_target(world, layer, pos, target);
    }
    if let Some(mob) = target {
        crate::creatures::wound(world, mob, layer, BITES, Cause::Magic);
    }
}

/// Keeps the conduit's `Target`, the mob's unique id or -1, and shows it.
fn show_target(world: &mut World, layer: Entity, pos: BlockPos, target: Option<Entity>) {
    let id = target.and_then(|mob| world.get::<crate::tags::UniqueId>(mob)).map_or(-1, |id| id.0);
    let Some(mut nbt) = world.get::<Layer>(layer).and_then(|level| level.world.block_entity(pos)).map(|entity| entity.nbt.clone()) else { return };
    if let Value::Compound(fields) = &mut nbt {
        fields.retain(|(name, _)| name != "Target");
        fields.push(("Target".into(), Value::Long(id)));
        fields.sort_by(|a, b| a.0.cmp(&b.0));
    }
    send::keep_entity(world, layer, pos, &nbt);
}
