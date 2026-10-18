//! One chunk column and what has been encoded from it. Encodings are made once, on first use or
//! by [`Column::warm`] off the tick thread, shared by every player, and dropped by the write that
//! outdates them.

use std::sync::OnceLock;

use acacia_proto::nbt::Value;
use acacia_proto::types::{Blob, SubChunkEntryItem};
use acacia_proto::xxh64;
use acacia_world::{BlockRegistry, Chunk, ColumnHeights, Dimension, LevelChunkData, SECTION_VOLUME, SubChunkData};
use bytes::Bytes;
use sapling_leveldb::{BlockEntity, block_entities, with_block_entity, without_block_entity};

use crate::storage::StoredColumn;

/// Biome of columns nobody painted: `minecraft:plains`, in the Nether `minecraft:hell`.
fn default_biome(dim: Dimension) -> u32 {
    if dim == Dimension::nether(dim.air) { 8 } else { 1 }
}

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

pub(crate) struct Column {
    chunk: Chunk,
    /// As stored (`StoredColumn::block_entities`).
    block_entities: Vec<u8>,
    /// The same, read, each with its bytes on the wire.
    tiles: OnceLock<Vec<(BlockEntity, Bytes)>>,
    /// As stored (`StoredColumn::actors`): nothing here moves or changes them.
    actors: Vec<u8>,
    /// Written since it was created, loaded or last handed to storage.
    modified: bool,
    damaged: bool,
    heights: OnceLock<ColumnHeights>,
    announce: OnceLock<LevelChunkData>,
    announce_cached: OnceLock<LevelChunkData>,
    /// Encoded sections with their blob hashes, bottom first; an inner `None` is all air.
    sections: Box<[OnceLock<Option<Blob>>]>,
}

impl Column {
    pub fn empty(x: i32, z: i32, dim: Dimension) -> Self {
        Self::from_stored(StoredColumn::of(Chunk::empty(x, z, dim)))
    }

    /// Stands in for a column storage holds but could not read: void, and never written or
    /// saved, so what storage holds survives.
    pub fn damaged(x: i32, z: i32, dim: Dimension) -> Self {
        Self { damaged: true, ..Self::empty(x, z, dim) }
    }

    /// A column as storage holds it; one saved without biomes gets the default.
    pub fn from_stored(StoredColumn { mut chunk, block_entities, actors }: StoredColumn) -> Self {
        if chunk.biome(0, chunk.dimension().min_y, 0).is_none() {
            chunk.fill_biomes(default_biome(chunk.dimension()));
        }
        let sections = (0..chunk.section_count()).map(|_| OnceLock::new()).collect();
        Self { chunk, block_entities, tiles: OnceLock::new(), actors, modified: false, damaged: false, heights: OnceLock::new(), announce: OnceLock::new(), announce_cached: OnceLock::new(), sections }
    }

    /// A copy for an arena to write to. The blocks are copied; the encodings are the same bytes
    /// until a write outdates one.
    // TODO: copy only the written section: needs `Arc` sections inside `acacia_world::Chunk`.
    pub fn fork(&self) -> Self {
        Self {
            chunk: self.chunk.clone(),
            block_entities: self.block_entities.clone(),
            tiles: self.tiles.clone(),
            actors: self.actors.clone(),
            modified: false,
            damaged: false,
            heights: self.heights.clone(),
            announce: self.announce.clone(),
            announce_cached: self.announce_cached.clone(),
            sections: self.sections.clone(),
        }
    }

    pub fn is_damaged(&self) -> bool {
        self.damaged
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Storage holds, or is being handed, the column as it is now.
    pub fn mark_saved(&mut self) {
        self.modified = false;
    }

    /// The save did not happen: the column is unsaved again.
    pub fn mark_modified(&mut self) {
        self.modified = true;
    }

    /// A copy of the column as storage should hold it.
    pub fn stored(&self) -> StoredColumn {
        StoredColumn { chunk: self.chunk.clone(), block_entities: self.block_entities.clone(), actors: self.actors.clone() }
    }

    pub fn actors(&self) -> &[u8] {
        &self.actors
    }

    /// Replaces the column's actors; it needs saving when they changed.
    pub fn set_actors(&mut self, actors: Vec<u8>) {
        if actors != self.actors {
            self.actors = actors;
            self.modified = true;
        }
    }

    pub fn block(&self, x: i32, y: i32, z: i32) -> u32 {
        self.chunk.block(x, y, z)
    }

    /// Sets a layer-0 block. A block entity belongs to the block it was saved with, so one at
    /// this position goes when the block becomes another kind.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, id: u32, registry: &BlockRegistry) {
        let name = |id| registry.get(id).map(|state| state.name);
        if !self.block_entities.is_empty() && name(self.chunk.block(x, y, z)) != name(id) {
            match without_block_entity(&self.block_entities, [x, y, z]) {
                Ok(rest) => {
                    self.block_entities = rest;
                    self.tiles.take();
                }
                Err(error) => tracing::warn!(x, y, z, %error, "unreadable block entities are kept as they are"),
            }
        }
        self.write(x, y, z, 0, id);
    }

    /// Makes `entity` the block entity of the block at `pos`; false when the column's block
    /// entities are unreadable, which are kept as they are.
    pub fn set_block_entity(&mut self, pos: [i32; 3], entity: &Value) -> bool {
        let Ok(record) = with_block_entity(&self.block_entities, pos, entity) else { return false };
        (self.block_entities, self.modified) = (record, true);
        self.tiles.take();
        true
    }

    /// The layer-1 block: the water a waterlogged block stands in, air otherwise.
    pub fn liquid(&self, x: i32, y: i32, z: i32) -> u32 {
        self.chunk.liquid(x, y, z)
    }

    pub fn set_liquid(&mut self, x: i32, y: i32, z: i32, id: u32) {
        self.write(x, y, z, 1, id);
    }

    /// Writes one block and drops every encoding it outdates: a later reader, cached or not,
    /// gets the section as it is now, under a new blob hash.
    fn write(&mut self, x: i32, y: i32, z: i32, layer: u32, id: u32) {
        self.chunk.set(x, y, z, layer, id);
        self.modified = true;
        self.heights.take();
        self.announce.take();
        self.announce_cached.take();
        if let Some(section) = self.slot(y >> 4).and_then(|slot| self.sections.get_mut(slot)) {
            section.take();
        }
    }

    /// Where the column holds a layer-0 block `wanted` accepts: x and z within it, y in the world.
    pub fn find(&self, wanted: impl Fn(u32) -> bool) -> Vec<[i32; 3]> {
        let mut ids: Vec<u32> = self.chunk.palette_ids().filter(|id| wanted(*id)).collect();
        ids.dedup();
        let (mut found, mut blocks, mut liquid) = (Vec::new(), [0; SECTION_VOLUME], [0; SECTION_VOLUME]);
        for section in (0..self.chunk.section_count()).filter(|_| !ids.is_empty()) {
            if self.chunk.section_uniform(section).is_some_and(|id| !ids.contains(&id)) || !self.chunk.copy_section(section, &mut blocks, &mut liquid) {
                continue;
            }
            let floor = self.chunk.dimension().min_y + 16 * section as i32;
            // XZY order: see `Chunk::copy_section`.
            found.extend(blocks.iter().enumerate().filter(|(_, id)| ids.contains(id)).map(|(at, _)| [(at >> 8) as i32, floor + (at & 15) as i32, (at >> 4 & 15) as i32]));
        }
        found
    }

    /// The y of the highest block `counts` accepts in block column `(x, z)`.
    pub fn top(&self, x: i32, z: i32, counts: impl Fn(u32) -> bool) -> Option<i32> {
        let dim = self.chunk.dimension();
        (dim.min_y..dim.min_y + dim.height as i32).rev().find(|&y| counts(self.chunk.block(x, y, z)))
    }

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
        self.heights.get_or_init(|| self.chunk.column_heights(&|id| registry.get(id).is_some_and(|s| s.light_filter > 0)))
    }

    /// Outer `None`: outside the dimension. Inner `None`: all air.
    fn section(&self, section_y: i32, registry: &BlockRegistry) -> Option<&Option<Blob>> {
        let cached = self.sections.get(self.slot(section_y)?)?;
        Some(cached.get_or_init(|| {
            let map = |id| registry.network_hash(id);
            let payload = self.chunk.sub_chunk(section_y, self.heights(registry), &[], &map).and_then(|data| data.payload)?;
            Some(Blob { hash: xxh64(&payload), payload })
        }))
    }

    fn slot(&self, section_y: i32) -> Option<usize> {
        usize::try_from(section_y - (self.chunk.dimension().min_y >> 4)).ok()
    }

    #[cfg(test)]
    pub fn is_section_cached(&self, section_y: i32) -> bool {
        self.slot(section_y).and_then(|slot| self.sections.get(slot)).is_some_and(|s| s.get().is_some())
    }
}
