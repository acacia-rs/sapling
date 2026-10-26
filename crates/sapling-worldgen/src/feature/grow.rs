//! One feature placed by name outside generation, as a sapling places its tree
//! (docs/research/saplings.md).

use std::sync::Arc;

use super::data::{FEATURES, NAMES};
use super::molang::Scope;
use super::place::Ctx;
use super::view::View;
use crate::block::Block;
use crate::density::NO_SURFACE;
use crate::random::Mt19937;
use crate::terrain::ChunkShape;

/// Whether `name` (`minecraft:oak_tree_feature`) is a ported feature [`place_feature`] knows.
pub fn is_feature(name: &str) -> bool {
    NAMES.contains(&name)
}

/// Places the feature `name` (`minecraft:oak_tree_feature`) at `at` with a generator seeded
/// `seed`, over the chunks `load` gives (those within a chunk of `at`'s): its writes as world
/// (x, y, z), block placed, block replaced. `None` for a name not ported or a feature that did not place.
// TODO: heightmaps read during the placement are the loaded chunks' own, scanned, not the world's.
pub fn place_feature(name: &str, at: [i32; 3], seed: u32, load: &dyn Fn(i32, i32) -> Arc<ChunkShape>, biome: &dyn Fn([i32; 3]) -> Option<u32>) -> Option<Vec<([i32; 3], Block, Block)>> {
    let feature = &FEATURES[NAMES.iter().position(|known| *known == name)?];
    let chunk = (at[0].div_euclid(16), at[2].div_euclid(16));
    let mut view = View::new(chunk, load);
    let (mut random, mut scope) = (Mt19937::new(seed), Scope::new());
    scope.set_origin(at);
    let ctx = &mut Ctx { view: &mut view, random: &mut random, scope: &mut scope, biome, surface: &|_, _| NO_SURFACE, in_structure: &|_| false, in_tunnels: &|_| false };
    feature.place(ctx, at)?;
    Some(view.into_writes().world_writes(chunk))
}
