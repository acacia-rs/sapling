//! What every entity the shard moves has, whatever it is: a [`Kind`], a [`Body`] and what its
//! viewers were last [`Told`]. Finding, listing, moving and removing any of them, and keeping
//! each filed under the column it is in (mobs.rs). Each kind does the rest itself: creatures.rs,
//! drops.rs, orbs.rs, blocks/falling.
// TODO: the mobs a world folder stores are still statues (mobs.rs), outside this.

use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{EntityId, EntityInfo};

use crate::body::{Body, Physics};
use crate::components::{InLayer, Layer, NetId};
use crate::mobs;
use crate::sync::Told;

/// What an entity is, namespaced: `minecraft:pig`, `minecraft:item` and so on.
#[derive(Component, Clone, Debug)]
pub(crate) struct Kind(pub String);

/// Puts an entity of `kind` with `body` into the world, shown by what `announce` makes of its
/// id; viewers start out knowing it at `wire`, where the wire puts it.
pub(crate) fn spawn(world: &mut World, layer: Entity, kind: &str, body: Body, wire: [f32; 3], announce: impl FnOnce(u64) -> Option<Vec<Bytes>>) -> Option<Entity> {
    let entity = mobs::spawn_shown(world, layer, body.at, announce)?;
    world.entity_mut(entity).insert((Kind(kind.to_owned()), body, Told::new(wire)));
    Some(entity)
}

/// Each entity with a body: its ECS entity, world and what the API is told of it.
fn all(world: &World) -> impl Iterator<Item = (Entity, Entity, EntityInfo)> + '_ {
    world.iter_entities().filter_map(|entity| {
        let (id, layer, kind, body) = (entity.get::<NetId>()?, entity.get::<InLayer>()?, entity.get::<Kind>()?, entity.get::<Body>()?);
        let info = EntityInfo { id: EntityId(id.0), kind: kind.0.clone(), feet: body.at, velocity: body.motion, on_ground: body.on_ground };
        Some((entity.id(), layer.0, info))
    })
}

/// The entity whose id is `id`.
pub(crate) fn find(world: &World, id: EntityId) -> Option<Entity> {
    all(world).find(|(.., info)| info.id == id).map(|(entity, ..)| entity)
}

pub(crate) fn describe(world: &World, id: EntityId) -> Option<EntityInfo> {
    all(world).find(|(.., info)| info.id == id).map(|(.., info)| info)
}

/// Every entity of `layer`, oldest first.
pub(crate) fn listed(world: &World, layer: Entity) -> Vec<EntityInfo> {
    let mut found: Vec<EntityInfo> = all(world).filter(|(_, in_layer, _)| *in_layer == layer).map(|(.., info)| info).collect();
    found.sort_unstable_by_key(|info| info.id.0);
    found
}

/// Moves the entity a tick by `physics`; its body now, and what tells its viewers, whose wire
/// puts it `lift` above the bottom of its box.
pub(crate) fn step(world: &mut World, entity: Entity, layer: Entity, id: u64, physics: &Physics, lift: f32) -> Option<(Body, Vec<Bytes>)> {
    let mut body = *world.get::<Body>(entity)?;
    body.step(&world.get::<Layer>(layer)?.world, physics);
    let [x, y, z] = body.at;
    let packets = world.get_mut::<Told>(entity)?.update(id, [x, y + lift, z], body.motion, body.on_ground);
    *world.get_mut::<Body>(entity)? = body;
    Some((body, packets))
}

/// Every entity's world, kind and body.
pub(crate) fn bodies(world: &World) -> Vec<(Entity, String, Body)> {
    world.iter_entities().filter_map(|entity| Some((entity.get::<InLayer>()?.0, entity.get::<Kind>()?.0.clone(), *entity.get::<Body>()?))).collect()
}

/// Changes the body of the entity `id`; false when it is gone.
pub(crate) fn change(world: &mut World, id: EntityId, change: impl FnOnce(&mut Body)) -> bool {
    let Some(entity) = find(world, id) else { return false };
    world.get_mut::<Body>(entity).map(|mut body| change(&mut body)).is_some()
}

pub(crate) fn remove(world: &mut World, id: EntityId) -> bool {
    let Some(entity) = find(world, id) else { return false };
    mobs::remove(world, entity);
    true
}

/// Files every entity under the column it is in now.
pub(crate) fn follow(world: &mut World) {
    let all: Vec<(Entity, [f32; 3])> = world.query::<(Entity, &Body)>().iter(world).map(|(entity, body)| (entity, body.at)).collect();
    for (entity, at) in all {
        mobs::follow(world, entity, at);
    }
}
