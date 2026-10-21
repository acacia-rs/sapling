//! Harm offered to plugins before it lands ([`DamageEvent`]): a cancelled hit does nothing, a
//! changed one lands as changed. Where each kind of harm asks: vitals.rs and vitals/stand.rs for
//! players, creatures/harm.rs, afflictions.rs, attacks.rs and blast.rs for mobs, and `kill` for
//! both.

use bevy_ecs::prelude::*;
use sapling_api::{Attacker, DamageEvent, EntityId, Target};

use crate::components::{Identity, NetId};
use crate::creatures::{Cause, Health};
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

/// [`ask`] about what `health` lost by `cause` since `before`, a hit already taken, and puts
/// back what plugins spare; whether a loss stands.
pub(crate) fn weigh(world: &mut World, victim: Entity, health: &mut Health, before: &Health, cause: Cause) -> bool {
    let lost = health.lost_since(before);
    let asked = ask(world, victim, lost, cause, None);
    if asked != Some(lost) {
        health.reweigh(before, asked);
    }
    asked.is_some_and(|amount| amount > 0.0)
}
