//! Tadpoles and frogs (docs/research/mob-life.md "Frogspawn"): a tadpole is a baby from the
//! start, and once its `minecraft:ageable` 1200 seconds are up it becomes a frog where it swam
//! (`minecraft:transformation`, heard as `convert_to_frog`), keeping its name. A new frog takes its
//! biome's variant as its pack's `entity_spawned` gives it: cold over warm over temperate.

use bevy_ecs::prelude::*;

use super::super::genes::Gene;
use super::{Raised, clicks, shown};

const TADPOLE: &str = "minecraft:tadpole";
const FROG: &str = "minecraft:frog";
const TEMPERATE: i32 = 0;
const COLD: i32 = 1;
const WARM: i32 = 2;
/// Each a biome's tags all of which pick the variant, in the pack's order.
const WARM_TAGS: &[&[&str]] = &[&["desert"], &["jungle"], &["savanna"], &["mesa"], &["nether"], &["warm", "ocean"], &["lukewarm", "ocean"], &["mangrove_swamp"]];
const COLD_TAGS: &[&[&str]] = &[&["mountain"], &["ice"], &["cold"], &["frozen"], &["the_end"], &["deep_dark"]];
/// `mountains` picks cold but for these.
const MILD_MOUNTAINS: [&str; 3] = ["meadow", "cherry_grove", "stony_peaks"];

/// The variant a new frog at `feet` in `layer` takes from its biome; `None` for another kind.
pub(in crate::creatures) fn frog_variant(world: &World, layer: Entity, kind: &str, feet: [f32; 3]) -> Option<Gene> {
    if kind != FROG {
        return None;
    }
    let [x, y, z] = feet.map(|c| c.floor() as i32);
    let biome = world.get::<crate::components::Layer>(layer).and_then(|layer| layer.world.biome(x, y, z));
    let has = |tag: &str| biome.is_some_and(|biome| sapling_data::biome_has_tag(biome, tag));
    let any = |sets: &[&[&str]]| sets.iter().any(|tags| tags.iter().all(|tag| has(tag)));
    let cold = any(COLD_TAGS) || has("mountains") && !MILD_MOUNTAINS.iter().any(|tag| has(tag));
    Some(Gene(if cold { COLD } else if any(WARM_TAGS) { WARM } else { TEMPERATE }))
}

/// A newcomer tadpole is a baby with its whole growing ahead of it.
pub(super) fn newcomer(raised: &mut Raised, kind: &str, grows: u32) {
    if kind == TADPOLE {
        raised.kept.baby = Some(grows as i32);
    }
}

/// The grown tadpole `entity` becomes a frog; false for another kind.
pub(super) fn grow_up(world: &mut World, entity: Entity, kind: &str) -> bool {
    if kind != TADPOLE {
        return false;
    }
    let Some((layer, _, _, body)) = shown::of(world, entity) else { return true };
    let name = world.get::<Raised>(entity).map(|raised| raised.kept.name.clone()).unwrap_or_default();
    shown::sound(world, entity, "convert_to_frog");
    if let Some(frog) = super::super::spawn_entity(world, layer, FROG, body.at, sapling_api::SpawnReason::Transformed)
        && !name.is_empty()
    {
        clicks::name(world, frog, name);
    }
    crate::mobs::remove(world, entity);
    true
}
