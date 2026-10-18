//! The End as BDS's `TheEndGenerator` (constructor 1482414a0, vtable 14a924380, `loadChunk` 148242a00) makes it:
//! Java 1.12's End terrain, end cities and decoration on MT19937, one biome (docs/research/worldgen-bds-end.md).

mod spikes;
mod stage;
mod terrain;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::Dimension;
use crate::biome::ChunkBiomes;
use crate::cache::SourceCache;
use crate::chunk::ChunkShape;
use crate::feature::{Features, Inputs};
use crate::structure::end::EndStructures;
use crate::structured::order::Order;
use crate::structured::{Pieced, Source};

pub use spikes::{Spike, spikes};
pub use terrain::HEIGHT;

/// `minecraft:the_end`, every column at every y.
pub const THE_END: u8 = 9;
const BIOMES_LIMIT: usize = 64;
const TERRAIN_LIMIT: usize = 256;
/// Post-processes kept, and ten times as many chunk states (as the overworld's).
const PIECED_LIMIT: usize = 1024;

pub struct EndGenerator {
    seed: u32,
    noises: terrain::EndNoises,
    features: Features,
    structures: EndStructures,
    chunk_biomes: SourceCache<(i32, i32), ChunkBiomes>,
    terrain: SourceCache<(i32, i32), ChunkShape>,
    pieced: SourceCache<(i32, i32), Option<Source>>,
    staged: SourceCache<(i32, i32, u8), Pieced>,
    structure_order: Order,
    loaded: SourceCache<(i32, i32), ChunkShape>,
}

impl EndGenerator {
    /// Every MT takes the seed's low 32 bits.
    pub fn new(seed: i64) -> Self {
        Self {
            seed: seed as u32,
            noises: terrain::EndNoises::new(seed as u32),
            features: Features::for_dimension(seed, 1, Dimension::End),
            structures: EndStructures::new(seed),
            chunk_biomes: SourceCache::new(BIOMES_LIMIT),
            terrain: SourceCache::new(TERRAIN_LIMIT),
            pieced: SourceCache::new(PIECED_LIMIT),
            staged: SourceCache::new(10 * PIECED_LIMIT),
            structure_order: Order::default(),
            loaded: SourceCache::new(TERRAIN_LIMIT),
        }
    }

    /// End stone and air from the density alone.
    pub fn chunk_shape(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.noises.chunk_shape(chunk_x, chunk_z)
    }

    /// The buffer `loadChunk` commits (1482431de): the density's end stone. The surface builder `minecraft:the_end`
    /// (1487e4fd0) turns stone into end stone and finds none.
    pub fn terrain_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        (*self.terrain(chunk_x, chunk_z)).clone()
    }

    fn terrain(&self, chunk_x: i32, chunk_z: i32) -> Arc<ChunkShape> {
        self.terrain.get((chunk_x, chunk_z), || {
            let mut shape = self.chunk_shape(chunk_x, chunk_z);
            shape.share();
            shape
        })
    }

    /// Every ported stage: the structured chunk (`loaded_blocks`), then the decoration of every chunk that reaches it.
    pub fn chunk_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let mut shape = self.loaded_blocks(chunk_x, chunk_z);
        let loaded = |x, z| self.loaded(x, z);
        let terrain = |x, z| self.terrain(x, z);
        let biomes = |x, z| self.chunk_biomes(x, z);
        let tops = |x, z| Arc::new(self.terrain(x, z).tops());
        let surface = |_, _| crate::density::NO_SURFACE;
        let none = |_: [i32; 3]| false;
        let island = |x, z| self.island_height(x, z, 1, 1);
        let inputs = Inputs { loaded: &loaded, carved: &terrain, generated_tops: &tops, biomes: &biomes, surface: &surface, in_structure: &none, in_tunnels: &none, island: Some(&island) };
        self.features.place(&mut shape, (chunk_x, chunk_z), inputs);
        shape
    }

    /// Decorates in the order one BDS run did (`OverworldGenerator::set_decoration_order`).
    #[doc(hidden)]
    pub fn set_decoration_order(&mut self, order: &[(i32, i32)]) {
        self.features.set_order(order, None);
    }

    pub fn chunk_biomes(&self, chunk_x: i32, chunk_z: i32) -> Arc<ChunkBiomes> {
        self.chunk_biomes.get((chunk_x, chunk_z), || ChunkBiomes::from_columns((chunk_x, chunk_z), &[THE_END; 256]))
    }

    /// Java's `getIslandHeightValue` of a chunk's cell column (0..3 each way); decoration asks cell 1, 1 (148244960).
    pub fn island_height(&self, chunk_x: i32, chunk_z: i32, cell_x: i32, cell_z: i32) -> f32 {
        self.noises.island_height(chunk_x, chunk_z, cell_x, cell_z)
    }

    /// The obsidian spikes around the main island, in BDS's list order.
    pub fn spikes(&self) -> [Spike; 10] {
        spikes(self.seed)
    }
}
