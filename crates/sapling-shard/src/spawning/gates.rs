//! What BDS's code asks of a spot besides a kind's pack rules: the placement `ActorSpawnRuleGroup`
//! installs for `minecraft:slime` (docs/research/mob-spawning.md "Slimes").

use bevy_ecs::prelude::*;
use sapling_world::ChunkPos;
use sapling_worldgen::random::Mt19937;

use super::SpawnDice;
use super::spots::Found;
use crate::components::Layer;

const SLIME: &str = "minecraft:slime";
/// Below this a slime appears only in a slime chunk.
const SLIME_CHUNK_TOP: i32 = 39;
/// The levels swamp slimes appear at.
const SWAMP: std::ops::Range<i32> = 50..69;
/// How full the moon is on each day of its cycle, swamp slimes' odds (bds-blocks `14a75c000`).
const MOON: [f32; 8] = [1.0, 0.75, 0.5, 0.25, 0.0, 0.25, 0.5, 0.75];

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
    roll < MOON[(time / 24_000).unsigned_abs() as usize % MOON.len()] && sapling_data::biome_has_tag(found.biome, "swamp")
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
