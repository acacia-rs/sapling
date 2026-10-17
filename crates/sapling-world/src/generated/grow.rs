//! A worldgen feature placed over the loaded columns, as a sapling grows its tree
//! (`sapling_worldgen::place_feature`).

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use acacia_world::BlockRegistry;
use sapling_worldgen::{Block, ChunkShape, HEIGHT, MIN_Y};

use super::convert::BlockIds;
use crate::{ChunkPos, World};

/// What a runtime id is to worldgen, and back.
struct Ids {
    forward: BlockIds,
    back: HashMap<u32, Block>,
}

/// Built from the first registry asked: every world holds the vanilla registry.
fn ids(registry: &BlockRegistry) -> &'static Ids {
    static IDS: OnceLock<Ids> = OnceLock::new();
    IDS.get_or_init(|| {
        let forward = BlockIds::of(registry);
        let mut back = HashMap::new();
        Block::ALL.iter().chain(&Block::templates().collect::<Vec<_>>()).for_each(|&block| _ = back.entry(forward.id(block)).or_insert(block));
        Ids { forward, back }
    })
}

impl World {
    /// Places the feature `name` at `at` with a generator seeded `seed` over the loaded columns:
    /// the cells it writes and their new runtime ids, in the order it wrote them; `None` where it
    /// did not place. Columns not loaded read as air.
    // TODO: a block worldgen does not name reads as stone; layer 1 and biomes are not read.
    pub fn place_feature(&self, name: &str, at: [i32; 3], seed: u32) -> Option<Vec<([i32; 3], u32)>> {
        let ids = ids(&self.registry);
        let load = |chunk_x: i32, chunk_z: i32| Arc::new(self.shape_of(ChunkPos::new(chunk_x, chunk_z), ids));
        let writes = sapling_worldgen::place_feature(name, at, seed, &load, &|_| None)?;
        Some(writes.into_iter().map(|(pos, block, _)| (pos, ids.forward.id(block))).collect())
    }

    fn shape_of(&self, pos: ChunkPos, ids: &Ids) -> ChunkShape {
        let mut shape = ChunkShape::empty();
        if !self.is_loaded(pos) {
            return shape;
        }
        for y in MIN_Y..MIN_Y + HEIGHT as i32 {
            for z in 0..16 {
                for x in 0..16 {
                    let id = self.block(pos.x * 16 + x as i32, y, pos.z * 16 + z as i32);
                    if id != self.dim.air {
                        shape.set_block(x, y, z, ids.back.get(&id).copied().unwrap_or(Block::Stone));
                    }
                }
            }
        }
        shape
    }
}
