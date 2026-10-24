//! A player's own movement policy, over the shard's (`ShardCtx::set_movement_policy`).

use bevy_ecs::prelude::*;
use sapling_api::{MovementBackend, MovementPolicy, PlayerId};

use super::Simulated;
use crate::components::Settings;
use crate::handlers::Ctx;

#[derive(Component, Clone, Copy)]
pub(crate) struct Own(pub MovementPolicy);

impl Own {
    /// The policy of a player with `own`, on a shard whose default is `default`.
    pub fn or(own: Option<&Self>, default: MovementPolicy) -> MovementPolicy {
        own.map_or(default, |own| own.0)
    }
}

impl MovementBackend for Ctx<'_> {
    fn movement_policy(&self, player: PlayerId) -> Option<MovementPolicy> {
        let entity = self.entity(player)?;
        Some(Own::or(self.world.get::<Own>(entity), self.world.resource::<Settings>().0.movement_check.policy))
    }

    fn set_movement_policy(&mut self, player: PlayerId, policy: Option<MovementPolicy>) -> bool {
        let Some(entity) = self.entity(player) else { return false };
        match policy {
            Some(policy) => drop(self.world.entity_mut(entity).insert(Own(policy))),
            None => drop(self.world.entity_mut(entity).remove::<Own>()),
        }
        if let Some(mut simulated) = self.world.get_mut::<Simulated>(entity) {
            simulated.restart();
        }
        true
    }
}
