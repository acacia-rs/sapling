use std::collections::HashMap;
use std::sync::Arc;

use acacia_proto::packets::LevelChunk;
use acacia_proto::types::{SubChunkEntryItem, SubChunkEntryItemResult};
use acacia_world::{BlockRegistry, Dimension, SubChunkData};

use crate::ChunkPos;
use crate::column::Column;

/// One dimension's blocks. Columns nobody wrote to are void: they exist for every position and
/// encode as all air.
pub struct World {
    dimension: i32,
    dim: Dimension,
    registry: Arc<BlockRegistry>,
    columns: HashMap<ChunkPos, Column>,
    /// Stands in for every unwritten column, so they share one encoding.
    void: Column,
}

impl World {
    /// An empty overworld.
    pub fn void() -> Self {
        let registry = BlockRegistry::vanilla_arc();
        let dim = Dimension::overworld(registry.air_id());
        Self { dimension: 0, dim, registry, columns: HashMap::new(), void: Column::empty(0, 0, dim) }
    }

    /// The protocol dimension id (0 = overworld).
    pub fn dimension(&self) -> i32 {
        self.dimension
    }

    pub fn registry(&self) -> &BlockRegistry {
        &self.registry
    }

    /// The runtime id of a block's default-ordered first state, e.g. `minecraft:stone`.
    pub fn block_id(&self, name: &str) -> Option<u32> {
        self.registry.states_of(name).next().map(|(id, _)| id)
    }

    pub fn block(&self, x: i32, y: i32, z: i32) -> u32 {
        self.column(ChunkPos::of_block(x, z)).block(x, y, z)
    }

    /// Sets a layer-0 block by runtime id. Positions outside the dimension's height are ignored.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, id: u32) {
        let pos = ChunkPos::of_block(x, z);
        self.columns.entry(pos).or_insert_with(|| Column::empty(pos.x, pos.z, self.dim)).set_block(x, y, z, id);
    }

    /// Fills the box between two corners, both included.
    pub fn fill(&mut self, from: [i32; 3], to: [i32; 3], id: u32) {
        let range = |axis: usize| from[axis].min(to[axis])..=from[axis].max(to[axis]);
        for x in range(0) {
            for z in range(2) {
                for y in range(1) {
                    self.set_block(x, y, z, id);
                }
            }
        }
    }

    /// The `LevelChunk` announcing a column in sub-chunk request mode.
    pub fn level_chunk(&self, pos: ChunkPos) -> LevelChunk {
        self.column(pos).level_chunk().packet(pos.x, pos.z, self.dimension)
    }

    /// One entry of a `Subchunk` answer: section `section_y` of column `pos`, requested at
    /// `offset` from the request's origin.
    pub fn sub_chunk(&self, pos: ChunkPos, section_y: i32, offset: [i8; 3]) -> SubChunkEntryItem {
        self.column(pos).sub_chunk(section_y, &self.registry, offset).unwrap_or_else(|| {
            let missing = SubChunkData { payload: None, blob: None, heightmap: acacia_world::Heightmap::TooLow };
            SubChunkEntryItem { result: SubChunkEntryItemResult::YIndexOutOfBounds, ..missing.entry(offset[0], offset[1], offset[2]) }
        })
    }

    fn column(&self, pos: ChunkPos) -> &Column {
        self.columns.get(&pos).unwrap_or(&self.void)
    }
}

#[cfg(test)]
mod tests {
    use acacia_proto::types::HeightMapDataType;
    use acacia_world::Chunk;

    use super::*;

    fn platform() -> (World, u32) {
        let mut world = World::void();
        let stone = world.block_id("minecraft:stone").unwrap();
        world.fill([-8, 64, -8], [7, 64, 7], stone);
        (world, stone)
    }

    #[test]
    fn platform_spans_four_columns_and_void_stays_air() {
        let (world, stone) = platform();
        let air = world.registry().air_id();
        assert_eq!((world.block(-8, 64, -8), world.block(7, 64, 7)), (stone, stone));
        assert_eq!((world.block(8, 64, 0), world.block(0, 65, 0), world.block(0, 400, 0)), (air, air, air));
        assert_eq!(world.columns.len(), 4);
        // y 64 is the ninth section from the bottom (-64).
        assert_eq!(world.level_chunk(ChunkPos::new(0, 0)).highest_subchunk_count, Some(9));
        let void = world.level_chunk(ChunkPos::new(5, 5));
        assert_eq!((void.x, void.z, void.highest_subchunk_count, void.cache_enabled), (5, 5, Some(0), false));
    }

    #[test]
    fn sections_decode_back_to_the_blocks_written() {
        let (world, stone) = platform();
        let entry = world.sub_chunk(ChunkPos::new(-1, 0), 4, [-1, 4, 0]);
        assert_eq!((entry.dx, entry.dy, entry.dz, entry.result), (-1, 4, 0, SubChunkEntryItemResult::Success));
        assert_eq!(entry.heightmap_type, HeightMapDataType::HasData);
        let registry = world.registry();
        let mut decoded = Chunk::empty(-1, 0, Dimension::overworld(registry.air_id()));
        let unhash = |hash| registry.runtime_id_from_hash(hash).expect("a vanilla state hash");
        decoded.set_sub_chunk_mapped(4, &entry.payload.unwrap(), &unhash).unwrap();
        assert_eq!((decoded.block(-1, 64, 3), decoded.block(-1, 65, 3)), (stone, registry.air_id()));

        let above = world.sub_chunk(ChunkPos::new(-1, 0), 5, [0, 0, 0]);
        assert_eq!((above.result, above.payload, above.heightmap_type), (SubChunkEntryItemResult::SuccessAllAir, None, HeightMapDataType::TooLow));
        let void = world.sub_chunk(ChunkPos::new(9, 9), 4, [0, 0, 0]);
        assert_eq!(void.result, SubChunkEntryItemResult::SuccessAllAir);
        assert_eq!(world.sub_chunk(ChunkPos::new(0, 0), 24, [0, 0, 0]).result, SubChunkEntryItemResult::YIndexOutOfBounds);
        assert_eq!(world.sub_chunk(ChunkPos::new(0, 0), -5, [0, 0, 0]).result, SubChunkEntryItemResult::YIndexOutOfBounds);
    }

    #[test]
    fn a_write_drops_only_its_section_from_the_cache() {
        let (mut world, stone) = platform();
        let pos = ChunkPos::new(0, 0);
        let before = world.sub_chunk(pos, 4, [0, 0, 0]);
        world.sub_chunk(pos, 0, [0, 0, 0]);
        assert!(world.columns[&pos].is_section_cached(4) && world.columns[&pos].is_section_cached(0));

        world.set_block(3, 90, 3, stone);
        assert!(world.columns[&pos].is_section_cached(4) && world.columns[&pos].is_section_cached(0));
        assert_eq!(world.level_chunk(pos).highest_subchunk_count, Some(10));
        // The heightmap is not cached with the section: the new block above raises it.
        let after = world.sub_chunk(pos, 4, [0, 0, 0]);
        assert_eq!(after.payload, before.payload);
        assert_ne!(after.heightmap, before.heightmap);

        world.set_block(3, 64, 3, world.registry().air_id());
        assert!(!world.columns[&pos].is_section_cached(4));
        assert_ne!(world.sub_chunk(pos, 4, [0, 0, 0]).payload, before.payload);
    }
}
