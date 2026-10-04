//! The mobs a world's storage holds, shown to players. Each stored actor of a loaded column is an
//! entity that stands where it was saved: nothing moves it, and nothing a player does reaches it.
//! It leaves with its column, and the column saves it back as it was (`sapling_world::Column`).
// TODO: item entities (AddItemEntity needs the item registry), an index of mobs by chunk (the
// diff is O(players x mobs) a tick), and everything that makes a mob more than a statue.

use std::collections::HashMap;

use acacia_proto::packets::RemoveEntity;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_data::AddEntityOptions;
use sapling_world::{Actor, ChunkPos};

use crate::chunks::UNLOAD_INTERVAL;
use crate::components::{InLayer, Layer, NetId, Outgoing, Players, Stage, Tick};
use crate::view::View;

const ITEM: &str = "minecraft:item";

/// A stored actor, in the column it was stored with.
#[derive(Component)]
pub(crate) struct Mob {
    actor: Actor,
    home: ChunkPos,
}

/// The mobs this player's client has as entities, with the ids it knows them by.
#[derive(Component, Default)]
pub(crate) struct SeesMobs(HashMap<Entity, u64>);

/// Makes entities of the actors in the columns that came from storage this tick.
pub(crate) fn spawn_arrived(mut commands: Commands, mut players: ResMut<Players>, mut layers: Query<(Entity, &mut Layer)>) {
    for (layer, mut level) in &mut layers {
        for home in level.world.take_arrived() {
            let stored = match sapling_world::actors(level.world.actors(home)) {
                Ok(stored) => stored,
                Err(error) => {
                    tracing::warn!(x = home.x, z = home.z, %error, "a column's actors are unreadable; showing none of them");
                    continue;
                }
            };
            for actor in stored.into_iter().filter(|actor| actor.identifier != ITEM) {
                let id = players.next_net_id;
                players.next_net_id += 1;
                commands.spawn((Mob { actor, home }, NetId(id), InLayer(layer)));
            }
        }
    }
}

/// On the ticks columns are unloaded, removes the mobs of those that went.
pub(crate) fn despawn_unloaded(mut commands: Commands, tick: Res<Tick>, layers: Query<&Layer>, mobs: Query<(Entity, &Mob, &InLayer)>) {
    if tick.0 % UNLOAD_INTERVAL != 0 {
        return;
    }
    for (entity, mob, in_layer) in &mobs {
        if !layers.get(in_layer.0).is_ok_and(|layer| layer.world.is_loaded(mob.home)) {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_packet(mob: &Mob, id: &NetId) -> Bytes {
    let actor = &mob.actor;
    sapling_net::encode(&sapling_data::add_entity(&AddEntityOptions {
        id: id.0,
        identifier: &actor.identifier,
        feet: actor.position,
        yaw: actor.yaw,
        pitch: actor.pitch,
        variant: actor.variant,
        mark_variant: actor.mark_variant,
        color: actor.color,
        baby: actor.baby,
        name: &actor.name,
    }))
}

type Viewer<'a> = (&'a Stage, &'a InLayer, &'a View, &'a mut SeesMobs, &'a mut Outgoing);

/// Adds the mobs whose column came into each player's view and removes those that left it or
/// are gone.
pub(crate) fn update(mut viewers: Query<Viewer>, mobs: Query<(Entity, &Mob, &NetId, &InLayer)>) {
    let mut spawns: HashMap<Entity, Bytes> = HashMap::new();
    for (stage, in_layer, view, mut sees, mut out) in &mut viewers {
        if *stage != Stage::InWorld {
            continue;
        }
        let mut now = HashMap::new();
        for (entity, mob, id, layer) in &mobs {
            if layer.0 != in_layer.0 || !view.covers(mob.home) {
                continue;
            }
            now.insert(entity, id.0);
            if !sees.0.contains_key(&entity) {
                out.0.push(spawns.entry(entity).or_insert_with(|| spawn_packet(mob, id)).clone());
            }
        }
        for (_, id) in sees.0.iter().filter(|(gone, _)| !now.contains_key(*gone)) {
            out.push(&RemoveEntity { entity_id_self: *id as i64 });
        }
        sees.0 = now;
    }
}
