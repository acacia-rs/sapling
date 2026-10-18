//! The Nether as BDS's `NetherGenerator` (constructor 1481e6d60, vtable 14a923740, `loadChunk` 1481ec000)
//! makes it: Java 1.12's hell terrain and a legacy layer biome stack, both on MT19937, then the bedrock,
//! the biomes' surface builders and the cave carver, then decoration (docs/research/worldgen-bds-nether.md).

mod biome;
mod capped;
mod carver;
mod floor;
mod stage;
mod surface;
mod surface_spec;
mod terrain;

use std::sync::Arc;

use crate::Dimension;
use crate::biome::ChunkBiomes;
use crate::cache::SourceCache;
use crate::chunk::ChunkShape;
use crate::feature::{Features, Inputs};
use crate::random::Mt19937;
use crate::structure::nether::NetherStructures;
use crate::structured::order::Order;
use crate::structured::{Pieced, Source};

pub use biome::{BASALT_DELTAS, CRIMSON_FOREST, HELL, SOULSAND_VALLEY, WARPED_FOREST};
pub use terrain::{HEIGHT, SEA_LEVEL};

/// Chunk biomes and carved chunks (about 200 and 100 kB each) kept for the neighbours that read them.
const BIOMES_LIMIT: usize = 256;
const CARVED_LIMIT: usize = 256;
/// Post-processes kept, and ten times as many chunk states (as the overworld's).
const PIECED_LIMIT: usize = 1024;

pub struct NetherGenerator {
    noises: terrain::NetherNoises,
    biomes: biome::NetherBiomes,
    surface: surface::NetherSurface,
    carver: carver::NetherCarver,
    features: Features,
    structures: NetherStructures,
    chunk_biomes: SourceCache<(i32, i32), ChunkBiomes>,
    carved: SourceCache<(i32, i32), ChunkShape>,
    pieced: SourceCache<(i32, i32), Option<Source>>,
    staged: SourceCache<(i32, i32, u8), Pieced>,
    structure_order: Order,
    loaded: SourceCache<(i32, i32), ChunkShape>,
}

impl NetherGenerator {
    /// The terrain's MT takes the seed's low 32 bits.
    pub fn new(seed: i64) -> Self {
        Self {
            noises: terrain::NetherNoises::new(seed as u32),
            biomes: biome::NetherBiomes::new(seed),
            surface: surface::NetherSurface::new(seed),
            carver: carver::NetherCarver::new(seed),
            features: Features::for_dimension(seed, 1, Dimension::Nether),
            structures: NetherStructures::new(seed),
            chunk_biomes: SourceCache::new(BIOMES_LIMIT),
            carved: SourceCache::new(CARVED_LIMIT),
            pieced: SourceCache::new(PIECED_LIMIT),
            staged: SourceCache::new(10 * PIECED_LIMIT),
            structure_order: Order::default(),
            loaded: SourceCache::new(CARVED_LIMIT),
        }
    }

    /// Stone, water and air from the density alone.
    pub fn chunk_shape(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.noises.chunk_shape(chunk_x, chunk_z)
    }

    /// The density, then the bedrock.
    pub fn bedrock_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        self.bedrocked(chunk_x, chunk_z).0
    }

    /// The bedrock on the density, and the chunk MT where the bedrock left it.
    fn bedrocked(&self, chunk_x: i32, chunk_z: i32) -> (ChunkShape, Mt19937) {
        let mut shape = self.chunk_shape(chunk_x, chunk_z);
        let mut random = floor::chunk_random(chunk_x, chunk_z);
        floor::place_bedrock(&mut shape, &mut random);
        (shape, random)
    }

    /// `bedrock_blocks`, then the biomes' surface builders.
    pub fn surfaced_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let (mut shape, mut random) = self.bedrocked(chunk_x, chunk_z);
        let biomes = self.chunk_biomes(chunk_x, chunk_z);
        self.surface.build(&mut shape, &biomes, &self.noises, (chunk_x, chunk_z), &mut random);
        shape
    }

    /// `surfaced_blocks`, then the pregeneration pass's carvers: the buffer `loadChunk` commits (1481eca38).
    pub fn carved_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        (*self.carved(chunk_x, chunk_z)).clone()
    }

    fn carved(&self, chunk_x: i32, chunk_z: i32) -> Arc<ChunkShape> {
        self.carved.get((chunk_x, chunk_z), || {
            let mut shape = self.surfaced_blocks(chunk_x, chunk_z);
            self.carver.carve(&mut shape, chunk_x, chunk_z);
            shape.share();
            shape
        })
    }

    /// Every ported stage: the structured chunk (`loaded_blocks`), then the decoration of every chunk that reaches it.
    pub fn chunk_blocks(&self, chunk_x: i32, chunk_z: i32) -> ChunkShape {
        let mut shape = self.loaded_blocks(chunk_x, chunk_z);
        let loaded = |x, z| self.loaded(x, z);
        let carved = |x, z| self.carved(x, z);
        let biomes = |x, z| self.chunk_biomes(x, z);
        let tops = |x, z| Arc::new(self.carved(x, z).tops());
        let surface = |_, _| crate::density::NO_SURFACE;
        let none = |_: [i32; 3]| false;
        let inputs = Inputs { loaded: &loaded, carved: &carved, generated_tops: &tops, biomes: &biomes, surface: &surface, in_structure: &none, in_tunnels: &none, island: None };
        self.features.place(&mut shape, (chunk_x, chunk_z), inputs);
        shape
    }

    /// Decorates in the order one BDS run did (`OverworldGenerator::set_decoration_order`).
    #[doc(hidden)]
    pub fn set_decoration_order(&mut self, order: &[(i32, i32)], thread: Option<(u32, &[Option<u32>])>) {
        self.features.set_order(order, thread);
    }

    /// Bedrock biome ids, one per column at every y.
    pub fn chunk_biomes(&self, chunk_x: i32, chunk_z: i32) -> Arc<ChunkBiomes> {
        self.chunk_biomes.get((chunk_x, chunk_z), || {
            let columns = std::array::from_fn(|i| self.biomes.biome(chunk_x * 16 + (i % 16) as i32, chunk_z * 16 + (i / 16) as i32));
            ChunkBiomes::from_columns((chunk_x, chunk_z), &columns)
        })
    }

    /// The multi-noise layer's biome at a quart column, before the voronoi zoom.
    #[doc(hidden)]
    pub fn quart_biome(&self, qx: i32, qz: i32) -> u8 {
        self.biomes.quart_biome(qx, qz)
    }
}
