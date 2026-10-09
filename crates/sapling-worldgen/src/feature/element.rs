//! Features that structure pieces place (`FeaturePoolElement`: legacy place 145e141d0 with the
//! chunk's structure generator, v2 place 145e144c0 with its own), through the post-process's 3x3
//! block source.

use std::sync::Arc;

use super::Load;
use super::data::{ELEMENTS, FEATURES};
use super::molang::Scope;
use super::place::Ctx;
use super::view::View;
use crate::block::Block;
use crate::random::Mt19937;
use crate::terrain::ChunkShape;

/// What an element's feature reads besides blocks.
pub struct ElementWorld<'a> {
    /// The neighbours' terrain as the structure stage sees it.
    pub load: Load<'a>,
    pub biome: &'a dyn Fn([i32; 3]) -> Option<u32>,
    pub surface: &'a dyn Fn(i32, i32) -> i32,
}

/// Places the feature `name` at `at` from `chunk`'s post-process, the chunk itself being `centre`;
/// returns its writes as (position, block, block replaced), or None for a feature not ported.
pub fn place_element(name: &str, chunk: (i32, i32), centre: Arc<ChunkShape>, world: &ElementWorld, random: &mut Mt19937, at: [i32; 3]) -> Option<Vec<([i32; 3], Block, Block)>> {
    let name = name.trim_start_matches("minecraft:");
    let &(_, index) = ELEMENTS.iter().find(|(n, _)| *n == name)?;
    let load = |x: i32, z: i32| if (x, z) == chunk { centre.clone() } else { (world.load)(x, z) };
    // The structure stage never recomputes heightmaps: every chunk's is its carved terrain's.
    let mut view = View::new(chunk, &load).with_frozen_heights(world.load);
    let mut scope = Scope::new();
    scope.set_origin(at);
    let ctx = &mut Ctx { view: &mut view, random, scope: &mut scope, biome: world.biome, surface: world.surface, in_structure: &|_| false };
    FEATURES[index].place(ctx, at);
    Some(view.into_writes().world_writes(chunk))
}

/// The v2 element's generator: an MT seeded with Java's `Mth.getSeed` of the position (x times
/// its constant in 64 bits), shifted down 16.
pub fn element_random([x, y, z]: [i32; 3]) -> Mt19937 {
    Mt19937::new(crate::random::wide_position_seed(x, y, z) as u32)
}
