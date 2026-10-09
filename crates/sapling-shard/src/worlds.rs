//! The worlds of a shard, by name, and the way from one to another. A world is a layer entity;
//! a player points at its layer, and moving it within the shard is that one write plus what the
//! client needs to swap what it shows (docs/DESIGN.md "Step 5 as built").

mod ctx;

use std::collections::HashMap;

use bevy_ecs::prelude::*;
use sapling_world::ChunkPos;

use crate::blobs::BlobCache;
use crate::chunks::SectionBacklog;
use crate::components::{DefaultLayer, InLayer, Layer, Outgoing, in_world_identity};
use crate::handoff::{Peers, ShardMsg};
use crate::plugins::Plugins;
use crate::view::View;
use crate::{clock, movement};

#[derive(Component)]
pub(crate) struct Named(pub String);

/// Where a world is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Place {
    Here(Entity),
    /// On the shard of this index.
    Shard(usize),
}

/// Every world of the server by name. Each shard keeps its own copy; the others' worlds are
/// what they announced (`ShardMsg::WorldAdded`).
#[derive(Resource, Default)]
pub(crate) struct Atlas {
    here: HashMap<String, Entity>,
    elsewhere: HashMap<String, usize>,
}

impl Atlas {
    pub fn find(&self, name: &str) -> Option<Place> {
        self.here
            .get(name)
            .map(|layer| Place::Here(*layer))
            .or_else(|| self.elsewhere.get(name).map(|shard| Place::Shard(*shard)))
    }

    pub fn learn(&mut self, name: String, shard: usize) {
        self.elsewhere.insert(name, shard);
    }

    pub fn forget(&mut self, name: &str) {
        self.elsewhere.remove(name);
    }

    /// Takes a world of this shard off the map.
    pub fn remove(&mut self, name: &str) {
        self.here.remove(name);
    }

    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .here
            .keys()
            .chain(self.elsewhere.keys())
            .cloned()
            .collect();
        names.sort();
        names.dedup();
        names
    }
}

/// The world a handler's calls act on when they name none; unset, the shard's first.
#[derive(Resource, Default)]
pub(crate) struct Scope(Option<Entity>);

/// What happened to players' worlds this tick, for the handlers.
pub(crate) enum Change {
    Moved {
        entity: Entity,
        from: String,
        to: String,
    },
    Failed {
        entity: Entity,
        to: String,
    },
}

#[derive(Resource, Default)]
pub(crate) struct Changes(pub Vec<Change>);

/// Adds a world to the shard and tells the other shards; `None` for a name that is taken.
pub(crate) fn add(
    world: &mut World,
    name: &str,
    level: sapling_world::World,
    spawn: [f32; 3],
) -> Option<Entity> {
    if world.resource::<Atlas>().find(name).is_some() {
        return None;
    }
    let layer = world
        .spawn((
            Layer {
                world: level,
                spawn,
            },
            Named(name.to_owned()),
        ))
        .id();
    world
        .resource_mut::<Atlas>()
        .here
        .insert(name.to_owned(), layer);
    let peers = world.resource::<Peers>();
    peers.tell_others(|| ShardMsg::WorldAdded {
        name: name.to_owned(),
        shard: peers.index,
    });
    Some(layer)
}

/// Tells the other shards that this shard's worlds are gone with it.
pub(crate) fn withdraw(world: &World) {
    let peers = world.resource::<Peers>();
    for name in world.resource::<Atlas>().here.keys() {
        peers.tell_others(|| ShardMsg::WorldRemoved { name: name.clone() });
    }
}

pub(crate) fn name_of(world: &World, layer: Entity) -> String {
    world
        .get::<Named>(layer)
        .map(|name| name.0.clone())
        .unwrap_or_default()
}

/// The layer of the world in scope.
pub(crate) fn current(world: &World) -> Entity {
    let scope = world
        .resource::<Scope>()
        .0
        .filter(|layer| world.get::<Layer>(*layer).is_some());
    scope.unwrap_or_else(|| world.resource::<DefaultLayer>().0)
}

pub(crate) fn focus(world: &mut World, layer: Option<Entity>) {
    world.resource_mut::<Scope>().0 = layer;
}

/// Puts a player's world in scope for the handlers about to be called for it.
pub(crate) fn focus_on(world: &mut World, player: Entity) {
    let layer = world.get::<InLayer>(player).map(|in_layer| in_layer.0);
    focus(world, layer);
}

/// What a client needs after its player's world changed under it: every column anew, nothing
/// held from the old world, its feet on the new spawn and the new world's time. The entities of
/// the old world leave with the next visibility diff, which no longer finds them in view.
pub(crate) fn settle(world: &mut World, player: Entity, layer: Entity) {
    let Some((spawn, dimension)) = world
        .get::<Layer>(layer)
        .map(|layer| (layer.spawn, layer.world.dimension()))
    else {
        return;
    };
    let to = ChunkPos::of_block(spawn[0].floor() as i32, spawn[2].floor() as i32);
    if let Ok((mut view, mut blobs, mut backlog, mut out)) = world
        .query::<(
            &mut View,
            &mut BlobCache,
            &mut SectionBacklog,
            &mut Outgoing,
        )>()
        .get_mut(world, player)
    {
        blobs.release();
        backlog.clear();
        // A vanilla client keeps some of the old world's columns the new view does not cover:
        // each is overwritten with an empty one, while its publisher area is still the old one.
        let empty = sapling_world::empty_column(blobs.is_enabled());
        for pos in view.leave(to) {
            empty.blobs.iter().for_each(|blob| blobs.advertise(blob));
            out.push(&empty.packet(pos.x, pos.z, dimension));
        }
    }
    // Inputs are ignored until the client says it followed: they are positions in the old world.
    movement::teleport_onto_ground(world, player, spawn);
    clock::tell(world, layer, player);
}

/// Moves a player to another world of this shard.
pub(crate) fn enter(world: &mut World, player: Entity, layer: Entity) {
    crate::containers::close(world, player);
    let Some(mut in_layer) = world.get_mut::<InLayer>(player) else {
        return;
    };
    let from = std::mem::replace(&mut in_layer.0, layer);
    let (from, to) = (name_of(world, from), name_of(world, layer));
    settle(world, player, layer);
    world.resource_mut::<Changes>().0.push(Change::Moved {
        entity: player,
        from,
        to,
    });
}

/// Tells the handlers which players changed worlds and which transfers failed.
pub(crate) fn dispatch(world: &mut World, plugins: &mut Plugins) {
    for change in std::mem::take(&mut world.resource_mut::<Changes>().0) {
        let (Change::Moved { entity, .. } | Change::Failed { entity, .. }) = &change;
        let Some(player) = in_world_identity(world, *entity) else {
            continue;
        };
        focus_on(world, *entity);
        match change {
            Change::Moved { from, to, .. } => plugins.each(world, |handler, ctx| {
                handler.on_world_change(ctx, &player, &from, &to)
            }),
            Change::Failed { to, .. } => plugins.each(world, |handler, ctx| {
                handler.on_transfer_failed(ctx, &player, &to)
            }),
        }
    }
    focus(world, None);
}
