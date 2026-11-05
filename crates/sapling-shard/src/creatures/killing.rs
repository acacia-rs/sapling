//! A mob killed outright (`ShardCtx::kill_entity`, `/kill`): hurt by all it has left, or gone at once.

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use bevy_ecs::prelude::*;
use sapling_api::EntityId;

use super::death::die;
use super::{Cause, Creature, Dying, Health, NO_DEATH, harm};
use crate::body::Body;
use crate::components::InLayer;
use crate::{entities, mobs};

/// Kills the entity `id` (see `ShardCtx::kill_entity`).
pub(super) fn kill(world: &mut World, id: EntityId) -> bool {
    let Some(entity) = entities::find(world, id).filter(|entity| world.get::<Dying>(*entity).is_none() && !crate::dragon::is_dying(world, *entity)) else { return false };
    let (Some(kind), Some(body), Some(layer)) = (world.get::<entities::Kind>(entity).map(|kind| kind.0.clone()), world.get::<Body>(entity).copied(), world.get::<InLayer>(entity).map(|layer| layer.0)) else { return false };
    // A killed boat or minecart leaves its item (research/captures/bds-i2rails.txt).
    if crate::vehicles::is_vehicle(&kind) {
        crate::vehicles::wreck(world, entity, true);
        return true;
    }
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
