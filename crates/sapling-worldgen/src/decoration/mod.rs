//! The hardcoded part of `decorationPostProcessChunk` (1482143a0), after carvers: lava lakes.
//! Water lakes and the legacy cave and canyon configs only run in legacy worlds. Monster rooms,
//! feature rules and the freeze pass are not ported.

mod lake;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::random::{LargeFeatureSeed, Mt19937};
use crate::terrain::{CellKind, ChunkShape, HEIGHT, MIN_Y};

use lake::Lake;

/// A lake starts 3..11 blocks into its chunk and spans 16, so it reaches one chunk either way.
const LAKE_REACH: i32 = 1;

pub struct Decoration {
    seed: LargeFeatureSeed,
    lakes: Mutex<HashMap<(i32, i32), Arc<Option<Lake>>>>,
}

impl Decoration {
    /// The post-process generator is seeded like the carvers': `(x*a + z*b) ^ seed` on the low
    /// 32 bits of the world seed.
    pub fn new(world_seed: i64) -> Self {
        Self { seed: LargeFeatureSeed::new(world_seed as u32), lakes: Mutex::default() }
    }

    /// `carved` gives any chunk's terrain after its carvers, for lakes that test their surroundings.
    pub fn decorate(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32, carved: &dyn Fn(i32, i32) -> ChunkShape) {
        for source_x in chunk_x - LAKE_REACH..=chunk_x + LAKE_REACH {
            for source_z in chunk_z - LAKE_REACH..=chunk_z + LAKE_REACH {
                if let Some(lake) = self.lake(source_x, source_z, carved).as_ref() {
                    lake.apply(chunk_x, chunk_z, |x, y, z, block| shape.set_block(x, y, z, block));
                }
            }
        }
    }

    fn lake(&self, chunk_x: i32, chunk_z: i32, carved: &dyn Fn(i32, i32) -> ChunkShape) -> Arc<Option<Lake>> {
        if let Some(lake) = self.lakes.lock().unwrap().get(&(chunk_x, chunk_z)) {
            return lake.clone();
        }
        let mut shapes: HashMap<(i32, i32), ChunkShape> = HashMap::new();
        let mut kind = |x: i32, y: i32, z: i32| {
            let (cx, cz) = (x.div_euclid(16), z.div_euclid(16));
            let shape = shapes.entry((cx, cz)).or_insert_with(|| carved(cx, cz));
            if !(MIN_Y..MIN_Y + HEIGHT as i32).contains(&y) {
                return CellKind::Air;
            }
            shape.kind(x.rem_euclid(16) as usize, y, z.rem_euclid(16) as usize)
        };
        let mut random = Mt19937::new(self.seed.chunk(chunk_x, chunk_z));
        let lake = Arc::new(lake::lava_lake(&mut random, chunk_x, chunk_z, &mut kind));
        let mut lakes = self.lakes.lock().unwrap();
        if lakes.len() >= 16384 {
            lakes.clear();
        }
        lakes.entry((chunk_x, chunk_z)).or_insert(lake).clone()
    }
}
