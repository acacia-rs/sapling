//! How a mob comes into the world: spawned (through the API, an egg, a spawner, breeding) or made
//! from what a world folder kept of it, with its legs, health, raising and trade.

use bevy_ecs::prelude::*;
use sapling_api::{EntityId, EntitySpawnEvent, SpawnReason};
use sapling_world::Actor;

use super::shown::{announcement, reshow, spawn_sound};
use super::{Creature, Health, NO_DEATH, afflictions, bees, breezes, creaking, dolphins, drinks, frogs, genes, ghasts, harm, hunting, husbandry, legs, rams, slimes, spells, squids, stares, strides, strolling, swimming, teleports, vexes, villagers, wings, wither};
use crate::body::Body;
use crate::components::{Layer, NetId};
use crate::entities;

/// A mob of `kind` with its feet at `feet` in `layer`: one BDS was recorded announcing, in a
/// loaded column. It is heard appearing.
pub(crate) fn spawn(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], reason: SpawnReason) -> Option<EntityId> {
    spawn_as(world, layer, kind, feet, reason, 0.0, None)
}

/// [`spawn`] facing `yaw`, and as its kind's spawn `event` makes it (a panda's gene) where known.
pub(super) fn spawn_as(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], reason: SpawnReason, yaw: f32, event: Option<&str>) -> Option<EntityId> {
    let gene = event.and_then(|event| genes::of_event(kind, event)).or_else(|| genes::roll(world, kind)).or_else(|| husbandry::frog_variant(world, layer, kind, feet));
    let id = born(world, layer, kind, feet, Birth { reason, yaw, gene, sound: "spawn" })?;
    let entity = entities::find(world, id)?;
    if let Some(gene) = gene {
        entities::restore(world, entity, |compound, _| sapling_world::varied_actor(compound, gene.0, yaw).ok());
    }
    villagers::arm(world, entity, layer, kind, false, event);
    super::drowned::arm(world, entity, kind, event);
    super::captains::arm(world, entity, kind, event);
    if event.is_some() {
        super::warden::arm(world, entity, kind, event);
    }
    Some(id)
}

/// How a mob comes: why, facing where, of which gene, and heard as what.
pub(super) struct Birth {
    pub reason: SpawnReason,
    pub yaw: f32,
    pub gene: Option<genes::Gene>,
    pub sound: &'static str,
}

/// [`spawn`] as `birth` has it.
pub(super) fn born(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], birth: Birth) -> Option<EntityId> {
    let Birth { reason, yaw, gene, sound } = birth;
    let size = sapling_data::mob_size(kind)?;
    let level = &world.get::<Layer>(layer)?.world;
    let [x, y, z] = feet.map(|c| c.floor() as i32);
    if !(level.in_height(y) && level.is_loaded(sapling_world::ChunkPos::of_block(x, z))) {
        return None;
    }
    if crate::plugins::stopped_in(world, layer, |_| EntitySpawnEvent::new(kind.to_owned(), feet, reason)) {
        return None;
    }
    let body = Body::new(feet, [0.0; 3], slimes::sizes::body_size(kind, gene).unwrap_or(size));
    let properties = super::properties::rolled(world, layer, kind, feet, reason == SpawnReason::Bred);
    let rolled = strides::roll(world, kind);
    let entity = entities::spawn_mob(world, layer, kind, body, feet, |id| Some(announcement(id, kind, &body, (None, None, Some(&properties), rolled.as_ref()), None, yaw, gene.map_or(0, |gene| gene.0))))?;
    world.entity_mut(entity).insert((Creature { look: None }, properties));
    if let Some(rolled) = rolled {
        world.entity_mut(entity).insert(strides::Strode(rolled));
    }
    husbandry::adopt(world, entity, kind, None);
    if let Some(gene) = gene {
        world.entity_mut(entity).insert(gene);
    }
    give_legs(world, entity, kind);
    if let Some(mut legs) = world.get_mut::<legs::Legs>(entity) {
        legs.yaw = yaw;
    }
    give_health(world, entity, kind, None, 0);
    wither::begin(world, entity, layer);
    let id = world.get::<NetId>(entity)?.0;
    harm::first_breath(world, entity, layer, id, kind, &body);
    entities::after_announcing(world, layer, [x, y, z], vec![spawn_sound(sound, id, kind, &body)]);
    let unique_id = entities::new_unique_id(world, entity)?;
    entities::keep(world, entity, layer, unique_id, strides::keep(sapling_world::new_actor(unique_id, kind, feet, [0.0; 3]), rolled));
    Some(EntityId(id))
}

/// Makes the stored mob `actor` (its compound `compound`) an entity of `layer`; false for a kind
/// with no recorded box, which stays a statue.
pub(crate) fn from_storage(world: &mut World, layer: Entity, actor: Actor, unique_id: i64, compound: Vec<u8>) -> bool {
    if actor.identifier == crate::dragon::CRYSTAL {
        return crate::dragon::crystal_from_storage(world, layer, actor.position, actor.yaw, unique_id, compound);
    }
    if actor.identifier == husbandry::KNOT {
        return husbandry::knot_from_storage(world, layer, actor.position, unique_id, compound);
    }
    let Some(size) = sapling_data::mob_size(&actor.identifier) else { return false };
    let kind = actor.identifier.clone();
    let gene = genes::stored(&kind, actor.variant);
    let body = Body::new(actor.position, [0.0; 3], slimes::sizes::body_size(&kind, gene).unwrap_or(size));
    let (kept, compound) = strides::stored(world, &kind, compound);
    let Some(entity) = entities::spawn_mob(world, layer, &kind, body, actor.position, |id| Some(announcement(id, &kind, &body, (Some(&actor), None, None, kept.as_ref()), None, 0.0, actor.variant))) else { return false };
    if let Some(gene) = gene {
        world.entity_mut(entity).insert(gene);
    }
    if let Some(kept) = kept {
        world.entity_mut(entity).insert(strides::Strode(kept));
    }
    give_legs(world, entity, &kind);
    give_health(world, entity, &kind, actor.health, actor.fire.max(0) as u16);
    afflictions::adopt(world, entity, &compound);
    wither::adopt(world, entity, &compound);
    husbandry::adopt(world, entity, &kind, Some((&actor, &compound)));
    world.entity_mut(entity).insert(Creature { look: Some(actor) });
    super::piglins::adopt(world, entity);
    super::captains::adopt(world, entity);
    if kind == crate::dragon::DRAGON {
        crate::dragon::adopt(world, entity);
    }
    entities::keep(world, entity, layer, unique_id, compound);
    villagers::arm(world, entity, layer, &kind, true, None);
    reshow(world, entity);
    crate::riding::adopt(world, entity);
    true
}

/// A mob of `kind` that appears at `feet` for `reason`, as [`spawn`] but the entity.
pub(super) fn spawn_entity(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], reason: SpawnReason) -> Option<Entity> {
    spawn(world, layer, kind, feet, reason).and_then(|id| entities::find(world, id))
}

/// Gives a mob of a kind that strolls or hunts what it does so with.
fn give_legs(world: &mut World, entity: Entity, kind: &str) {
    let traits = sapling_data::mob_traits(kind);
    if traits.strolls.is_some() || traits.panics.is_some() || traits.hunts.is_some() {
        world.entity_mut(entity).insert((legs::Legs::default(), strolling::Stroll::default()));
    }
    hunting::arm(world, entity, kind);
    slimes::arm(world, entity, kind);
    teleports::arm(world, entity, kind);
    rams::arm(world, entity, kind);
    spells::arm(world, entity, kind);
    breezes::arm(world, entity, kind);
    ghasts::arm(world, entity, kind);
    vexes::arm(world, entity, kind);
    stares::arm(world, entity, kind);
    creaking::arm(world, entity, kind);
    super::warden::arm(world, entity, kind, None);
    dolphins::arm(world, entity, kind);
    swimming::arm(world, entity, kind);
    squids::arm(world, entity, kind);
    frogs::arm(world, entity, kind);
    wings::arm(world, entity, kind);
    bees::arm(world, entity, kind);
    drinks::arm(world, entity, kind);
    wither::arm(world, entity, kind);
}

/// Gives a mob of a kind that dies the health BDS announced its kind with (a mount its own),
/// `left` of it, and `burning` ticks of fire.
fn give_health(world: &mut World, entity: Entity, kind: &str, left: Option<f32>, burning: u16) {
    if let Some(max) = sapling_data::mob_health(kind).filter(|_| !NO_DEATH.contains(&kind)) {
        let max = genes::health(kind, world.get::<genes::Gene>(entity).copied(), max);
        let max = world.get::<strides::Strode>(entity).map_or(max, |strode| strode.0.health);
        world.entity_mut(entity).insert(Health::new(max, left, burning));
    }
}
