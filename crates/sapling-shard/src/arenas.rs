//! Worlds made at runtime from a template (docs/DESIGN.md "Arenas"): copy-on-write, never saved,
//! reset by dropping what they changed, removed when the game is over.

use std::collections::HashMap;
use std::sync::Arc;

use bevy_ecs::prelude::*;
use sapling_world::Template;

use crate::components::{Conn, DefaultLayer, InLayer, Layer};
use crate::handoff::{Peers, ShardMsg};
use crate::view::View;
use crate::worlds::{self, Atlas, Place};

/// The templates handlers can name, with where players appear in a world made from each.
#[derive(Resource, Default)]
pub(crate) struct Templates(HashMap<String, (Arc<Template>, [f32; 3])>);

impl Templates {
    pub fn add(&mut self, name: &str, template: Arc<Template>, spawn: [f32; 3]) {
        self.0.insert(name.to_owned(), (template, spawn));
    }
}

/// Marks a world made here from a template: only those are reset and removed.
#[derive(Component)]
struct Arena;

fn arena(world: &World, name: &str) -> Option<Entity> {
    match world.resource::<Atlas>().find(name) {
        Some(Place::Here(layer)) if world.get::<Arena>(layer).is_some() => Some(layer),
        _ => None,
    }
}

fn players_in(world: &mut World, layer: Entity) -> Vec<Entity> {
    let mut players = world.query_filtered::<(Entity, &InLayer), With<Conn>>();
    players.iter(world).filter(|(_, in_layer)| in_layer.0 == layer).map(|(player, _)| player).collect()
}

pub(crate) fn create(world: &mut World, name: &str, template: &str) -> bool {
    let Some((template, spawn)) = world.resource::<Templates>().0.get(template).cloned() else { return false };
    let Some(layer) = worlds::add(world, name, sapling_world::World::arena(&template), spawn) else { return false };
    world.entity_mut(layer).insert(Arena);
    true
}

/// Removes an arena: its players go to the shard's first world, its mobs go with it.
pub(crate) fn remove(world: &mut World, name: &str) -> bool {
    let Some(layer) = arena(world, name) else { return false };
    crate::world_notices::closing(world, layer);
    let fallback = world.resource::<DefaultLayer>().0;
    for player in players_in(world, layer) {
        worlds::enter(world, player, fallback);
    }
    let mut others = world.query_filtered::<(Entity, &InLayer), Without<Conn>>();
    let mobs: Vec<Entity> = others.iter(world).filter(|(_, in_layer)| in_layer.0 == layer).map(|(mob, _)| mob).collect();
    for mob in mobs {
        world.despawn(mob);
    }
    world.resource_mut::<Atlas>().remove(name);
    world.resource::<Peers>().tell_others(|| ShardMsg::WorldRemoved { name: name.to_owned() });
    world.despawn(layer);
    true
}

/// Puts an arena back as its template: the columns it changed are announced again to the
/// players who have them, whose clients then ask for the sections anew.
pub(crate) fn reset(world: &mut World, name: &str) -> bool {
    let Some(layer) = arena(world, name) else { return false };
    let changed = world.get_mut::<Layer>(layer).map(|mut layer| layer.world.reset()).unwrap_or_default();
    for player in players_in(world, layer) {
        if let Some(mut view) = world.get_mut::<View>(player) {
            changed.iter().for_each(|pos| view.resend(*pos));
        }
    }
    true
}
