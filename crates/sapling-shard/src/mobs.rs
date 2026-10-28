//! Entities shown to players: each is filed under the column it is in and announced to whoever
//! sees that column, and removed from their screens when it goes or leaves their view. Stored
//! actors the shard does not move (paintings, kinds without a recorded box) are statues here;
//! the rest are entities (entities.rs, actors.rs).

use std::collections::HashMap;

use acacia_proto::packets::{AddItemEntity, RemoveEntity, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataFlags1};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_data::{AddEntityOptions, Stack};
use sapling_world::{Actor, ActorState, ChunkPos, ItemStack};

use crate::chunks::UNLOAD_INTERVAL;
use crate::components::{InLayer, Layer, NetId, Outgoing, Players, Stage, Tick};
use crate::grid::Grid;
use crate::view::View;

const ITEM: &str = "minecraft:item";

/// A stored actor, in the column it was stored with.
#[derive(Component)]
pub(crate) struct Mob {
    /// What tells a client of it: the entity, then what it wears and holds.
    announcement: Vec<Bytes>,
    /// Told after every mob a client is shown that tick: what refers to others (its riders).
    after: Vec<Bytes>,
    home: ChunkPos,
}

fn stack(stored: &ItemStack) -> Stack<'_> {
    Stack { name: &stored.name, count: stored.count, damage: stored.damage, block: stored.block, tag: stored.tag.as_ref() }
}

/// The mobs this player's client has as entities, with the ids it knows them by.
#[derive(Component, Default)]
pub(crate) struct SeesMobs {
    mobs: HashMap<Entity, u64>,
    /// The view's moves and the mobs' changes the set was last made for.
    made_for: Option<(u64, u64)>,
}

impl SeesMobs {
    /// The ids of the mobs the client has, emptying the set: they are about to be removed.
    pub fn take_ids(&mut self) -> Vec<u64> {
        self.made_for = None;
        self.mobs.drain().map(|(_, id)| id).collect()
    }
}

/// Every mob with its id, by the column it is in.
#[derive(Resource, Default)]
pub(crate) struct MobGrid {
    grid: Grid<(Entity, u64)>,
    /// Counts the ticks that added or removed mobs.
    changes: u64,
}

/// What shows a stored actor as it was stored; `None` for a dropped item that names no item a
/// client knows.
pub(crate) fn announcement(actor: &Actor, id: u64) -> Option<Vec<Bytes>> {
    announcement_with(actor, id, MetadataFlags1(0), None)
}

/// [`announcement`] with `extra` flags on top of those its stored states show, and its entity
/// `properties` where they are known.
pub(crate) fn announcement_with(actor: &Actor, id: u64, extra: MetadataFlags1, properties: Option<EntityProperties>) -> Option<Vec<Bytes>> {
    if actor.identifier == ITEM {
        return Some(item_shown(sapling_data::add_item_entity(id, &stack(actor.item.as_ref()?), actor.position, [0.0; 3])?));
    }
    if let Some(painting) = &actor.painting {
        return Some(vec![sapling_net::encode(&sapling_data::add_painting(id, actor.position, painting.direction.into(), &painting.motif))]);
    }
    let flag = |state: &ActorState| match state {
        ActorState::Saddled => MetadataFlags1::SADDLED,
        ActorState::Tamed => MetadataFlags1::TAMED,
        ActorState::Sheared => MetadataFlags1::SHEARED,
        ActorState::Chested => MetadataFlags1::CHESTED,
        ActorState::Sitting => MetadataFlags1::SITTING,
    };
    let mut added = sapling_data::add_entity(&AddEntityOptions {
        flags: MetadataFlags1(actor.states.iter().fold(extra.0, |flags, state| flags | flag(state).0)),
        health: actor.health,
        id,
        identifier: &actor.identifier,
        feet: actor.position,
        yaw: actor.yaw,
        pitch: actor.pitch,
        variant: actor.variant,
        mark_variant: actor.mark_variant,
        color: actor.color,
        baby: actor.baby,
        name: &actor.name,
    });
    if let Some(properties) = properties {
        added.properties = properties;
    }
    let mut packets = vec![sapling_net::encode(&added)];
    if actor.armor.iter().any(Option::is_some) {
        let slots: Vec<_> = actor.armor.iter().map(|slot| slot.as_ref().map(stack)).collect();
        packets.push(sapling_net::encode(&sapling_data::mob_armor(id, &slots)));
    }
    for (held, offhand) in [(&actor.mainhand, false), (&actor.offhand, true)] {
        packets.extend(held.as_ref().and_then(|held| sapling_data::mob_hand(id, &stack(held), offhand)).map(|held| sapling_net::encode(&held)));
    }
    Some(packets)
}

/// What shows a stack as an item at `position` moving at `velocity`; `None` for an item no
/// client knows.
pub(crate) fn item_announcement(id: u64, item: &ItemStack, position: [f32; 3], velocity: [f32; 3], fished: bool) -> Option<Vec<Bytes>> {
    let shown = sapling_data::add_item_entity(id, &stack(item), position, velocity)?;
    Some(item_shown(AddItemEntity { is_from_fishing: fished, ..shown }))
}

/// An item's announcement and its metadata again, as BDS follows one (findings D9, bds-kill.txt).
fn item_shown(item: AddItemEntity) -> Vec<Bytes> {
    let again = SetEntityData { runtime_entity_id: item.runtime_entity_id, metadata: item.metadata.clone(), properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 };
    vec![sapling_net::encode(&item), sapling_net::encode(&again)]
}

/// Puts an entity into the world at `feet`, shown to those who see its column by what
/// `announce` makes of its id; `None` when that makes nothing.
pub(crate) fn spawn_shown(world: &mut World, layer: Entity, feet: [f32; 3], announce: impl FnOnce(u64) -> Option<Vec<Bytes>>) -> Option<Entity> {
    let id = world.resource::<Players>().next_net_id;
    let announcement = announce(id)?;
    world.resource_mut::<Players>().next_net_id += 1;
    let home = ChunkPos::of_block(feet[0].floor() as i32, feet[2].floor() as i32);
    let mob = world.spawn((Mob { announcement, after: Vec::new(), home }, NetId(id), InLayer(layer))).id();
    let mut grid = world.resource_mut::<MobGrid>();
    grid.grid.insert(layer, home, (mob, id));
    grid.changes += 1;
    Some(mob)
}

/// What players who come to see the entity are told of it now.
pub(crate) fn announced(world: &World, mob: Entity) -> Option<Vec<Bytes>> {
    world.get::<Mob>(mob).map(|mob| mob.announcement.clone())
}

/// What players who come to see a moving entity later are told of it.
pub(crate) fn reannounce(world: &mut World, mob: Entity, announcement: Vec<Bytes>) {
    if let Some(mut mob) = world.get_mut::<Mob>(mob) {
        mob.announcement = announcement;
    }
}

/// What players who come to see the entity later are told once every entity they are shown with
/// it is announced.
pub(crate) fn show_after(world: &mut World, mob: Entity, after: Vec<Bytes>) {
    if let Some(mut mob) = world.get_mut::<Mob>(mob) {
        mob.after = after;
    }
}

/// Files the entity under the column of `at` when it moved into another: it is shown to those
/// who see that one, and unloaded with it.
pub(crate) fn follow(world: &mut World, mob: Entity, at: [f32; 3]) {
    let home = ChunkPos::of_block(at[0].floor() as i32, at[2].floor() as i32);
    let Some((old, id, layer)) = world.query::<(&Mob, &NetId, &InLayer)>().get(world, mob).ok().map(|(mob, id, layer)| (mob.home, id.0, layer.0)) else { return };
    if old == home {
        return;
    }
    if let Some(mut shown) = world.get_mut::<Mob>(mob) {
        shown.home = home;
    }
    let mut grid = world.resource_mut::<MobGrid>();
    grid.grid.remove(layer, old, &(mob, id));
    grid.grid.insert(layer, home, (mob, id));
    grid.changes += 1;
}

/// Takes a mob out of the world, off what it rode and from under its riders; those who saw it
/// are told by [`update`].
pub(crate) fn remove(world: &mut World, mob: Entity) {
    crate::riding::forget(world, mob);
    let Some((home, id, layer)) = world.query::<(&Mob, &NetId, &InLayer)>().get(world, mob).ok().map(|(mob, id, layer)| (mob.home, id.0, layer.0)) else { return };
    let mut grid = world.resource_mut::<MobGrid>();
    grid.grid.remove(layer, home, &(mob, id));
    grid.changes += 1;
    world.despawn(mob);
}

/// On the ticks columns are unloaded, removes the mobs of those that went.
pub(crate) fn despawn_unloaded(mut commands: Commands, tick: Res<Tick>, mut grid: ResMut<MobGrid>, layers: Query<&Layer>, mobs: Query<(Entity, &Mob, &NetId, &InLayer)>) {
    if tick.0 % UNLOAD_INTERVAL != 0 {
        return;
    }
    let before = grid.changes;
    for (entity, mob, id, in_layer) in &mobs {
        if !layers.get(in_layer.0).is_ok_and(|layer| layer.world.is_loaded(mob.home)) {
            commands.entity(entity).despawn();
            grid.grid.remove(in_layer.0, mob.home, &(entity, id.0));
            grid.changes = before + 1;
        }
    }
}

type Viewer<'a> = (&'a Stage, &'a InLayer, &'a View, &'a mut SeesMobs, &'a mut Outgoing);

/// Adds the mobs whose column came into each player's view and removes those that left it or
/// are gone. A player whose view stayed where it was while no mob came or went is skipped.
pub(crate) fn update(grid: Res<MobGrid>, mut viewers: Query<Viewer>, mobs: Query<&Mob>) {
    for (stage, in_layer, view, mut sees, mut out) in &mut viewers {
        let made_for = Some((view.moves(), grid.changes));
        if *stage != Stage::InWorld || sees.made_for == made_for {
            continue;
        }
        let mut covered: Vec<(Entity, u64)> = grid.grid.covered(in_layer.0, view).copied().collect();
        covered.sort_unstable_by_key(|(_, id)| *id);
        let new: Vec<&Mob> = covered.iter().filter(|(entity, _)| !sees.mobs.contains_key(entity)).filter_map(|(entity, _)| mobs.get(*entity).ok()).collect();
        for mob in &new {
            out.0.extend_from_slice(&mob.announcement);
        }
        for mob in &new {
            out.0.extend_from_slice(&mob.after);
        }
        let now: HashMap<Entity, u64> = covered.into_iter().collect();
        for (_, id) in sees.mobs.iter().filter(|(gone, _)| !now.contains_key(*gone)) {
            out.push(&RemoveEntity { entity_id_self: *id as i64 });
        }
        *sees = SeesMobs { mobs: now, made_for };
    }
}
