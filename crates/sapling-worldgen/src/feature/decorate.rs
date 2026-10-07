//! One source chunk's feature rules in BDS's order: `decorateWorldGenPostProcess` (14821f400) runs
//! the passes, `BiomeDecorationSystem::decorate` (14580bb90) the chunk's biomes in palette order,
//! `decorateBiome` (14580b170) each biome's rules for the pass.

use acacia_molang::Host;

use super::data::{FEATURES, RULES};
use super::molang::{Molang, Scope};
use super::scatter;
use super::spec::{Feature, Pass, Rule};
use super::view::View;
use crate::biome::ChunkBiomes;
use crate::random::{LargeFeatureSeed, Mt19937};
use crate::terrain::{HEIGHT, MIN_Y};

impl Host for View<'_> {}

impl Feature {
    /// Whether the feature placed anything.
    pub fn place(&self, view: &mut View, at: [i32; 3], random: &mut Mt19937) -> bool {
        match self {
            Feature::Ore(ore) => ore.place(view, at, random),
        }
    }
}

/// Bedrock's `HashedString` hash: 64-bit FNV-1 (multiply, then xor), 0 for "".
const fn hashed_string(name: &str) -> u64 {
    let bytes = name.as_bytes();
    if bytes.is_empty() {
        return 0;
    }
    let mut hash = 0xCBF2_9CE4_8422_2325u64;
    let mut i = 0;
    while i < bytes.len() {
        hash = hash.wrapping_mul(0x100_0000_01B3) ^ bytes[i] as u64;
        i += 1;
    }
    hash
}

/// The rule's generator seed: boost's `hash_combine` of the chunk seed and the low 32 bits of the
/// rule name's hash.
fn rule_seed(chunk_seed: u32, name: &str) -> u32 {
    let hash = hashed_string(name) as u32;
    chunk_seed ^ hash.wrapping_add(0x9E37_79B9).wrapping_add(chunk_seed << 6).wrapping_add(chunk_seed >> 2)
}

pub fn decorate_chunk(seed: &LargeFeatureSeed, (chunk_x, chunk_z): (i32, i32), biomes: &ChunkBiomes, view: &mut View) {
    let chunk_seed = seed.chunk(chunk_x, chunk_z);
    let order = palette_order(biomes);
    let mut scope = Scope::new();
    for pass in Pass::ALL {
        for &biome in &order {
            // With one biome BDS places every position without looking up its biome.
            let only = (order.len() > 1).then_some(biome);
            for rule in RULES.iter().filter(|rule| rule.pass == pass && rule.biomes.contains(biome)) {
                run(rule, chunk_seed, (chunk_x, chunk_z), only, biomes, view, &mut scope);
            }
        }
    }
}

/// The positions come from one generator, the features draw from a second seeded alike.
fn run(rule: &Rule, chunk_seed: u32, chunk: (i32, i32), only: Option<u32>, biomes: &ChunkBiomes, view: &mut View, scope: &mut Scope) {
    let seed = rule_seed(chunk_seed, rule.name);
    let mut positions = Mt19937::new(seed);
    let origin = [chunk.0 * 16, 0, chunk.1 * 16];
    scope.set_origin(origin);
    let count = scatter::iterations(&rule.distribution, &mut positions, &mut Molang { scope, host: view });
    if count == 0 {
        return;
    }
    let mut placing = Mt19937::new(seed);
    for index in (0..count).rev() {
        let at = scatter::position(&rule.distribution, index, origin, &mut positions, &mut Molang { scope, host: view });
        if only.is_some_and(|biome| biome_at(biomes, chunk, at) != Some(biome)) {
            continue;
        }
        FEATURES[rule.feature].place(view, at, &mut placing);
    }
}

// TODO: BDS's answer above and below the world (rules scatter up to y 480); clamped until known.
fn biome_at(biomes: &ChunkBiomes, chunk: (i32, i32), [x, y, z]: [i32; 3]) -> Option<u32> {
    let (lx, lz) = (x - chunk.0 * 16, z - chunk.1 * 16);
    if !(0..16).contains(&lx) || !(0..16).contains(&lz) {
        return None;
    }
    Some(biomes.biome(lx as usize, y.clamp(MIN_Y, MIN_Y + HEIGHT as i32 - 1), lz as usize))
}

/// The chunk's biomes as BDS lists them (1438c8450): sub-chunk palettes bottom up, first seen first.
// TODO: palette insertion order inside a sub-chunk follows fillBiomes' write order; x, z, y assumed.
fn palette_order(biomes: &ChunkBiomes) -> Vec<u32> {
    let mut order = Vec::new();
    for base in (MIN_Y..MIN_Y + HEIGHT as i32).step_by(16) {
        for x in 0..16 {
            for z in 0..16 {
                for y in base..base + 16 {
                    let biome = biomes.biome(x, y, z);
                    if !order.contains(&biome) {
                        order.push(biome);
                    }
                }
            }
        }
    }
    order
}

#[cfg(test)]
mod tests {
    #[test]
    fn hashed_string_is_fnv1() {
        assert_eq!(super::hashed_string(""), 0);
        // FNV-1 64 of "a": (offset * prime) ^ 0x61.
        assert_eq!(super::hashed_string("a"), 0xAF63_BD4C_8601_B7BE);
    }
}
