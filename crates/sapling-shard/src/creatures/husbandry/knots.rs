//! Leash knots: a player's click on a fence ties the mobs on its leads near to a knot there, 0.25
//! above the fence's floor. A click on the knot hands its mobs to the clicker and takes it away;
//! a hit or its fence going lets them go. A knot holding nothing goes at once; one a world folder kept
//! waits [`EMPTY`] ticks for its mobs to come back (research/captures/bds-uuknot.txt).

use acacia_proto::types::MetadataFlags1;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_data::AddEntityOptions;

use super::{Raised, distance, leads};
use crate::body::Body;
use crate::components::{InLayer, Layer, Position, Tick};
use crate::{entities, mobs};

pub(crate) const KNOT: &str = "minecraft:leash_knot";
/// How far above its fence's floor a knot's feet are.
const RAISED: f32 = 0.25;
const SIZE: [f32; 2] = [0.375, 0.5];
/// How long a kept knot holding nothing stays, which the mobs it holds have to come back from storage in.
const EMPTY: u64 = 100;

/// A knot, the fence it is tied to and the tick until which it stays holding nothing.
#[derive(Component)]
pub(super) struct Knot(BlockPos, u64);

fn is_fence(world: &World, layer: Entity, [x, y, z]: BlockPos) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    level.registry().get(level.block(x, y, z)).is_some_and(|state| state.name.ends_with("fence"))
}

fn shown(world: &mut World, layer: Entity, pos: BlockPos, grace: u64) -> Option<Entity> {
    let feet = [pos[0] as f32 + 0.5, pos[1] as f32 + RAISED, pos[2] as f32 + 0.5];
    let body = Body::new(feet, [0.0; 3], sapling_data::mob_size(KNOT).unwrap_or(SIZE));
    let announce = |id| {
        let options = AddEntityOptions { id, identifier: KNOT, feet, yaw: 0.0, pitch: 0.0, variant: 0, mark_variant: 0, color: 0, baby: false, flags: MetadataFlags1(0), health: None, name: "" };
        Some(vec![sapling_net::encode(&sapling_data::add_entity(&options))])
    };
    let entity = entities::spawn(world, layer, KNOT, body, feet, announce)?;
    let now = world.resource::<Tick>().0;
    world.entity_mut(entity).insert(Knot(pos, now + grace));
    Some(entity)
}

/// A knot a world folder kept stands again, on the fence under `feet`.
pub(crate) fn from_storage(world: &mut World, layer: Entity, feet: [f32; 3], unique_id: i64, compound: Vec<u8>) -> bool {
    let Some(entity) = shown(world, layer, feet.map(|c| c.floor() as i32), EMPTY) else { return false };
    entities::keep(world, entity, layer, unique_id, compound);
    true
}

/// The knot on the fence at `pos` in `layer`, if there is one.
fn knot_at(world: &mut World, layer: Entity, pos: BlockPos) -> Option<Entity> {
    world.query::<(Entity, &Knot, &InLayer)>().iter(world).find(|(_, knot, in_layer)| knot.0 == pos && in_layer.0 == layer).map(|(entity, ..)| entity)
}

/// The mobs on leads `holder` holds.
fn held_by(world: &mut World, holder: Entity) -> Vec<Entity> {
    world.query::<(Entity, &Raised)>().iter(world).filter(|(_, raised)| raised.holder == Some(holder)).map(|(entity, _)| entity).collect()
}

/// A player's click on the block `name` at `pos`: a fence takes the mobs on its leads near, tied
/// to its knot. False where it is no fence or the player holds none.
pub(crate) fn tie_to_fence(world: &mut World, player: Entity, layer: Entity, pos: BlockPos, name: &str) -> bool {
    if !name.ends_with("fence") {
        return false;
    }
    let Some(feet) = world.get::<Position>(player).map(|position| position.feet) else { return false };
    let held: Vec<Entity> = held_by(world, player).into_iter().filter(|mob| world.get::<Body>(*mob).is_some_and(|body| distance(body.at, feet) <= leads::BREAKS)).collect();
    if held.is_empty() {
        return false;
    }
    let Some(knot) = knot_at(world, layer, pos).or_else(|| new_knot(world, layer, pos)) else { return false };
    for mob in held {
        leads::tie(world, mob, knot);
    }
    super::shown::sound_unattributed(world, knot, "leashknot.place", 0.5);
    true
}

fn new_knot(world: &mut World, layer: Entity, pos: BlockPos) -> Option<Entity> {
    let entity = shown(world, layer, pos, 0)?;
    let unique_id = entities::new_unique_id(world, entity)?;
    let feet = world.get::<Body>(entity)?.at;
    entities::keep(world, entity, layer, unique_id, sapling_world::new_actor(unique_id, KNOT, feet, [0.0; 3]));
    Some(entity)
}

/// The player's click on `entity`: a knot hands its mobs to the player, heard `lead.leash` from
/// it, and goes with the knots' next tick, as BDS's did (bds-uuknot.txt). False for anything else.
pub(crate) fn undo(world: &mut World, player: Entity, entity: Entity) -> bool {
    let Some(mut knot) = world.get_mut::<Knot>(entity) else { return false };
    knot.1 = 0;
    let held = held_by(world, entity);
    for mob in &held {
        leads::tie(world, *mob, player);
    }
    if let (Some(mob), Some(feet)) = (held.first(), world.get::<Position>(player).map(|position| position.feet)) {
        leads::heard_from_holder(world, *mob, "lead.leash", feet);
    }
    true
}

/// A hit at `entity`: a knot lets its mobs go and goes. False for anything else.
pub(crate) fn knock_down(world: &mut World, entity: Entity) -> bool {
    if world.get::<Knot>(entity).is_none() {
        return false;
    }
    take_away(world, entity);
    true
}

fn take_away(world: &mut World, knot: Entity) {
    for mob in held_by(world, knot) {
        leads::untie(world, mob, "lead.unleash");
    }
    super::shown::sound_unattributed(world, knot, "leashknot.break", 0.0);
    mobs::remove(world, knot);
}

/// Knots whose fence went, or that hold nothing past their grace, go.
pub(super) fn tick(world: &mut World) {
    let knots: Vec<(Entity, Entity, BlockPos, u64)> = world.query::<(Entity, &Knot, &InLayer)>().iter(world).map(|(entity, knot, layer)| (entity, layer.0, knot.0, knot.1)).collect();
    if knots.is_empty() {
        return;
    }
    let leashed: Vec<i64> = world.query::<&Raised>().iter(world).filter_map(|raised| raised.kept.leasher).collect();
    let now = world.resource::<Tick>().0;
    for (knot, layer, pos, until) in knots {
        let holds = now < until || leads::unique_id_of(world, knot).is_some_and(|id| leashed.contains(&id));
        if !holds || !is_fence(world, layer, pos) {
            take_away(world, knot);
        }
    }
}
