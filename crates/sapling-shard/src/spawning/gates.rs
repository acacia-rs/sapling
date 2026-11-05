//! What BDS's code asks of a spot besides a kind's pack rules: the placement `ActorSpawnRuleGroup`
//! installs for `minecraft:slime` (docs/research/mob-spawning.md "Slimes"), and each spawned mob's
//! own `checkSpawnRules` (actor vtable +0x488; mob-spawning.md "Own spawn checks").

use bevy_ecs::prelude::*;
use sapling_world::{ChunkPos, NETHER, World as Level};
use sapling_worldgen::random::Mt19937;

use super::SpawnDice;
use super::spots::Found;
use crate::body::Body;
use crate::components::{InLayer, Layer};
use crate::creatures::Creature;

const SLIME: &str = "minecraft:slime";
const PIGMAN: &str = "minecraft:zombie_pigman";
const GUARDIANS: [&str; 2] = ["minecraft:guardian", "minecraft:elder_guardian"];
/// Monsters that skip Monster's darkness test: their class overrides its check, or its
/// `isDarkEnoughToSpawn` (vtable +0x570) with one returning true (docs/research/mob-spawning.md "Own spawn checks").
const OWN_CHECKS: [&str; 12] = [
    SLIME, PIGMAN, "minecraft:silverfish", "minecraft:endermite", "minecraft:shulker", "minecraft:phantom", "minecraft:pillager", "minecraft:blaze", "minecraft:piglin", "minecraft:piglin_brute",
    "minecraft:ghast", "minecraft:magma_cube",
];
const SEA_LEVEL: i32 = 63;
/// Below this a slime appears only in a slime chunk.
const SLIME_CHUNK_TOP: i32 = 39;
/// The levels swamp slimes appear at.
const SWAMP: std::ops::Range<i32> = 50..69;

/// Whether `kind` may appear at `found` by BDS's code; rolls natural spawning's dice for a swamp slime.
pub(super) fn admits(world: &mut World, layer: Entity, kind: &str, found: &Found) -> bool {
    match kind {
        SLIME => slime(world, layer, found),
        _ => true,
    }
}

fn slime(world: &mut World, layer: Entity, found: &Found) -> bool {
    let [x, y, z] = found.cell;
    if y < SLIME_CHUNK_TOP {
        return is_slime_chunk(ChunkPos::of_block(x, z));
    }
    if !SWAMP.contains(&y) || found.brightness >= 8 {
        return false;
    }
    let dice = &mut world.get_resource_or_insert_with(SpawnDice::default).0;
    if u32::from(found.brightness) > dice.roll(8) {
        return false;
    }
    let roll = dice.fraction();
    let Some(time) = world.get::<Layer>(layer).map(|layer| layer.world.level().time) else { return false };
    roll < sapling_data::moon_fullness(time) && sapling_data::biome_has_tag(found.biome, "swamp")
}

/// Whether a mob of `kind` with its feet `at` in `cell` passes its class's own spawn check: a
/// zombie pigman's (1429904c0), a guardian's (1459a02d0), or Monster's darkness (14298f930).
pub(super) fn own_check(world: &mut World, layer: Entity, kind: &str, at: [f32; 3], cell: [i32; 3]) -> bool {
    if kind == PIGMAN {
        return unobstructed(world, layer, kind, at);
    }
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    if GUARDIANS.contains(&kind) {
        let covered = !sees_sky_from_below_water(level, cell);
        return covered || world.get_resource_or_insert_with(SpawnDice::default).0.roll(20) == 0;
    }
    let category = sapling_data::spawn_category(kind).or_else(|| super::structures::category(kind));
    if category != Some("monster") || OWN_CHECKS.contains(&kind) || level.dimension() == NETHER {
        return true;
    }
    let [x, y, z] = cell;
    level.light_at(x, y, z).is_some_and(|(_, block)| block == 0)
}

/// Nothing in the mob's box: no block's collision box and no mob (every mob obstructs spawning, `Mob::initializeComponents`).
fn unobstructed(world: &mut World, layer: Entity, kind: &str, at: [f32; 3]) -> bool {
    let Some(size) = sapling_data::mob_size(kind) else { return false };
    let body = Body::new(at, [0.0; 3], size);
    let (min, max) = body.bounds();
    let overlaps = |other: &Body| {
        let (low, high) = other.bounds();
        (0..3).all(|axis| min[axis] < high[axis] && low[axis] < max[axis])
    };
    let near: Vec<Entity> = world.query_filtered::<(Entity, &InLayer, &Body), With<Creature>>().iter(world).filter(|(_, in_layer, other)| in_layer.0 == layer && overlaps(other)).map(|(entity, ..)| entity).collect();
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    near.iter().all(|entity| crate::creatures::is_dying(world, *entity)) && body.is_free(level)
}

/// Java's `canSeeSkyFromBelowWater`: open sky above, or below sea level only liquids and
/// see-through blocks between the cell and the sea's surface.
fn sees_sky_from_below_water(level: &Level, [x, y, z]: [i32; 3]) -> bool {
    if y >= SEA_LEVEL {
        return level.top_solid(x, z).is_none_or(|top| y > top);
    }
    let registry = level.registry();
    (y + 1..SEA_LEVEL).all(|y| registry.get(level.block(x, y, z)).is_none_or(|state| !state.is_solid() || state.is_liquid()))
}

/// BDS's slime chunks: one in ten, by an MT19937 seeded from the chunk alone, whatever the world's seed.
fn is_slime_chunk(chunk: ChunkPos) -> bool {
    let seed = (chunk.x as u32).wrapping_mul(0x1f1f_1f1f) ^ chunk.z as u32;
    Mt19937::new(seed).next_u32() % 10 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slime_chunks_are_one_in_ten_by_the_chunk_seed() {
        assert_eq!(Mt19937::new(0).next_u32(), 2_357_136_044, "the reference MT19937 seeded 0");
        assert!(!is_slime_chunk(ChunkPos::new(0, 0)));
        let found = (-50..50).flat_map(|x| (-50..50).map(move |z| ChunkPos::new(x, z))).filter(|&chunk| is_slime_chunk(chunk)).count();
        assert!((900..1100).contains(&found), "{found} of 10000");
    }
}
