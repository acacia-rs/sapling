//! A slime's or a magma cube's size, its `Variant`: 1, 2 or 4, each as likely, as its pack's
//! `minecraft:entity_spawned` rolls it, or the one its `spawn_small` and like events name. Each
//! size has its own box, health, pace and touch.

use acacia_proto::packets::AddEntity;
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use bevy_ecs::prelude::*;

use super::super::genes::Gene;
use crate::blocks::Dice;

/// Of each kind, by size: its box, health, pace, what its touch costs (none for a small slime's),
/// and its hop's speed across, as told once the hop's tick has slowed it, and up.
pub(super) struct Size {
    pub(super) size: [f32; 2],
    health: f32,
    pub(super) pace: f32,
    pub(super) touch: f32,
    pub(super) hop: f32,
    pub(super) rise: f32,
}

const fn size(width: f32, height: f32, health: f32, pace: f32, touch: f32, hop: f32, rise: f32) -> Size {
    Size { size: [width, height], health, pace, touch, hop, rise }
}

/// The packs' `slime_small`, `slime_medium` and `slime_large` groups. Hops as BDS's told them
/// (bds-kinds2.txt, bds-player-omens.txt, bds-effects3.txt): a slime's not by its pace, and a magma
/// cube's 0.1 higher a size. No large slime was drilled: its hop is a medium one's.
const SLIME: [Size; 3] = [
    size(0.52, 0.52, 1.0, 0.3, 0.0, 0.0482, 0.42),
    size(1.04, 1.04, 4.0, 0.4, 2.0, 0.0856, 0.42),
    size(2.08, 2.08, 16.0, 0.6, 4.0, 0.0856, 0.42),
];
const MAGMA_CUBE: [Size; 3] = [
    size(0.52, 0.52, 1.0, 0.6, 3.0, 0.1926, 0.52),
    size(1.04, 1.02, 4.0, 0.66, 4.0, 0.2984, 0.62),
    size(2.08, 2.08, 16.0, 0.75, 6.0, 0.301, 0.82),
];
pub(super) const KINDS: [&str; 2] = ["minecraft:slime", "minecraft:magma_cube"];
const VARIANTS: [i32; 3] = [1, 2, 4];
const EVENTS: [&str; 3] = ["spawn_small", "spawn_medium", "spawn_large"];

/// What a mob of `kind` and `gene` is of its size, if its kind has sizes.
pub(super) fn of(kind: &str, gene: Option<Gene>) -> Option<&'static Size> {
    let sizes = match kind {
        "minecraft:slime" => &SLIME,
        "minecraft:magma_cube" => &MAGMA_CUBE,
        _ => return None,
    };
    let at = VARIANTS.iter().position(|&variant| Some(variant) == gene.map(|gene| gene.0))?;
    Some(&sizes[at])
}

/// A new mob of `kind`'s size, if its kind has sizes.
pub(in super::super) fn roll(world: &mut World, kind: &str) -> Option<Gene> {
    KINDS.contains(&kind).then(|| Gene(VARIANTS[world.get_resource_or_insert_with(Dice::default).roll(3) as usize]))
}

/// The size a spawn `event` gives a mob of `kind`.
pub(in super::super) fn of_event(kind: &str, event: &str) -> Option<Gene> {
    let at = EVENTS.iter().position(|name| event.strip_prefix("minecraft:").unwrap_or(event) == *name).filter(|_| KINDS.contains(&kind))?;
    Some(Gene(VARIANTS[at]))
}

/// A stored mob of `kind`'s size, by the `Variant` it was stored with.
pub(in super::super) fn stored(kind: &str, variant: i32) -> Option<Gene> {
    (KINDS.contains(&kind) && VARIANTS.contains(&variant)).then_some(Gene(variant))
}

/// Shows `added` at its size: its box, health and pace, where BDS announced its kind's largest.
pub(in super::super) fn dress(added: &mut AddEntity, kind: &str, gene: Option<Gene>) {
    let Some(size) = of(kind, gene) else { return };
    for item in &mut added.metadata {
        match item.key {
            Key::BoundingboxWidth => item.value = Value::Default(Plain::Float(size.size[0])),
            Key::BoundingboxHeight => item.value = Value::Default(Plain::Float(size.size[1])),
            _ => {}
        }
    }
    for attribute in &mut added.attributes {
        match attribute.name.as_str() {
            "minecraft:health" => (attribute.value, attribute.max) = (attribute.value.min(size.health), size.health),
            "minecraft:movement" => attribute.value = size.pace,
            _ => {}
        }
    }
}

/// A mob of `kind` and `gene`'s full health, if its kind has sizes.
pub(in super::super) fn health(kind: &str, gene: Option<Gene>) -> Option<f32> {
    of(kind, gene).map(|size| size.health)
}

/// A mob of `kind` and `gene`'s box, if its kind has sizes.
pub(in super::super) fn body_size(kind: &str, gene: Option<Gene>) -> Option<[f32; 2]> {
    of(kind, gene).map(|size| size.size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spawn_event_names_a_size_and_a_stored_variant_keeps_it() {
        assert_eq!(["spawn_small", "minecraft:spawn_medium", "spawn_large", "spawn_huge"].map(|event| of_event("minecraft:slime", event).map(|gene| gene.0)), [Some(1), Some(2), Some(4), None]);
        assert!(of_event("minecraft:cow", "spawn_small").is_none());
        assert_eq!([0, 2, 3].map(|variant| stored("minecraft:magma_cube", variant).map(|gene| gene.0)), [None, Some(2), None]);
        assert_eq!(body_size("minecraft:magma_cube", Some(Gene(2))), Some([1.04, 1.02]));
    }
}
