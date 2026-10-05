//! The mobs and dropped items a world's storage holds, shown to players. Each stored actor of a
//! loaded column is an entity that stays where it was saved: nothing moves it, and nothing a
//! player does reaches it. It leaves with its column, and the column saves it back as it was
//! (`sapling_world::Column`).
// TODO: everything that makes a mob more than a statue.

use std::collections::HashMap;

use acacia_proto::packets::RemoveEntity;
use acacia_proto::types::MetadataFlags1;
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

/// Every mob with its id, by the column it is in.
#[derive(Resource, Default)]
pub(crate) struct MobGrid {
    grid: Grid<(Entity, u64)>,
    /// Counts the ticks that added or removed mobs.
    changes: u64,
}

/// `None` for a dropped item that names no item a client knows.
fn announcement(actor: &Actor, id: u64) -> Option<Vec<Bytes>> {
    if actor.identifier == ITEM {
        let item = sapling_data::add_item_entity(id, &stack(actor.item.as_ref()?), actor.position)?;
        return Some(vec![sapling_net::encode(&item)]);
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
    let mut packets = vec![sapling_net::encode(&sapling_data::add_entity(&AddEntityOptions {
        flags: MetadataFlags1(actor.states.iter().fold(0, |flags, state| flags | flag(state).0)),
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
    }))];
    if actor.armor.iter().any(Option::is_some) {
        let slots: Vec<_> = actor.armor.iter().map(|slot| slot.as_ref().map(stack)).collect();
        packets.push(sapling_net::encode(&sapling_data::mob_armor(id, &slots)));
    }
    for (held, offhand) in [(&actor.mainhand, false), (&actor.offhand, true)] {
        packets.extend(held.as_ref().and_then(|held| sapling_data::mob_hand(id, &stack(held), offhand)).map(|held| sapling_net::encode(&held)));
    }
    Some(packets)
}

/// Makes entities of the actors in the columns that came from storage this tick.
pub(crate) fn spawn_arrived(mut commands: Commands, mut players: ResMut<Players>, mut grid: ResMut<MobGrid>, mut layers: Query<(Entity, &mut Layer)>) {
    let before = players.next_net_id;
    for (layer, mut level) in &mut layers {
        for home in level.world.take_arrived() {
            let stored = match sapling_world::actors(level.world.actors(home)) {
                Ok(stored) => stored,
                Err(error) => {
                    tracing::warn!(x = home.x, z = home.z, %error, "a column's actors are unreadable; showing none of them");
                    continue;
                }
            };
            for actor in stored {
                let id = players.next_net_id;
                let Some(announcement) = announcement(&actor, id) else {
                    tracing::debug!(x = home.x, z = home.z, item = ?actor.item, "a dropped item no client knows; not shown");
                    continue;
                };
                players.next_net_id += 1;
                let mob = commands.spawn((Mob { announcement, home }, NetId(id), InLayer(layer))).id();
                grid.grid.insert(layer, home, (mob, id));
            }
        }
    }
    if players.next_net_id != before {
        grid.changes += 1;
    }
}

/// Puts a stack into the world as an item lying at `feet`, shown like a stored one; `None` for
/// an item no client knows.
pub(crate) fn spawn_item(world: &mut World, layer: Entity, item: &ItemStack, feet: [f32; 3]) -> Option<Entity> {
    let id = world.resource::<Players>().next_net_id;
    let announcement = vec![sapling_net::encode(&sapling_data::add_item_entity(id, &stack(item), feet)?)];
    world.resource_mut::<Players>().next_net_id += 1;
    let home = ChunkPos::of_block(feet[0].floor() as i32, feet[2].floor() as i32);
    let mob = world.spawn((Mob { announcement, home }, NetId(id), InLayer(layer))).id();
    let mut grid = world.resource_mut::<MobGrid>();
    grid.grid.insert(layer, home, (mob, id));
    grid.changes += 1;
    Some(mob)
}

/// Takes a mob out of the world; those who saw it are told by [`update`].
pub(crate) fn remove(world: &mut World, mob: Entity) {
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
        for (entity, _) in covered.iter().filter(|(entity, _)| !sees.mobs.contains_key(entity)) {
            if let Ok(mob) = mobs.get(*entity) {
                out.0.extend_from_slice(&mob.announcement);
            }
        }
        let now: HashMap<Entity, u64> = covered.into_iter().collect();
        for (_, id) in sees.mobs.iter().filter(|(gone, _)| !now.contains_key(*gone)) {
            out.push(&RemoveEntity { entity_id_self: *id as i64 });
        }
        *sees = SeesMobs { mobs: now, made_for };
    }
}
