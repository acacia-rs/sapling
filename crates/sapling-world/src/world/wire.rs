//! A world's columns as clients are sent them.

use acacia_proto::types::SubChunkEntryItemResult;
use acacia_world::{Heightmap, LevelChunkData, SubChunkData};

use super::World;
use crate::ChunkPos;
use crate::column::SubChunkAnswer;

impl World {
    /// What announces a column in sub-chunk request mode: `.packet(x, z, dimension)` is the
    /// `LevelChunk`, and for a blob-cache client (`cached`) `.blobs` holds the biomes it names.
    /// `None` until the column is loaded.
    pub fn level_chunk(&self, pos: ChunkPos, cached: bool) -> Option<&LevelChunkData> {
        Some(self.column(pos)?.level_chunk(cached))
    }

    /// One entry of a `Subchunk` answer: section `section_y` of column `pos`, requested at
    /// `offset` from the request's origin.
    pub fn sub_chunk(&self, pos: ChunkPos, section_y: i32, offset: [i8; 3], cached: bool) -> SubChunkAnswer {
        let failed = |result| {
            let missing = SubChunkData { payload: None, blob: None, heightmap: Heightmap::TooLow };
            SubChunkAnswer { entry: acacia_proto::types::SubChunkEntryItem { result, ..missing.entry(offset[0], offset[1], offset[2]) }, blob: None }
        };
        let Some(column) = self.column(pos) else { return failed(SubChunkEntryItemResult::ChunkNotFound) };
        column.sub_chunk(section_y, &self.registry, offset, cached).unwrap_or_else(|| failed(SubChunkEntryItemResult::YIndexOutOfBounds))
    }
}
