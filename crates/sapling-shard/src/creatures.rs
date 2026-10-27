//! Mobs, spawned through the plugin API or kept by a world folder (birth.rs): bodies (body.rs) that fall and
//! collide, stroll (strolling.rs), swim (swimming.rs, squids.rs), fly about (wings.rs), hunt if their kind does (hunting.rs) and are raised (husbandry.rs), shown as BDS announced their kind (`sapling_data::add_entity`) or
//! as they were stored; and the entity side of the handlers' context (backend.rs).
// TODO: no AI but strolling, swimming, flying about, hunting and raising; a spawned mob is
// saved with the least a compound needs, which BDS was not tried on.

mod afflictions;
mod arrows;
mod attacks;
mod backend;
mod birth;
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
mod husbandry;
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
mod squids;
mod strolling;
mod stares;
mod struck;
mod summoned;
mod swimming;
mod teleports;
mod vexes;
mod villagers;
mod wings;
mod wither;

use bevy_ecs::prelude::*;
use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use sapling_api::EntityId;

use sapling_world::Actor;

use crate::body::Body;
use birth::{Birth, born, spawn_as, spawn_entity};
use death::{die, leaves_loot};
use shown::{head, physics_of, reshow};
pub(crate) use birth::{from_storage, spawn};
pub(crate) use shown::is_baby;
pub(crate) use afflictions::{breathed_on, splashed};
pub(crate) use arrows::{ARROW, FIREBALL, Lands, Shot};
pub(crate) use flight::{Arrow, Launch, Loosed, Pickup, facing, from_storage as shot_from_storage, launch, lying as lying_shots, sound, unstick};
pub(crate) use attacks::attacked;
pub(crate) use blast::{Wreck, explode};
pub(crate) use enchanted::thorned;
pub(crate) use harm::{holds, is_full_block, touches, wound};
pub(crate) use health::{Cause, Health};
pub(crate) use hunting::{on_difficulty, sees};
pub(crate) use husbandry::{Raised, interact, kept as kept_raising, owner_hit, owner_hurt, shear_in, tame_by_riding, tie_to_fence};
pub(crate) use omens::{Omens, befall, infested_hurt};
pub(crate) use struck::struck_by_lightning;
pub(crate) use villagers::{bargains, interact as trade_with, stop_trading, traded};
pub(crate) use wither::{build as build_wither, hurt as wither_hurt, spares};
use crate::components::{InLayer, NetId, Tick};
use crate::{entities, mobs};


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
}

/// A mob that died at the tick it holds; it goes [`DYING`] ticks after.
#[derive(Component)]
struct Dying(u64);

/// Whether the entity is a mob that has not died.
pub(crate) fn is_alive(world: &World, entity: Entity) -> bool {
    world.get::<Creature>(entity).is_some() && world.get::<Dying>(entity).is_none()
}

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
    husbandry::tick(world);
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
        // A rider is carried to its seat, a vehicle its driver's client moves put where it says (riding.rs).
        // TODO: a riding mob neither thinks nor is harmed.
        let ridden = world.get::<crate::riding::Riding>(entity).is_some() || crate::riding::is_driven(world, entity);
        let Some(start) = world.get::<Body>(entity).copied().filter(|_| !ridden) else { continue };
        // With `sapling:mobAi` off a mob still falls, is pushed and is hurt, but decides nothing.
        let thinks = world.get::<Dying>(entity).is_none() && crate::rules::on(world, layer, sapling_data::MOB_AI);
        if thinks {
            stares::watch(world, entity, layer, id, &kind, &start);
            drinks::tick(world, entity, layer, id, &start, hunting::prey_at(world, entity));
            teleports::tick(world, entity, layer, hunting::prey_at(world, entity));
            if villagers::tick(world, entity, layer) {
                continue;
            }
        }
        // A teleport starts the tick afresh where it landed: no fall from where it left.
        let Some(start) = world.get::<Body>(entity).copied() else { continue };
        if thinks && !legs::ridden(world, entity, layer, &kind, &start) && !wither::steer(world, entity, layer) && !rams::steer(world, entity, layer, id, &kind) && !hunting::steer(world, entity, layer, id, &kind) && !husbandry::steer(world, entity, layer, &kind) && !villagers::steer(world, entity, layer)
            && !swimming::steer(world, entity, layer, &kind) && !squids::steer(world, entity) && !wings::steer(world, entity, layer, &kind)
        {
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
        entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), packets.clone());
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
            reshow(world, entity);
        }
    }
}
