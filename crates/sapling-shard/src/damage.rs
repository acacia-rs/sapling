//! Harm offered to plugins before it lands ([`DamageEvent`]): a cancelled hit does nothing, a
//! changed one lands as changed. Where each kind of harm asks: vitals.rs for players,
//! creatures/harm.rs, attacks.rs and blast.rs for mobs.

use bevy_ecs::prelude::*;
use sapling_api::{Attacker, DamageEvent, EntityId, Target};

use crate::components::{Identity, NetId};
use crate::creatures::Cause;
use crate::plugins;

/// What plugins make of a hit of `amount` by `cause` on a player or a mob; `None` where they
/// cancel it.
pub(crate) fn ask(world: &mut World, victim: Entity, amount: f32, cause: Cause, attacker: Option<Attacker>) -> Option<f32> {
    let target = match world.get::<Identity>(victim) {
        Some(identity) => Target::Player(identity.0.clone()),
        None => Target::Entity(EntityId(world.get::<NetId>(victim)?.0)),
    };
    let event = plugins::offer(world, DamageEvent::new(target, cause, attacker, amount));
    (!event.is_cancelled()).then_some(event.amount.max(0.0))
}
