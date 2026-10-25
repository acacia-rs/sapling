//! Mobs, spawned through the plugin API or kept by a world folder: bodies (body.rs) that fall and
//! collide, stroll (strolling.rs) and hunt if their kind does (hunting.rs), shown as BDS announced their kind (`sapling_data::add_entity`) or
//! as they were stored; and the entity side of the handlers' context (backend.rs).
// TODO: no AI but strolling and hunting; their physics are the Java game's (`Physics::MOB`); a spawned mob is
// saved with the least a compound needs, which BDS was not tried on.

mod afflictions;
mod arrows;
mod attacks;
mod backend;
mod blast;
mod breezes;
mod dolphins;
mod drinks;
mod bullets;
mod charges;
mod death;
mod duels;
mod enchanted;
mod fireballs;
mod flight;
mod floating;
mod genes;
mod ghasts;
mod harm;
mod health;
mod hits;
mod hunting;
mod landing;
mod legs;
mod omens;
mod paths;
mod rams;
mod shooting;
mod shown;
mod slimes;
mod spells;
mod splashes;
mod strolling;
mod stares;
mod struck;
mod summoned;
mod teleports;
mod vexes;
mod wither;

use bevy_ecs::prelude::*;
use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use sapling_api::{EntityId, EntitySpawnEvent, SpawnReason};

use sapling_world::Actor;

use crate::body::Body;
use death::{die, leaves_loot};
use shown::{announcement, head, physics_of, spawn_sound};
pub(crate) use afflictions::{breathed_on, splashed};
pub(crate) use arrows::{ARROW, FIREBALL, Lands, Shot};
pub(crate) use flight::{Arrow, Launch, Loosed, Pickup, facing, from_storage as shot_from_storage, launch, lying as lying_shots, unstick};
pub(crate) use attacks::attacked;
pub(crate) use blast::{Wreck, explode};
pub(crate) use enchanted::thorned;
pub(crate) use harm::{holds, is_full_block, touches, wound};
pub(crate) use health::{Cause, Health};
pub(crate) use hunting::{on_difficulty, sees};
pub(crate) use omens::{Omens, befall, infested_hurt};
pub(crate) use struck::struck_by_lightning;
pub(crate) use wither::{build as build_wither, hurt as wither_hurt, spares};
use crate::components::{InLayer, Layer, NetId, Tick};
use crate::{blocks, entities, mobs};


/// A mob: spawned through the API, or kept by a world folder and dressed as it was stored
/// (`look`). Its [`Body`] and [`entities::Kind`] are beside it.
#[derive(Component)]
pub(crate) struct Creature {
    look: Option<Actor>,
}

impl Creature {
    /// One dressed as BDS announces its kind.
    pub(crate) fn plain() -> Self {
        Self { look: None }
    }

    fn is_baby(&self) -> bool {
        self.look.as_ref().is_some_and(|look| look.baby)
    }
}

/// A mob of `kind` with its feet at `feet` in `layer`: one BDS was recorded announcing, in a
/// loaded column. It is heard appearing.
pub(crate) fn spawn(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], reason: SpawnReason) -> Option<EntityId> {
    spawn_as(world, layer, kind, feet, reason, 0.0, None)
}

/// [`spawn`] facing `yaw`, and as its kind's spawn `event` makes it (a panda's gene) where known.
fn spawn_as(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], reason: SpawnReason, yaw: f32, event: Option<&str>) -> Option<EntityId> {
    let gene = event.and_then(|event| genes::of_event(kind, event)).or_else(|| genes::roll(world, kind));
    born(world, layer, kind, feet, Birth { reason, yaw, gene, sound: "spawn" })
}

/// How a mob comes: why, facing where, of which gene, and heard as what.
struct Birth {
    reason: SpawnReason,
    yaw: f32,
    gene: Option<genes::Gene>,
    sound: &'static str,
}

/// [`spawn`] as `birth` has it.
fn born(world: &mut World, layer: Entity, kind: &str, feet: [f32; 3], birth: Birth) -> Option<EntityId> {
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
    let entity = entities::spawn_mob(world, layer, kind, body, feet, |id| Some(announcement(id, kind, &body, None, None, yaw, gene.map_or(0, |gene| gene.0))))?;
    world.entity_mut(entity).insert(Creature { look: None });
    if let Some(gene) = gene {
        world.entity_mut(entity).insert(gene);
    }
    give_legs(world, entity, kind);
    if let Some(mut legs) = world.get_mut::<legs::Legs>(entity) {
        legs.yaw = yaw;
    }
    give_health(world, entity, kind, None, 0);
    let id = world.get::<NetId>(entity)?.0;
    harm::first_breath(world, entity, layer, id, kind, &body);
    entities::after_announcing(world, layer, [x, y, z], vec![spawn_sound(sound, id, kind, &body)]);
    let unique_id = entities::new_unique_id(world, entity)?;
    entities::keep(world, entity, layer, unique_id, sapling_world::new_actor(unique_id, kind, feet, [0.0; 3]));
    Some(EntityId(id))
}

/// Makes the stored mob `actor` (its compound `compound`) an entity of `layer`; false for a kind
/// with no recorded box, which stays a statue.
pub(crate) fn from_storage(world: &mut World, layer: Entity, actor: Actor, unique_id: i64, compound: Vec<u8>) -> bool {
    if actor.identifier == crate::dragon::CRYSTAL {
        return crate::dragon::crystal_from_storage(world, layer, actor.position, actor.yaw, unique_id, compound);
    }
    let Some(size) = sapling_data::mob_size(&actor.identifier) else { return false };
    let kind = actor.identifier.clone();
    let gene = genes::stored(&kind, actor.variant);
    let body = Body::new(actor.position, [0.0; 3], slimes::sizes::body_size(&kind, gene).unwrap_or(size));
    let Some(entity) = entities::spawn_mob(world, layer, &kind, body, actor.position, |id| Some(announcement(id, &kind, &body, Some(&actor), None, 0.0, actor.variant))) else { return false };
    if let Some(gene) = gene {
        world.entity_mut(entity).insert(gene);
    }
    give_legs(world, entity, &kind);
    give_health(world, entity, &kind, actor.health, actor.fire.max(0) as u16);
    world.entity_mut(entity).insert(Creature { look: Some(actor) });
    if kind == crate::dragon::DRAGON {
        crate::dragon::adopt(world, entity);
    }
    entities::keep(world, entity, layer, unique_id, compound);
    true
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
    dolphins::arm(world, entity, kind);
    drinks::arm(world, entity, kind);
    wither::arm(world, entity, kind);
}

/// Gives a mob of a kind that dies the health BDS announced its kind with, `left` of it, and
/// `burning` ticks of fire.
fn give_health(world: &mut World, entity: Entity, kind: &str, left: Option<f32>, burning: u16) {
    if let Some(max) = sapling_data::mob_health(kind).filter(|_| !NO_DEATH.contains(&kind)) {
        let max = genes::health(kind, world.get::<genes::Gene>(entity).copied(), max);
        world.entity_mut(entity).insert(Health::new(max, left, burning));
    }
}

/// A mob that died at the tick it holds; it goes [`DYING`] ticks after.
#[derive(Component)]
struct Dying(u64);

/// Ticks from a mob's death to its going, as BDS was seen to (research/captures/bds-kill.txt).
const DYING: u64 = 21;
/// Killed, these go at once, without a death or loot, as BDS's armor stand.
const NO_DEATH: &[&str] = &["minecraft:armor_stand"];

/// Kills the entity `id` (see `ShardCtx::kill_entity`).
fn kill(world: &mut World, id: EntityId) -> bool {
    let Some(entity) = entities::find(world, id).filter(|entity| world.get::<Dying>(*entity).is_none() && !crate::dragon::is_dying(world, *entity)) else { return false };
    let (Some(kind), Some(body), Some(layer)) = (world.get::<entities::Kind>(entity).map(|kind| kind.0.clone()), world.get::<Body>(entity).copied(), world.get::<InLayer>(entity).map(|layer| layer.0)) else { return false };
    if world.get::<Creature>(entity).is_none() || NO_DEATH.contains(&kind.as_str()) {
        mobs::remove(world, entity);
        return true;
    }
    if let Some(health) = world.get::<Health>(entity).copied() {
        let whole = health.left() + health.cushion();
        let Some(harm) = crate::damage::ask(world, entity, whole, Cause::Generic, None) else { return false };
        if harm < whole {
            let mut health = world.get_mut::<Health>(entity).expect("read above");
            health.lose(harm);
            let hit = harm::strike(id.0, &health, Cause::Generic, None);
            entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![hit]);
            return harm > 0.0;
        }
    }
    let death = EntityEvent { runtime_entity_id: id.0, event_id: EntityEventEventId::DeathAnimation, data: 0, fire_at_position: None };
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&death)]);
    die(world, entity, layer, &kind, &body);
    true
}

/// Moves every spawned mob a tick and hurts it by where it is; a dead one goes once it has died.
pub(crate) fn tick(world: &mut World) {
    flight::tick(world);
    afflictions::tick(world);
    omens::tick(world);
    spells::tick(world);
    bullets::tick(world);
    charges::tick(world);
    fireballs::tick(world);
    wither::tick(world);
    let now = world.resource::<Tick>().0;
    let dead: Vec<Entity> = world.query::<(Entity, &Dying)>().iter(world).filter(|(_, dying)| now >= dying.0 + DYING).map(|(entity, _)| entity).collect();
    for entity in dead {
        attacks::leave_experience(world, entity);
        slimes::split(world, entity);
        mobs::remove(world, entity);
    }
    let all: Vec<(Entity, Entity, u64, String)> =
        world.query_filtered::<(Entity, &InLayer, &NetId, &entities::Kind), With<Creature>>().iter(world).map(|(entity, layer, id, kind)| (entity, layer.0, id.0, kind.0.clone())).collect();
    for (entity, layer, id, kind) in all {
        // The fight moves its dragon (dragon.rs).
        if kind == crate::dragon::DRAGON && world.get::<crate::dragon::Dragon>(entity).is_some() {
            continue;
        }
        let Some(start) = world.get::<Body>(entity).copied() else { continue };
        // With `sapling:mobAi` off a mob still falls, is pushed and is hurt, but decides nothing.
        let thinks = world.get::<Dying>(entity).is_none() && crate::rules::on(world, layer, sapling_data::MOB_AI);
        if thinks {
            stares::watch(world, entity, layer, id, &kind, &start);
            drinks::tick(world, entity, layer, id, &start, hunting::prey_at(world, entity));
            teleports::tick(world, entity, layer, hunting::prey_at(world, entity));
        }
        // A teleport starts the tick afresh where it landed: no fall from where it left.
        let Some(start) = world.get::<Body>(entity).copied() else { continue };
        if thinks && !wither::steer(world, entity, layer) && !rams::steer(world, entity, layer, id, &kind) && !hunting::steer(world, entity, layer, id, &kind) {
            let traits = sapling_data::mob_traits(&kind);
            let strolls = world.resource::<crate::components::Settings>().0.mobs_stroll;
            if strolls {
                strolling::steer(world, entity, layer, traits.strolls.or(traits.hunts.map(|hunts| hunts.speed)), traits.panics, traits.burns_in_sun);
            }
        }
        if thinks {
            floating::tick(world, entity, layer, &kind, &start);
            slimes::tick(world, entity, layer, id, &kind, &start);
        }
        let Some((body, packets)) = entities::step(world, entity, layer, id, physics_of(&kind), 0.0) else { continue };
        blocks::to_viewers(world, layer, body.at.map(|c| c.floor() as i32), &packets);
        if thinks {
            hunting::strike(world, entity, layer, id, &kind, &body);
            rams::strike(world, entity, layer, id, &kind, &body);
            slimes::after_move(world, entity, layer, id, &kind, &start, &body);
            if blast::tick(world, entity, layer, id, &kind, &body) {
                continue;
            }
        }
        let before = world.get::<Health>(entity).map(|health| (health.left(), health.is_burning()));
        if harm::tick(world, entity, layer, id, &kind, &start, &body, world.get::<Dying>(entity).is_some()) {
            die(world, entity, layer, &kind, &body);
        }
        let health = world.get::<Health>(entity);
        if !packets.is_empty() || health.map(|health| (health.left(), health.is_burning())) != before {
            let shown = announcement(id, &kind, &body, world.get::<Creature>(entity).and_then(|creature| creature.look.as_ref()), health, world.get::<legs::Legs>(entity).map_or(0.0, |legs| legs.yaw), world.get::<genes::Gene>(entity).map_or(0, |gene| gene.0));
            mobs::reannounce(world, entity, shown);
        }
    }
}

