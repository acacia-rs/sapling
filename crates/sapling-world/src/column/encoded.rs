//! What a column hands clients: its `LevelChunk` announcement, its sections and their block
//! entities, each encoded once and kept until a write outdates it (`Column::write`).

use std::sync::OnceLock;

use acacia_proto::types::{Blob, SubChunkEntryItem};
use acacia_proto::xxh64;
use acacia_world::{BlockRegistry, ColumnHeights, Dimension, LevelChunkData, SubChunkData};
use bytes::Bytes;
use sapling_leveldb::{BlockEntity, block_entities};

use super::Column;

/// One entry of a `Subchunk` answer. For a blob-cache client the section travels as `blob`,
/// which the entry names by hash; the sender keeps it until the client says whether it has it.
#[derive(Debug, Clone, PartialEq)]
pub struct SubChunkAnswer {
    pub entry: SubChunkEntryItem,
    pub blob: Option<Blob>,
}

/// The announcement of a column with no blocks in a dimension (0 to 2), encoded once: it has no
/// sections for a client to ask for, so sending it empties whatever the client holds there.
pub fn empty_column(dimension: i32, cached: bool) -> &'static LevelChunkData {
    static EMPTY: [OnceLock<Column>; 3] = [const { OnceLock::new() }; 3];
    let slot = dimension.clamp(0, 2) as usize;
    EMPTY[slot].get_or_init(|| Column::empty(0, 0, Dimension::from_id(slot as i32, BlockRegistry::vanilla().air_id()))).level_chunk(cached)
}

impl Column {
    /// Encodes everything a client can ask of this column.
    pub fn warm(&self, registry: &BlockRegistry) {
        self.level_chunk(false);
        self.level_chunk(true);
        self.tiles();
        let lowest = self.chunk.dimension().min_y >> 4;
        for slot in 0..self.sections.len() as i32 {
            self.section(lowest + slot, registry);
        }
    }

    /// The request-mode `LevelChunk` content: how many sections to ask for, and the biomes, in
    /// the payload or (`cached`) as the column's one blob.
    pub fn level_chunk(&self, cached: bool) -> &LevelChunkData {
        match cached {
            false => self.announce.get_or_init(|| self.chunk.level_chunk_request()),
            true => self.announce_cached.get_or_init(|| self.chunk.level_chunk_request_cached()),
        }
    }

    /// The answer to a request for section `section_y`; `None` outside the dimension.
    pub fn sub_chunk(&self, section_y: i32, registry: &BlockRegistry, offset: [i8; 3], cached: bool) -> Option<SubChunkAnswer> {
        let section = self.section(section_y, registry)?;
        let heightmap = self.heights(registry).sub_chunk(section_y);
        // Block entities follow the section (uncached) or are the payload (cached).
        let in_section = |(tile, _): &&(BlockEntity, Bytes)| tile.pos[1] >> 4 == section_y;
        let tiles: Vec<u8> = self.tiles().iter().filter(in_section).flat_map(|(_, wire)| wire.iter().copied()).collect();
        let (payload, blob) = match (section, cached) {
            (None, _) => (None, None),
            (Some(blob), false) if tiles.is_empty() => (Some(blob.payload.clone()), None),
            (Some(blob), false) => (Some([&blob.payload[..], &tiles].concat().into()), None),
            (Some(blob), true) => (Some(tiles.into()), Some(blob.clone())),
        };
        let entry = SubChunkData { payload, blob: blob.clone(), heightmap }.entry(offset[0], offset[1], offset[2]);
        Some(SubChunkAnswer { entry, blob })
    }

    pub fn block_entity(&self, pos: [i32; 3]) -> Option<&BlockEntity> {
        self.block_entities().find(|tile| tile.pos == pos)
    }

    pub fn block_entities(&self) -> impl Iterator<Item = &BlockEntity> {
        self.tiles().iter().map(|(tile, _)| tile)
    }

    fn tiles(&self) -> &[(BlockEntity, Bytes)] {
        self.tiles.get_or_init(|| match block_entities(&self.block_entities) {
            Ok(tiles) => tiles.into_iter().map(|tile| (tile.network(), tile)).map(|(wire, tile)| (tile, wire)).collect(),
            Err(error) => {
                tracing::warn!(%error, "a column's block entities are unreadable; showing none of them");
                Vec::new()
            }
        })
    }

    fn heights(&self, registry: &BlockRegistry) -> &ColumnHeights {
        let stops_light = |id| registry.get(id).map_or_else(|| sapling_leveldb::custom_state(id).is_some_and(|s| s.light_filter > 0), |s| s.light_filter > 0);
        self.heights.get_or_init(|| self.chunk.column_heights(&stops_light))
    }

    /// Outer `None`: outside the dimension. Inner `None`: all air.
    fn section(&self, section_y: i32, registry: &BlockRegistry) -> Option<&Option<Blob>> {
        let cached = self.sections.get(self.slot(section_y)?)?;
        Some(cached.get_or_init(|| {
            let map = |id| sapling_leveldb::block_hash(registry, id);
            let payload = self.chunk.sub_chunk(section_y, self.heights(registry), &[], &map).and_then(|data| data.payload)?;
            Some(Blob { hash: xxh64(&payload), payload })
        }))
    }

    #[cfg(test)]
    pub fn is_section_cached(&self, section_y: i32) -> bool {
        self.slot(section_y).and_then(|slot| self.sections.get(slot)).is_some_and(|s| s.get().is_some())
    }
}
