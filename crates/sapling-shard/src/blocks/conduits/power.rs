//! What an active conduit does every 40 ticks (docs/research/effects-raids.md "Conduit"): conduit
//! power, not ambient, to each player in water or rain whose block is less than 16 for every 7
//! frame blocks off along each axis (research/captures/bds-conduit2.txt); with all 42, one hostile
//! mob in water or rain whose box meets the box 5 round the conduit is kept as its `Target` until
//! it dies or leaves that box, and bitten for 4 from the pulse after it was taken,
//! `conduit.attack` heard at the conduit (docs/research/effects-raids.md "Conduit" for the reach).

use std::collections::HashMap;

use acacia_proto::nbt::Value;
use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Effect};

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
const HUNTS: f32 = 5.0;
const BITES: f32 = 4.0;
const PLAYER_HEIGHT: f32 = 1.8;

/// The mob each hunting conduit has in its sights.
#[derive(Resource, Default)]
struct Prey(HashMap<(Entity, BlockPos), Entity>);

fn is_wet(world: &World, layer: Entity, feet: [f32; 3], height: f32) -> bool {
    crate::creatures::in_rain(world, layer, feet, height) || world.get::<Layer>(layer).is_some_and(|level| crate::creatures::holds(&level.world, feet.map(|c| c.floor() as i32), WATER))
}

/// Whether `body`'s box meets the box [`HUNTS`] round the block at `pos`.
fn is_hunted_near(pos: BlockPos, body: &Body) -> bool {
    let (low, high) = body.bounds();
    (0..3).all(|axis| high[axis] > pos[axis] as f32 - HUNTS && low[axis] < (pos[axis] + 1) as f32 + HUNTS)
}

/// Whether the block `feet` stand in is less than `reach` off `pos` along every axis.
fn is_within(pos: BlockPos, feet: [f32; 3], reach: i32) -> bool {
    let cell = feet.map(|c| c.floor() as i32);
    (0..3).all(|axis| (cell[axis] - pos[axis]).abs() < reach)
}

pub(super) fn empower(world: &mut World, layer: Entity, pos: BlockPos, frame: usize) {
    let reach = (frame / RANGE_PER) as i32 * RANGE_STEP;
    let players: Vec<Entity> = in_world(world)
        .into_iter()
        .filter(|player| world.get::<InLayer>(*player).is_some_and(|on| on.0 == layer))
        .filter(|player| world.get::<Position>(*player).is_some_and(|at| is_within(pos, at.feet, reach) && is_wet(world, layer, at.feet, PLAYER_HEIGHT)))
        .collect();
    for player in players {
        crate::vitals::give_effect(world, player, Effect::ConduitPower, LASTS, 0, true);
    }
}

/// Live hostile mobs in water or rain within the hunting box round `pos`.
fn hostiles(world: &mut World, layer: Entity, pos: BlockPos) -> Vec<(Entity, [f32; 3])> {
    let found: Vec<(Entity, [f32; 3], f32)> = world
        .query_filtered::<(Entity, &InLayer, &Kind, &Body, &Health), With<Creature>>()
        .iter(world)
        .filter(|(_, on, kind, body, health)| on.0 == layer && health.left() > 0.0 && is_hunted_near(pos, body) && sapling_data::has_family(&kind.0, "monster"))
        .map(|(mob, _, _, body, _)| (mob, body.at, body.size[1]))
        .collect();
    found.into_iter().filter(|(_, at, height)| is_wet(world, layer, *at, *height)).map(|(mob, at, _)| (mob, at)).collect()
}

fn is_alive_near(world: &World, mob: Entity, layer: Entity, pos: BlockPos) -> bool {
    world.get::<InLayer>(mob).is_some_and(|on| on.0 == layer) && world.get::<Health>(mob).is_some_and(|health| health.left() > 0.0) && world.get::<Body>(mob).is_some_and(|body| is_hunted_near(pos, body))
}

pub(super) fn hunt(world: &mut World, layer: Entity, pos: BlockPos, frame: usize) {
    let held = world.get_resource_or_insert_with(Prey::default).0.get(&(layer, pos)).copied();
    let target = if frame < HUNTS_WITH {
        None
    } else if let Some(mob) = held {
        is_alive_near(world, mob, layer, pos).then_some(mob)
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
        // BDS told nobody of the -1 after its prey died (bds-e1conduit.txt).
        let died = held.is_some_and(|mob| world.get::<Health>(mob).is_none_or(|health| health.left() <= 0.0));
        keep_target(world, layer, pos, target, !died);
    }
    if let Some(mob) = target.filter(|mob| held == Some(*mob)) {
        crate::creatures::wound(world, mob, layer, BITES, Cause::Magic);
        bite_sound(world, layer, pos);
    }
}

/// `conduit.attack` at the conduit, heard the tick after its bite as BDS's (bds-e1conduit.txt).
fn bite_sound(world: &mut World, layer: Entity, pos: BlockPos) {
    let position = Vec3f { x: pos[0] as f32, y: pos[1] as f32, z: pos[2] as f32 };
    let sound = LevelSoundEvent { sound_id: "conduit.attack".into(), position, extra_data: -1, entity_type: String::new(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
    crate::late_sounds::play_later(world, 1, layer, pos, sapling_net::encode(&sound));
}

/// Keeps the conduit's `Target`, the mob's unique id or -1, and shows it if `shown`.
fn keep_target(world: &mut World, layer: Entity, pos: BlockPos, target: Option<Entity>, shown: bool) {
    let id = target.and_then(|mob| world.get::<crate::tags::UniqueId>(mob)).map_or(-1, |id| id.0);
    let Some(mut nbt) = world.get::<Layer>(layer).and_then(|level| level.world.block_entity(pos)).map(|entity| entity.nbt.clone()) else { return };
    if let Value::Compound(fields) = &mut nbt {
        fields.retain(|(name, _)| name != "Target");
        fields.push(("Target".into(), Value::Long(id)));
        fields.sort_by(|a, b| a.0.cmp(&b.0));
    }
    if shown {
        send::keep_entity(world, layer, pos, &nbt);
    } else if let Some(mut level) = world.get_mut::<Layer>(layer) {
        level.world.set_block_entity(pos, &nbt);
    }
}
