//! The hardcoded part of `decorationPostProcessChunk` (1482143a0), after carvers: the lava lake,
//! then monster rooms. Water lakes and the legacy cave and canyon configs only run in legacy
//! worlds. The freeze pass is not ported; feature rules are `crate::feature`.

mod lake;
mod monster;

use std::collections::HashMap;
use std::sync::Arc;

use crate::block::Block;
use crate::cache::SourceCache;
use crate::random::{LargeFeatureSeed, Mt19937};
use crate::terrain::{ChunkShape, HEIGHT, MIN_Y};

use lake::Lake;
use monster::Writes;

/// A lake starts 3..11 blocks into its chunk and spans 16, so it reaches one chunk either way.
const LAKE_REACH: i32 = 1;

/// One chunk's decoration, reaching into its neighbours.
struct Decor {
    lake: Option<Lake>,
    /// Monster rooms reach from 4 blocks into their chunk to 11 into the next in +x and +z.
    rooms: Writes,
}

pub struct Decoration {
    seed: LargeFeatureSeed,
    decor: SourceCache<(i32, i32), Decor>,
}

impl Decoration {
    /// The post-process generator is seeded like the carvers': `(x*a + z*b) ^ seed` on the low
    /// 32 bits of the world seed.
    pub fn new(world_seed: i64) -> Self {
        Self { seed: LargeFeatureSeed::new(world_seed as u32), decor: SourceCache::new(16384) }
    }

    /// `loaded` gives any chunk's terrain before decoration, which lakes and rooms test.
    pub fn decorate(&self, shape: &mut ChunkShape, chunk_x: i32, chunk_z: i32, loaded: &dyn Fn(i32, i32) -> Arc<ChunkShape>) {
        for source_x in chunk_x - LAKE_REACH..=chunk_x + LAKE_REACH {
            for source_z in chunk_z - LAKE_REACH..=chunk_z + LAKE_REACH {
                if let Some(lake) = self.decor(source_x, source_z, loaded).lake.as_ref() {
                    lake.apply(chunk_x, chunk_z, |x, y, z, block| shape.set_block(x, y, z, block));
                }
            }
        }
        for source_x in chunk_x - 1..=chunk_x {
            for source_z in chunk_z - 1..=chunk_z {
                for &((x, y, z), block) in &self.decor(source_x, source_z, loaded).rooms {
                    if x.div_euclid(16) == chunk_x && z.div_euclid(16) == chunk_z && in_height(y) {
                        shape.set_block(x.rem_euclid(16) as usize, y, z.rem_euclid(16) as usize, block);
                    }
                }
            }
        }
    }

    fn decor(&self, chunk_x: i32, chunk_z: i32, loaded: &dyn Fn(i32, i32) -> Arc<ChunkShape>) -> Arc<Decor> {
        self.decor.get((chunk_x, chunk_z), || {
            let mut shapes: HashMap<(i32, i32), Arc<ChunkShape>> = HashMap::new();
            let mut block = |x: i32, y: i32, z: i32| {
                let (cx, cz) = (x.div_euclid(16), z.div_euclid(16));
                let shape = shapes.entry((cx, cz)).or_insert_with(|| loaded(cx, cz));
                if in_height(y) { shape.block(x.rem_euclid(16) as usize, y, z.rem_euclid(16) as usize) } else { Block::Air }
            };
            let mut random = Mt19937::new(self.seed.chunk(chunk_x, chunk_z));
            let lake = lake::lava_lake(&mut random, chunk_x, chunk_z, &mut |x, y, z| kind(block(x, y, z)));
            if let Some(lake) = &lake {
                random.consume_count(lake.border_draws() as u32);
            }
            let rooms = monster::monster_rooms(&mut random, (chunk_x, chunk_z), &mut block);
            Decor { lake, rooms }
        })
    }
}

fn kind(block: Block) -> crate::terrain::CellKind {
    use crate::terrain::CellKind;
    match block {
        Block::Air => CellKind::Air,
        Block::Water => CellKind::Water,
        Block::Lava => CellKind::Lava,
        _ => CellKind::Solid,
    }
}

fn in_height(y: i32) -> bool {
    (MIN_Y..MIN_Y + HEIGHT as i32).contains(&y)
}
