//! One chunk column and what has been encoded from it. Encodings are made on first request and
//! dropped by the write that outdates them.

use std::sync::OnceLock;

use acacia_proto::types::SubChunkEntryItem;
use acacia_world::{BlockRegistry, Chunk, ColumnHeights, Dimension, LevelChunkData, SubChunkData};
use bytes::Bytes;

/// Biome of columns nobody painted (`minecraft:plains`).
const DEFAULT_BIOME: u32 = 1;

pub(crate) struct Column {
    chunk: Chunk,
    heights: OnceLock<ColumnHeights>,
    level_chunk: OnceLock<LevelChunkData>,
    /// Section payloads, bottom first; an inner `None` is an all-air section.
    sections: Box<[OnceLock<Option<Bytes>>]>,
}

impl Column {
    pub fn empty(x: i32, z: i32, dim: Dimension) -> Self {
        let mut chunk = Chunk::empty(x, z, dim);
        chunk.fill_biomes(DEFAULT_BIOME);
        let sections = (0..chunk.section_count()).map(|_| OnceLock::new()).collect();
        Self { chunk, heights: OnceLock::new(), level_chunk: OnceLock::new(), sections }
    }

    pub fn block(&self, x: i32, y: i32, z: i32) -> u32 {
        self.chunk.block(x, y, z)
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, id: u32) {
        self.chunk.set(x, y, z, 0, id);
        self.heights.take();
        self.level_chunk.take();
        if let Some(section) = self.slot(y >> 4).and_then(|slot| self.sections.get_mut(slot)) {
            section.take();
        }
    }

    /// The request-mode `LevelChunk` content: biomes, and how many sections to ask for.
    pub fn level_chunk(&self) -> &LevelChunkData {
        self.level_chunk.get_or_init(|| self.chunk.level_chunk_request())
    }

    /// The answer to a request for section `section_y`; `None` outside the dimension.
    pub fn sub_chunk(&self, section_y: i32, registry: &BlockRegistry, offset: [i8; 3]) -> Option<SubChunkEntryItem> {
        let cached = self.sections.get(self.slot(section_y)?)?;
        let heights = self.heights.get_or_init(|| self.chunk.column_heights(&|id| registry.get(id).is_some_and(|s| s.light_filter > 0)));
        let payload = cached.get_or_init(|| {
            let map = |id| registry.network_hash(id);
            self.chunk.sub_chunk(section_y, heights, &[], &map).and_then(|data| data.payload)
        });
        let data = SubChunkData { payload: payload.clone(), blob: None, heightmap: heights.sub_chunk(section_y) };
        Some(data.entry(offset[0], offset[1], offset[2]))
    }

    fn slot(&self, section_y: i32) -> Option<usize> {
        usize::try_from(section_y - (self.chunk.dimension().min_y >> 4)).ok()
    }

    #[cfg(test)]
    pub fn is_section_cached(&self, section_y: i32) -> bool {
        self.slot(section_y).and_then(|slot| self.sections.get(slot)).is_some_and(|s| s.get().is_some())
    }
}
