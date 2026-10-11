//! The hardcoded part of `decorationPostProcessChunk` (1482143a0), after carvers: lava lakes.
//! Water lakes and the legacy cave and canyon configs only run in legacy worlds. Monster rooms run
//! with the feature rules (`crate::feature`), on the generator the lake leaves.

pub mod lake;

use std::collections::HashMap;
use std::sync::Arc;

use crate::cache::SourceCache;
use crate::random::{LargeFeatureSeed, Mt19937};
use crate::terrain::{CellKind, ChunkShape, HEIGHT, MIN_Y};

use lake::Lake;

/// A lake starts 3..11 blocks into its chunk and spans 16, so it reaches one chunk either way.
const LAKE_REACH: i32 = 1;

pub struct Decoration {
    seed: LargeFeatureSeed,
    /// Each chunk's lava lake and the raw draws its attempt took.
    lakes: SourceCache<(i32, i32), (Option<Lake>, u32)>,
}

impl Decoration {
    /// The post-process generator is seeded like the carvers': `(x*a + z*b) ^ seed` on the low
    /// 32 bits of the world seed.
    pub fn new(world_seed: i64) -> Self {
        Self { seed: LargeFeatureSeed::new(world_seed as u32), lakes: SourceCache::new(16384) }
    }

    /// `carved` gives any chunk's terrain after its carvers, for lakes that test their surroundings.
    pub fn decorate(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32, carved: &dyn Fn(i32, i32) -> Arc<ChunkShape>) {
        for source_x in chunk_x - LAKE_REACH..=chunk_x + LAKE_REACH {
            for source_z in chunk_z - LAKE_REACH..=chunk_z + LAKE_REACH {
                if let Some(lake) = self.lake(source_x, source_z, carved).0.as_ref() {
                    lake.apply(chunk_x, chunk_z, |x, y, z, block| shape.set_block(x, y, z, block));
                }
            }
        }
    }

    fn lake(&self, chunk_x: i32, chunk_z: i32, carved: &dyn Fn(i32, i32) -> Arc<ChunkShape>) -> Arc<(Option<Lake>, u32)> {
        self.lakes.get((chunk_x, chunk_z), || {
            let mut shapes: HashMap<(i32, i32), Arc<ChunkShape>> = HashMap::new();
            let mut kind = |x: i32, y: i32, z: i32| {
                let (cx, cz) = (x.div_euclid(16), z.div_euclid(16));
                let shape = shapes.entry((cx, cz)).or_insert_with(|| carved(cx, cz));
                if !(MIN_Y..MIN_Y + HEIGHT as i32).contains(&y) {
                    return CellKind::Air;
                }
                shape.kind(x.rem_euclid(16) as usize, y, z.rem_euclid(16) as usize)
            };
            let mut random = Mt19937::new(self.seed.chunk(chunk_x, chunk_z));
            let lake = lake::lava_lake(&mut random, chunk_x, chunk_z, &mut kind);
            (lake, random.draws())
        })
    }
}
