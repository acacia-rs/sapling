//! One source chunk's feature rules in BDS's order: `decorateWorldGenPostProcess` (14821f400) runs
//! the passes, `BiomeDecorationSystem::decorate` (14580bb90) the chunk's biomes in palette order,
//! `decorateBiome` (14580b170) each biome's rules for the pass.

use std::sync::Arc;

use super::data::{FEATURES, RULES};
use super::molang::{Molang, Scope, World};
use super::place::Ctx;
use super::scatter::{self, Draws};
use super::spec::{Pass, Rule};
use super::view::View;
use crate::biome::ChunkBiomes;
use crate::random::{LargeFeatureSeed, Mt19937};
use crate::terrain::{HEIGHT, MIN_Y};

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

/// What every rule of one source chunk shares.
struct Source<'a> {
    chunk: (i32, i32),
    chunk_seed: u32,
    /// The chunk's own generator, seeded with the chunk seed: rules' `math.random` draws from it.
    random: Mt19937,
    biomes: &'a dyn Fn(i32, i32) -> Arc<ChunkBiomes>,
}

pub fn decorate_chunk(seed: &LargeFeatureSeed, chunk: (i32, i32), biomes: &dyn Fn(i32, i32) -> Arc<ChunkBiomes>, view: &mut View) {
    let chunk_seed = seed.chunk(chunk.0, chunk.1);
    let own = biomes(chunk.0, chunk.1);
    let order = palette_order(&own);
    let mut source = Source { chunk, chunk_seed, random: Mt19937::new(chunk_seed), biomes };
    let mut scope = Scope::new();
    for pass in Pass::ALL {
        for &biome in &order {
            // With one biome BDS places every position without looking up its biome.
            let only = (order.len() > 1).then_some(biome);
            for rule in RULES.iter().filter(|rule| rule.pass == pass && rule.biomes.contains(biome)) {
                run(rule, &mut source, only, &own, view, &mut scope);
            }
        }
    }
}

/// The positions come from one generator, the features draw from a second seeded alike.
fn run(rule: &Rule, source: &mut Source, only: Option<u32>, own: &ChunkBiomes, view: &mut View, scope: &mut Scope) {
    let seed = rule_seed(source.chunk_seed, rule.name);
    let mut positions = Mt19937::new(seed);
    let origin = [source.chunk.0 * 16, 0, source.chunk.1 * 16];
    scope.set_origin(origin);
    scope.world_bound = false;
    let queries = scope.queries;
    let count = {
        let host = World { view, queries };
        let draws = &mut Draws { positions: &mut positions, molang: Some(&mut source.random) };
        scatter::iterations(&rule.distribution, draws, &mut Molang { scope, host: &host })
    };
    if count == 0 {
        return;
    }
    let mut placing = Mt19937::new(seed);
    let biomes = source.biomes;
    let biome = |[x, y, z]: [i32; 3]| biome_at(&biomes(x.div_euclid(16), z.div_euclid(16)), [x, y, z]);
    for index in (0..count).rev() {
        let at = {
            let host = World { view, queries };
            let draws = &mut Draws { positions: &mut positions, molang: Some(&mut source.random) };
            scatter::position(&rule.distribution, index, origin, draws, &mut Molang { scope, host: &host })
        };
        if only.is_some_and(|wanted| in_chunk(source.chunk, at).then(|| biome_at(own, at)).flatten() != Some(wanted)) {
            continue;
        }
        let ctx = &mut Ctx { view, random: &mut placing, scope, biome: &biome };
        FEATURES[rule.feature].place(ctx, at);
    }
}

fn in_chunk(chunk: (i32, i32), [x, _, z]: [i32; 3]) -> bool {
    x.div_euclid(16) == chunk.0 && z.div_euclid(16) == chunk.1
}

/// None outside the world's height: such positions match no biome (oracle: diamond and redstone
/// rules scatter below -64), so they never place nor draw.
fn biome_at(biomes: &ChunkBiomes, [x, y, z]: [i32; 3]) -> Option<u32> {
    if !(MIN_Y..MIN_Y + HEIGHT as i32).contains(&y) {
        return None;
    }
    Some(biomes.biome(x.rem_euclid(16) as usize, y, z.rem_euclid(16) as usize))
}

/// The chunk's biomes as BDS lists them (1438c8450): sub-chunk palettes bottom up, first seen first.
/// Inside a sub-chunk the palette fills in storage order, x then z then y (1438cb860).
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
