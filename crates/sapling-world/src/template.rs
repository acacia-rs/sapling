//! Templates and arenas (docs/DESIGN.md "Arenas"). A template is a world frozen for good; an
//! arena is a world that reads its template and holds only the columns written to since. Making
//! one copies nothing, and until a column is written every arena serves the template's own
//! encodings, so a map is encoded once however many games run on it.

use std::collections::HashMap;
use std::sync::Arc;

use acacia_world::{BlockRegistry, Dimension};

use crate::column::Column;
use crate::storage::{Level, StorageError};
use crate::{ChunkPos, World};

pub struct Template {
    dimension: i32,
    dim: Dimension,
    registry: Arc<BlockRegistry>,
    columns: HashMap<ChunkPos, Column>,
    /// The columns that hold actors: an arena's mobs start from them.
    with_actors: Vec<ChunkPos>,
    level: Level,
}

impl Template {
    /// Freezes a world as it is in memory: the columns written to it or loaded from its storage
    /// ([`World::load_all`]), with everything a client can ask of them encoded now. Its storage
    /// is let go, and what was not saved is not saved: a template only lives in memory.
    pub fn of(world: World) -> Arc<Self> {
        let World { dimension, dim, registry, columns, level, .. } = world;
        columns.values().for_each(|column| column.warm(&registry));
        let mut with_actors: Vec<ChunkPos> = columns.iter().filter(|(_, column)| !column.actors().is_empty()).map(|(pos, _)| *pos).collect();
        with_actors.sort();
        Arc::new(Self { dimension, dim, registry, columns, with_actors, level })
    }

    /// How many columns the template holds; the rest is void.
    pub fn columns(&self) -> usize {
        self.columns.len()
    }

    pub(crate) fn column(&self, pos: ChunkPos) -> Option<&Column> {
        self.columns.get(&pos)
    }
}

impl World {
    /// A world that starts as `template` and is never saved. Columns are shared with the template
    /// and every other arena of it until they are written to.
    pub fn arena(template: &Arc<Template>) -> Self {
        Self {
            dimension: template.dimension,
            dim: template.dim,
            registry: template.registry.clone(),
            columns: HashMap::new(),
            void: Column::empty(0, 0, template.dim),
            loader: None,
            level: template.level.clone(),
            arrived: Vec::new(),
            base: Some(template.clone()),
            base_actors_taken: false,
            players: crate::players::Players::default(),
        }
    }

    pub fn is_arena(&self) -> bool {
        self.base.is_some()
    }

    /// The columns of an arena that differ from its template, or did once.
    pub fn changed_columns(&self) -> usize {
        self.base.as_ref().map_or(0, |_| self.columns.len())
    }

    /// Puts an arena back as its template has it by dropping the columns it changed; returns
    /// which. Does nothing to a world that is not an arena.
    pub fn reset(&mut self) -> Vec<ChunkPos> {
        match self.base {
            Some(_) => self.columns.drain().map(|(pos, _)| pos).collect(),
            None => Vec::new(),
        }
    }

    /// Loads every column storage holds, on the calling thread: before [`Template::of`], never
    /// on a shard. Returns how many there are.
    pub fn load_all(&mut self) -> Result<usize, StorageError> {
        let Some(loader) = &self.loader else { return Ok(0) };
        let stored = loader.source().storage.columns(self.dimension)?;
        stored.iter().for_each(|pos| self.load_now(*pos));
        Ok(stored.len())
    }

    /// The template's columns with actors, once: the first [`World::take_arrived`] of an arena.
    pub(crate) fn take_base_actors(&mut self) -> Vec<ChunkPos> {
        match &self.base {
            Some(template) if !std::mem::replace(&mut self.base_actors_taken, true) => template.with_actors.clone(),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use acacia_world::Chunk;

    use super::*;
    use crate::{MemoryStorage, StoredColumn, WorldStorage};

    const ORIGIN: ChunkPos = ChunkPos::new(0, 0);

    /// A platform of stone at y 64 with a second floor at y 80, in four columns.
    fn template() -> (Arc<Template>, u32) {
        let mut world = World::void();
        let stone = world.block_id("minecraft:stone").unwrap();
        world.fill([-8, 64, -8], [7, 64, 7], stone);
        world.fill([-8, 80, -8], [7, 80, 7], stone);
        (Template::of(world), stone)
    }

    /// Where the encoded section `section_y` of the column at `pos` lives, and its hash.
    fn encoding(world: &World, pos: ChunkPos, section_y: i32) -> (*const u8, u64) {
        let blob = world.sub_chunk(pos, section_y, [0, 0, 0], true).blob.expect("a section with blocks");
        (blob.payload.as_ptr(), blob.hash)
    }

    #[test]
    fn fifty_arenas_share_their_template() {
        let (template, stone) = template();
        let arenas: Vec<World> = (0..50).map(|_| World::arena(&template)).collect();
        assert_eq!((Arc::strong_count(&template), template.columns()), (51, 4));
        let first = encoding(&arenas[0], ORIGIN, 4);
        for arena in &arenas {
            assert!(arena.is_arena() && arena.is_loaded(ChunkPos::new(40, 40)), "every column exists, most are void");
            assert_eq!((arena.changed_columns(), arena.block(0, 64, 0), arena.block(0, 65, 0)), (0, stone, arena.registry().air_id()));
            assert_eq!(encoding(arena, ORIGIN, 4), first, "one encoding in memory, under one hash");
            assert!(std::ptr::eq(arena.level_chunk(ORIGIN, true).unwrap(), arenas[0].level_chunk(ORIGIN, true).unwrap()));
        }
    }

    #[test]
    fn a_write_changes_one_arena_and_copies_one_column() {
        let (template, stone) = template();
        let (mut played, untouched) = (World::arena(&template), World::arena(&template));
        let (air, before) = (played.registry().air_id(), encoding(&untouched, ORIGIN, 4));
        assert!(played.set_block(0, 64, 0, air));
        assert_eq!((played.block(0, 64, 0), untouched.block(0, 64, 0), World::arena(&template).block(0, 64, 0)), (air, stone, stone));
        assert_eq!((played.changed_columns(), untouched.changed_columns()), (1, 0));

        // The written section is encoded anew under a new hash; the column's other section and
        // the other columns still are the template's encodings.
        let after = encoding(&played, ORIGIN, 4);
        assert!(after.1 != before.1 && encoding(&untouched, ORIGIN, 4) == before);
        assert_eq!(encoding(&played, ORIGIN, 5), encoding(&untouched, ORIGIN, 5));
        assert_eq!(encoding(&played, ChunkPos::new(-1, 0), 4), encoding(&untouched, ChunkPos::new(-1, 0), 4));
        // A column the template lacks is the arena's own from the first write.
        assert!(played.set_block(100, 64, 100, stone));
        assert_eq!((played.block(100, 64, 100), untouched.block(100, 64, 100), played.changed_columns()), (stone, air, 2));
    }

    #[test]
    fn a_reset_drops_what_the_arena_changed() {
        let (template, stone) = template();
        let mut arena = World::arena(&template);
        let (air, before) = (arena.registry().air_id(), encoding(&arena, ORIGIN, 4));
        arena.set_block(0, 64, 0, air);
        arena.set_liquid(1, 64, 0, stone);
        arena.set_block(100, 64, 100, stone);
        let mut changed = arena.reset();
        changed.sort();
        assert_eq!(changed, [ORIGIN, ChunkPos::new(6, 6)]);
        assert_eq!((arena.block(0, 64, 0), arena.liquid(1, 64, 0), arena.block(100, 64, 100), arena.changed_columns()), (stone, air, air, 0));
        assert_eq!(encoding(&arena, ORIGIN, 4), before, "the template's encoding again");
        assert!(arena.reset().is_empty() && World::void().reset().is_empty() && !World::void().is_arena());
        assert_eq!((arena.save_modified(), arena.save_and_wait()), (0, 0), "an arena is never saved");
    }

    #[test]
    fn a_template_from_storage_holds_every_stored_column_and_its_actors() {
        let storage = Arc::new(MemoryStorage::default());
        let registry = BlockRegistry::vanilla();
        let stone = registry.find("minecraft:stone", "").unwrap();
        for (x, actors) in [(0, vec![1u8, 2, 3]), (9, Vec::new())] {
            let mut chunk = Chunk::empty(x, 0, Dimension::overworld(registry.air_id()));
            chunk.set(0, 64, 0, 0, stone);
            storage.store_column(ChunkPos::new(x, 0), 0, &StoredColumn { chunk, block_entities: Vec::new(), actors }).unwrap();
        }
        let mut world = World::with_storage(storage);
        assert_eq!(world.load_all().unwrap(), 2);
        let template = Template::of(world);
        let mut arena = World::arena(&template);
        assert_eq!((template.columns(), arena.block(144, 64, 0), arena.actors(ORIGIN)), (2, stone, &[1u8, 2, 3][..]));
        // Its mobs are the arena's own: the columns that hold them are announced once.
        assert_eq!((arena.take_arrived(), arena.take_arrived()), (vec![ORIGIN], Vec::new()));
    }
}
