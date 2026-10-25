//! What every entity the shard moves has, whatever it is: a [`Kind`], a [`Body`] and what its
//! viewers were last [`Told`]. Finding, listing, moving and removing any of them, and keeping
//! each filed under the column it is in (mobs.rs). Each kind does the rest itself: creatures.rs,
//! drops.rs, orbs.rs, blocks/falling.
// TODO: the mobs a world folder stores are still statues (mobs.rs), outside this.

use acacia_proto::RawPacket;
use acacia_proto::packets::AddEntity;
use acacia_proto::types::{MetadataDictionaryItemValue, MetadataFlags1};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_api::{EntityId, EntityInfo};

use crate::blocks::Dice;
use crate::body::{Body, InLiquid, Liquid, Physics};
use crate::components::{InLayer, Layer, NetId, Tick};
use crate::drops::Dropped;
use crate::inventory::described;
use crate::sync::Told;
use crate::{mobs, splash, view};

/// What an entity is, namespaced: `minecraft:pig`, `minecraft:item` and so on.
#[derive(Component, Clone, Debug)]
pub(crate) struct Kind(pub String);

/// What a world folder keeps of an entity: its actor compound, written back with its place and
/// motion when its column is saved (actors.rs).
#[derive(Component, Clone, Debug)]
pub(crate) struct Stored {
    pub compound: Vec<u8>,
}

/// The stored actors (by world and `UniqueID`) that the shard moves as entities: their stored
/// compounds give way to what the entities are now, or to nothing once they are gone.
#[derive(Resource, Default)]
pub(crate) struct Managed(pub std::collections::HashSet<(Entity, i64)>);

/// The `UniqueID` a world folder keeps a new entity under, numbered as BDS numbers actors: a high
/// word that differs, a counter below.
pub(crate) fn new_unique_id(world: &World, entity: Entity) -> Option<i64> {
    Some(((world.resource::<Tick>().0 as i64) << 32) | (world.get::<NetId>(entity)?.0 as i64 & 0xffff_ffff))
}

/// Makes the entity one a world folder keeps.
pub(crate) fn keep(world: &mut World, entity: Entity, layer: Entity, unique_id: i64, compound: Vec<u8>) {
    world.get_resource_or_insert_with(Managed::default).0.insert((layer, unique_id));
    world.entity_mut(entity).insert(Stored { compound });
}

/// The tick an entity appeared in: its viewers are told of it after the tick's handlers, so a
/// move in that tick would reach them before the entity does.
#[derive(Component)]
struct Born(u64);

/// Moves of entities that appeared this tick, for their viewers once they are told of them.
#[derive(Resource, Default)]
pub(crate) struct AfterAnnouncing(Vec<(Entity, [i32; 3], Vec<Bytes>)>);

/// An entity taken by a player: it goes from screens the tick after, as BDS sends an item's or
/// an orb's RemoveEntity (research/captures/containers-findings.md D3, D6).
#[derive(Component)]
struct Leaving(u64);

/// Has the entity go the tick after this one; call once nothing moves or takes it any more.
pub(crate) fn leave(world: &mut World, entity: Entity) {
    let next = world.resource::<Tick>().0 + 1;
    world.entity_mut(entity).insert(Leaving(next));
}

/// Takes away the entities whose time to go has come.
pub(crate) fn remove_left(world: &mut World) {
    let now = world.resource::<Tick>().0;
    let due: Vec<Entity> = world.query::<(Entity, &Leaving)>().iter(world).filter(|(_, leaving)| leaving.0 <= now).map(|(entity, _)| entity).collect();
    for entity in due {
        mobs::remove(world, entity);
    }
}

/// Has `packets` sent to the viewers of `cell` once this tick's new entities are announced to them.
pub(crate) fn after_announcing(world: &mut World, layer: Entity, cell: [i32; 3], packets: Vec<Bytes>) {
    world.get_resource_or_insert_with(AfterAnnouncing::default).0.push((layer, cell, packets));
}

/// Sends the moves held back by [`step`]; runs after the tick's announcements (mobs.rs).
pub(crate) fn send_held_moves(world: &mut World) {
    let Some(held) = world.get_resource_mut::<AfterAnnouncing>().map(|mut held| std::mem::take(&mut held.0)) else { return };
    for (layer, cell, packets) in held {
        view::to_viewers(world, layer, cell, &packets);
    }
}

/// Puts an entity of `kind` with `body` into the world, shown by what `announce` makes of its
/// id; viewers start out knowing it at `wire`, where the wire puts it.
pub(crate) fn spawn(world: &mut World, layer: Entity, kind: &str, body: Body, wire: [f32; 3], announce: impl FnOnce(u64) -> Option<Vec<Bytes>>) -> Option<Entity> {
    let entity = mobs::spawn_shown(world, layer, body.at, announce)?;
    let tick = world.resource::<Tick>().0;
    world.entity_mut(entity).insert((Kind(kind.to_owned()), body, Told::new(wire), Born(tick)));
    Some(entity)
}

/// Has the entity's viewers told when it is moving (sync.rs [`Told::flag_moving`]).
pub(crate) fn flag_moving(world: &mut World, entity: Entity, ticked: bool) {
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.flag_moving(ticked);
    }
}

/// [`spawn`]s a mob whose viewers are told when it moves, from the flags its `AddEntity` (the
/// first packet `announce` makes) gave it.
pub(crate) fn spawn_mob(world: &mut World, layer: Entity, kind: &str, body: Body, wire: [f32; 3], announce: impl FnOnce(u64) -> Option<Vec<Bytes>>) -> Option<Entity> {
    let mut announced = None;
    let entity = spawn(world, layer, kind, body, wire, |id| {
        let packets = announce(id)?;
        announced = packets.first().and_then(|first| RawPacket::parse(first.clone()).ok()?.decode::<AddEntity>().ok()).and_then(|added| {
            added.metadata.into_iter().find_map(|item| if let MetadataDictionaryItemValue::Flags(flags) = item.value { Some(flags) } else { None })
        });
        Some(packets)
    })?;
    if let (Some(announced), Some(mut told)) = (announced, world.get_mut::<Told>(entity)) {
        told.flag_mob(announced);
    }
    Some(entity)
}

/// Sets or clears `flag` of the mob `entity` of `kind`: the whole word of flags to tell its
/// viewers now, `MOVING` too, so that no writer clears another's.
pub(crate) fn mob_flags(world: &mut World, entity: Entity, kind: &str, flag: MetadataFlags1, on: bool) -> MetadataFlags1 {
    let tracked = world.get_mut::<Told>(entity).and_then(|mut told| told.set_flag(flag, on));
    tracked.unwrap_or_else(|| sapling_data::grown_flags(kind, if on { flag } else { MetadataFlags1(0) }))
}

/// Each entity with a body: its ECS entity, world and what the API is told of it.
fn all(world: &World) -> impl Iterator<Item = (Entity, Entity, EntityInfo)> + '_ {
    world.iter_entities().filter_map(|entity| {
        let (id, layer, kind, body) = (entity.get::<NetId>()?, entity.get::<InLayer>()?, entity.get::<Kind>()?, entity.get::<Body>()?);
        let item = entity.get::<Dropped>().map(|dropped| described(dropped.stack()));
        let mut info = EntityInfo::new(EntityId(id.0), kind.0.clone(), body.at, body.motion, body.on_ground).with_item(item);
        if let Some(told) = entity.get::<Told>() {
            [info.yaw, info.pitch] = told.facing();
        }
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
/// puts it `lift` above the bottom of its box. An item lava burns, and an orb it throws out, fizz.
pub(crate) fn step(world: &mut World, entity: Entity, layer: Entity, id: u64, physics: &Physics, lift: f32) -> Option<(Body, Vec<Bytes>)> {
    let mut body = *world.get::<Body>(entity)?;
    let start = body;
    world.get_resource_or_insert_with(Dice::default);
    let moved = world.resource_scope(|world, mut dice: Mut<Dice>| Some(body.step(&world.get::<Layer>(layer)?.world, physics, &mut || dice.fraction())))?;
    *world.get_mut::<Body>(entity)? = body;
    let kind = world.get::<Kind>(entity)?.0.clone();
    let splash = splash::check(world, entity, id, &kind, moved, &body, physics.liquid);
    let in_lava = body.liquid() == Some(Liquid::Lava);
    if in_lava && (physics.burns || physics.liquid == InLiquid::Buoyant) {
        splash::fizz(world, layer, &start, 0);
    }
    let lifted = physics.liquid == InLiquid::Floats && body.liquid().is_some();
    let [x, y, z] = body.at;
    let mut told = world.get_mut::<Told>(entity)?;
    let flags = told.flags(id, std::array::from_fn(|axis| body.at[axis] - start.at[axis]), lifted, physics.burns && in_lava);
    let packets: Vec<Bytes> = flags.into_iter().chain(told.update(id, [x, y + lift, z], body.motion, body.on_ground)).chain(splash).collect();
    // Not yet announced: this move follows the announcement, in the same batch, as on BDS.
    if is_new(world, entity) {
        after_announcing(world, layer, body.at.map(|c| c.floor() as i32), packets);
        return Some((body, Vec::new()));
    }
    Some((body, packets))
}

fn is_new(world: &World, entity: Entity) -> bool {
    world.get::<Born>(entity).is_some_and(|born| born.0 == world.resource::<Tick>().0)
}

/// Sends `packets` about the entity to the viewers of `cell`, after its announcement when it
/// appeared this tick.
pub(crate) fn tell_viewers(world: &mut World, entity: Entity, layer: Entity, cell: [i32; 3], packets: Vec<Bytes>) {
    if packets.is_empty() {
        return;
    }
    if is_new(world, entity) {
        after_announcing(world, layer, cell, packets);
    } else {
        view::to_viewers(world, layer, cell, &packets);
    }
}

/// The world, kind and body of every entity that has had a tick of its own: on BDS one appearing
/// on a plate presses it the tick after (research/captures/bds-plates.txt).
pub(crate) fn bodies(world: &World) -> Vec<(Entity, String, Body)> {
    let now = world.resource::<Tick>().0;
    let ticked = |entity: &bevy_ecs::world::EntityRef<'_>| entity.get::<Born>().is_none_or(|born| born.0 < now);
    world.iter_entities().filter(ticked).filter_map(|entity| Some((entity.get::<InLayer>()?.0, entity.get::<Kind>()?.0.clone(), *entity.get::<Body>()?))).collect()
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
