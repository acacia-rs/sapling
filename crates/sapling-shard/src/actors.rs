//! The actors a world folder keeps, in and out of the shard. A column that arrives has its mobs
//! become creatures and its items lying items (entities that move and can be taken); what the
//! shard does not move (paintings, kinds without a recorded box) is shown as a statue. Before
//! columns are saved, each column's actors are written anew: the compounds the shard does not
//! move as they were, and every kept entity filed under the column with its place, motion,
//! health and fire now (`entities::Stored`), so a mob that walked into another column is saved there, and one that
//! is gone is not.

use bevy_ecs::prelude::*;
use sapling_world::ChunkPos;

use crate::body::Body;
use crate::chunks::UNLOAD_INTERVAL;
use crate::components::{InLayer, Layer, Tick};
use crate::entities::{Managed, Stored};
use crate::kept::Kept;
use crate::view::View;
use crate::{blocks, creatures, drops, mobs, orbs};

const ITEM: &str = "minecraft:item";
const ORB: &str = "minecraft:xp_orb";
const FALLING_BLOCK: &str = "minecraft:falling_block";

/// Brings in the actors of the columns that came from storage this tick, and notes their circuits.
pub(crate) fn arrive(world: &mut World) {
    let layers: Vec<Entity> = world.query_filtered::<Entity, With<Layer>>().iter(world).collect();
    for layer in layers {
        let Some(arrived) = world.get_mut::<Layer>(layer).map(|mut level| level.world.take_arrived()) else { continue };
        for home in arrived {
            blocks::circuits_arrived(world, layer, home);
            blocks::plants_arrived(world, layer, home);
            blocks::ticks_arrived(world, layer, home);
            let Some(record) = world.get::<Layer>(layer).map(|level| level.world.actors(home).to_vec()) else { continue };
            match sapling_world::stored_actors(&record) {
                Ok(stored) => stored.into_iter().for_each(|(actor, unique_id, compound)| bring_in(world, layer, home, actor, unique_id, compound)),
                Err(error) => tracing::warn!(x = home.x, z = home.z, %error, "a column's actors are unreadable; showing none of them"),
            }
        }
    }
}

/// A kept entity's stored compound with its place, motion, health, fire and raising as they are now.
pub(crate) fn compound_now(body: &Body, stored: &Stored, health: Option<&creatures::Health>, raised: Option<&creatures::Raised>) -> Option<Vec<u8>> {
    let moved = sapling_world::moved_actor(&stored.compound, body.at, body.motion).ok()?;
    let harmed = match health {
        Some(health) => sapling_world::harmed_actor(&moved, health.left(), health.max(), health.burning() as i16).ok()?,
        None => moved,
    };
    match raised {
        Some(raised) => creatures::kept_raising(&harmed, raised),
        None => Some(harmed),
    }
}

pub(crate) fn bring_in(world: &mut World, layer: Entity, home: ChunkPos, actor: Option<sapling_world::Actor>, unique_id: Option<i64>, compound: Vec<u8>) {
    let (Some(actor), Some(unique_id)) = (actor, unique_id) else { return };
    let moved = match (&actor.item, actor.identifier == ITEM) {
        (Some(stack), true) => drops::from_storage(world, layer, stack.clone(), actor.position, unique_id, compound),
        (_, false) if actor.identifier == ORB => orbs::from_storage(world, layer, actor.position, unique_id, compound),
        (_, false) if actor.identifier == FALLING_BLOCK => blocks::falling_from_storage(world, layer, actor.position, unique_id, compound),
        (_, false) if crate::shots::is_kept(&actor.identifier) => crate::shots::from_storage(world, layer, &actor, unique_id, compound),
        (_, false) if crate::vehicles::is_vehicle(&actor.identifier) => crate::vehicles::from_storage(world, layer, &actor, unique_id, compound),
        (_, false) if actor.painting.is_none() => creatures::from_storage(world, layer, actor.clone(), unique_id, compound),
        _ => false,
    };
    if !moved {
        let feet = [home.x as f32 * 16.0, actor.position[1], home.z as f32 * 16.0];
        let filed = if ChunkPos::of_block(actor.position[0].floor() as i32, actor.position[2].floor() as i32) == home { actor.position } else { feet };
        if mobs::spawn_shown(world, layer, filed, |id| mobs::announcement(&actor, id)).is_none() {
            tracing::debug!(x = home.x, z = home.z, item = ?actor.item, "a dropped item no client knows; not shown");
        }
    }
}

/// [`write_back`] on the ticks columns are saved.
pub(crate) fn write_back_before_saving(world: &mut World) {
    if world.resource::<Tick>().0 % UNLOAD_INTERVAL == 0 {
        write_back(world);
    }
}

/// Writes every loaded column's actors anew from what the shard has (see the module).
pub(crate) fn write_back(world: &mut World) {
    let kept: Vec<(Entity, ChunkPos, i64, Vec<u8>)> = world
        .query::<(&InLayer, &Body, &Stored, &crate::tags::UniqueId, Option<&creatures::Health>, Option<&creatures::Raised>)>()
        .iter(world)
        .filter_map(|(layer, body, stored, unique_id, health, raised)| {
            let home = ChunkPos::of_block(body.at[0].floor() as i32, body.at[2].floor() as i32);
            Some((layer.0, home, unique_id.0, compound_now(body, stored, health, raised)?))
        })
        .collect();
    let managed = world.get_resource::<Managed>().map(|managed| managed.0.clone()).unwrap_or_default();
    let layers: Vec<Entity> = world.query_filtered::<Entity, With<Layer>>().iter(world).collect();
    for layer in layers {
        let views: Vec<&View> = world.query::<(&InLayer, &View)>().iter(world).filter(|(in_layer, _)| in_layer.0 == layer).map(|(_, view)| view).collect();
        let held = world.get::<Kept>(layer);
        let seen = |pos: ChunkPos| views.iter().any(|view| view.covers(pos)) || held.is_some_and(|held| held.holds(pos));
        let Some(level) = world.get::<Layer>(layer).map(|level| &level.world) else { continue };
        let mut columns = level.columns_with_actors();
        columns.extend(kept.iter().filter(|(in_layer, ..)| *in_layer == layer).map(|(_, home, ..)| *home));
        columns.sort_unstable_by_key(|pos| (pos.x, pos.z));
        columns.dedup();
        let records: Vec<(ChunkPos, Vec<u8>)> = columns
            .into_iter()
            .filter(|pos| level.is_loaded(*pos))
            .filter_map(|pos| {
                let stored = sapling_world::stored_actors(level.actors(pos)).ok()?;
                let here = || kept.iter().filter(|(in_layer, home, ..)| *in_layer == layer && *home == pos);
                // A column nobody sees keeps where its own entities were: a fish that never stops would
                // otherwise make it changed at every save and never let it unload.
                let mut filed: Vec<i64> = stored.iter().filter_map(|(_, id, _)| id.filter(|id| managed.contains(&(layer, *id)))).collect();
                let mut now: Vec<i64> = here().map(|(_, _, id, _)| *id).collect();
                (filed.sort_unstable(), now.sort_unstable());
                if filed == now && !seen(pos) {
                    return None;
                }
                let unmoved = stored.into_iter().filter(|(_, id, _)| id.is_none_or(|id| !managed.contains(&(layer, id))));
                let mut record: Vec<u8> = unmoved.flat_map(|(.., compound)| compound).collect();
                here().for_each(|(.., compound)| record.extend(compound));
                Some((pos, record))
            })
            .collect();
        let Some(mut level) = world.get_mut::<Layer>(layer) else { continue };
        for (pos, record) in records {
            level.world.set_actors(pos, record);
        }
    }
}
