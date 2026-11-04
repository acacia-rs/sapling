//! A llama's strength (ED `llama.json` and `trader_llama.json`, `minecraft:add_attributes`): 1, 2
//! or 3 at 32 in 100 each, 4 or 5 at 2 in 100, of at most 5. Its chest holds 3 slots a point
//! (`additional_slots_per_strength`). Kept as `Strength` and `StrengthMax` (Actor::load
//! 140d48140) and shown as the `Strength`, `MaxStrength` and `ContainerExtraSlotsPerStrength`
//! metadata, in its announcement already (research/captures/bds-mountslots.txt).

use acacia_proto::packets::AddEntity;
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;

use super::packs::PER_STRENGTH;
use crate::blocks::Dice;
use crate::entities::Stored;

const KINDS: [&str; 2] = ["minecraft:llama", "minecraft:trader_llama"];
const WEIGHTS: [u32; 5] = [32, 32, 32, 2, 2];
const MAX: i32 = 5;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(super) struct Strength(pub i32);

/// The strength a weighted draw below 100 picks.
fn picked(mut draw: u32) -> i32 {
    for (strength, weight) in (1..).zip(WEIGHTS) {
        if draw < weight {
            return strength;
        }
        draw -= weight;
    }
    MAX
}

/// A strength drawn for a new `kind`; none for a mob that is no llama.
pub(crate) fn roll(world: &mut World, kind: &str) -> Option<i32> {
    KINDS.contains(&kind).then(|| picked(world.get_resource_or_insert_with(Dice::default).roll(WEIGHTS.iter().sum())))
}

/// The strength a stored llama's `compound` kept.
pub(crate) fn kept(compound: &[u8]) -> Option<i32> {
    sapling_world::actor_strength(compound).map(|(strength, _)| strength.clamp(1, MAX))
}

/// Gives a llama the strength its stored compound kept, else `rolled` (or one rolled now), kept
/// there; false for a mob that is no llama.
pub(crate) fn give(world: &mut World, mob: Entity, kind: &str, rolled: Option<i32>) -> bool {
    if !KINDS.contains(&kind) {
        return false;
    }
    let kept = world.get::<Stored>(mob).and_then(|stored| kept(&stored.compound));
    let strength = kept.or(rolled).or_else(|| roll(world, kind)).unwrap_or(1);
    world.entity_mut(mob).insert(Strength(strength.clamp(1, MAX)));
    if let Some(mut stored) = world.get_mut::<Stored>(mob).filter(|_| kept.is_none())
        && let Ok(compound) = sapling_world::strong_actor(&stored.compound, strength, MAX)
    {
        stored.compound = compound;
    }
    true
}

/// How strong `mob` is; none for a mob that is no llama.
pub(crate) fn of(world: &World, mob: Entity) -> Option<i32> {
    world.get::<Strength>(mob).map(|strength| strength.0)
}

/// Shows `strength` in a llama's announcement over the recorded llama's.
pub(crate) fn dress(added: &mut AddEntity, strength: i32) {
    for item in &mut added.metadata {
        let value = match item.key {
            Key::Strength => strength,
            Key::MaxStrength => MAX,
            Key::ContainerExtraSlotsPerStrength => PER_STRENGTH as i32,
            _ => continue,
        };
        item.value = Value::Default(Plain::Int(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strength_is_drawn_by_the_packs_weights() {
        assert_eq!([0, 31, 32, 95, 96, 97, 98, 99].map(picked), [1, 1, 2, 3, 4, 4, 5, 5]);
    }
}
