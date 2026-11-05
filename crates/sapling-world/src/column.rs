//! One chunk column and what has been encoded from it. Encodings are made once, on first use or
//! by [`Column::warm`] off the tick thread, shared by every player, and dropped by the write that
//! outdates them (encoded.rs).

mod encoded;

use std::sync::OnceLock;

use acacia_proto::nbt::Value;
use acacia_proto::types::Blob;
use acacia_world::{BlockRegistry, Chunk, ColumnHeights, Dimension, LevelChunkData, SECTION_VOLUME};
use bytes::Bytes;
use sapling_leveldb::{BlockEntity, with_block_entity, without_block_entity};

use crate::storage::StoredColumn;

pub use encoded::{SubChunkAnswer, empty_column};

/// Biome of columns nobody painted: `minecraft:plains`, in the Nether `minecraft:hell`, in the End
/// `minecraft:the_end`.
pub(crate) fn default_biome(dim: Dimension) -> u32 {
    match dim {
        _ if dim == Dimension::nether(dim.air) => 8,
        _ if dim == Dimension::end(dim.air) => 9,
        _ => 1,
    }
}

pub(crate) struct Column {
    chunk: Chunk,
    /// As stored (`StoredColumn::block_entities`).
    block_entities: Vec<u8>,
    /// The same, read, each with its bytes on the wire.
    tiles: OnceLock<Vec<(BlockEntity, Bytes)>>,
    /// As stored (`StoredColumn::actors`): nothing here moves or changes them.
    actors: Vec<u8>,
    /// As stored (`StoredColumn::pending_ticks`); the shard writes them back before a save.
    pending_ticks: Vec<u8>,
    /// As stored (`StoredColumn::spawn_areas`): nothing here changes them.
    spawn_areas: Vec<u8>,
    /// As generated (`StoredColumn::template_actors`), until the shard takes them.
    template_actors: Vec<sapling_worldgen::TemplateActor>,
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
    pub fn from_stored(StoredColumn { mut chunk, block_entities, actors, pending_ticks, spawn_areas, template_actors }: StoredColumn) -> Self {
        if chunk.biome(0, chunk.dimension().min_y, 0).is_none() {
            chunk.fill_biomes(default_biome(chunk.dimension()));
        }
        let sections = (0..chunk.section_count()).map(|_| OnceLock::new()).collect();
        Self { chunk, block_entities, tiles: OnceLock::new(), actors, pending_ticks, spawn_areas, template_actors, modified: false, damaged: false, heights: OnceLock::new(), announce: OnceLock::new(), announce_cached: OnceLock::new(), sections }
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
            pending_ticks: self.pending_ticks.clone(),
            spawn_areas: self.spawn_areas.clone(),
            template_actors: Vec::new(),
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
        StoredColumn {
            chunk: self.chunk.clone(),
            block_entities: self.block_entities.clone(),
            actors: self.actors.clone(),
            pending_ticks: self.pending_ticks.clone(),
            spawn_areas: self.spawn_areas.clone(),
            template_actors: Vec::new(),
        }
    }

    /// The mobs generation placed, handed out once.
    pub fn take_template_actors(&mut self) -> Vec<sapling_worldgen::TemplateActor> {
        std::mem::take(&mut self.template_actors)
    }

    pub fn spawn_areas(&self) -> &[u8] {
        &self.spawn_areas
    }

    pub fn actors(&self) -> &[u8] {
        &self.actors
    }

    /// Replaces the column's actors; it needs saving when they changed.
    pub fn set_actors(&mut self, actors: Vec<u8>) {
        self.modified |= replace(&mut self.actors, actors);
    }

    pub fn pending_ticks(&self) -> &[u8] {
        &self.pending_ticks
    }

    /// Replaces the column's waiting block ticks; it needs saving when they changed.
    pub fn set_pending_ticks(&mut self, ticks: Vec<u8>) {
        self.modified |= replace(&mut self.pending_ticks, ticks);
    }

    pub fn chunk(&self) -> &Chunk {
        &self.chunk
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
        (0..self.chunk.section_count()).rev().filter(|&section| self.chunk.section_uniform(section).is_none_or(&counts)).find_map(|section| {
            let floor = dim.min_y + 16 * section as i32;
            (floor..floor + 16).rev().find(|&y| counts(self.chunk.block(x, y, z)))
        })
    }

    /// The sections from the bottom up to the highest that holds anything but air in either layer.
    pub fn filled_sections(&self) -> usize {
        let air = self.chunk.dimension().air;
        (0..self.chunk.section_count()).rev().find(|&section| self.chunk.section_uniform(section) != Some(air)).map_or(0, |section| section + 1)
    }

    fn slot(&self, section_y: i32) -> Option<usize> {
        usize::try_from(section_y - (self.chunk.dimension().min_y >> 4)).ok()
    }
}

/// Puts `new` in `kept`; whether that changed it.
fn replace(kept: &mut Vec<u8>, new: Vec<u8>) -> bool {
    let changed = *kept != new;
    *kept = new;
    changed
}
