//! Mobs a structure's template placed in a generated column (an outpost's pillagers): made once as
//! the column arrives, kept with it as any mob, not made again (docs/research/structure-spawns.md
//! "Template actors").

use crate::common;

use std::sync::{Arc, mpsc};

use common::stored::enter_stored;
use common::{Harness, config};
use sapling_api::{Difficulty, EntityInfo};
use sapling_shard::ShardConfig;
use sapling_world::{BlockRegistry, Chunk, ChunkPos, Dimension, MemoryStorage, StoredColumn, World, WorldStorage};
use sapling_worldgen::TemplateActor;

/// Storage that generates column (0, 0), a stone floor at y 63 with a captain on it, where `inner` has none.
struct Generating(Arc<MemoryStorage>);

impl WorldStorage for Generating {
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, sapling_world::StorageError> {
        if let Some(stored) = self.0.load_column(pos, dimension)? {
            return Ok(Some(stored));
        }
        let registry = BlockRegistry::vanilla();
        let mut chunk = Chunk::empty(pos.x, pos.z, Dimension::overworld(registry.air_id()));
        let stone = registry.states_of("minecraft:stone").next().unwrap().0;
        (0..16).for_each(|x| (0..16).for_each(|z| chunk.set(x, 63, z, 0, stone)));
        let captain = TemplateActor { kind: "minecraft:pillager", event: "minecraft:spawn_as_illager_captain", feet: [pos.x as f32 * 16.0 + 8.0, 64.0, pos.z as f32 * 16.0 + 8.0] };
        let template_actors = if pos == ChunkPos::new(0, 0) { vec![captain] } else { Vec::new() };
        Ok(Some(StoredColumn { template_actors, ..StoredColumn::of(chunk) }))
    }

    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), sapling_world::StorageError> {
        self.0.store_column(pos, dimension, column)
    }
}

fn pillagers(h: &mut Harness) -> Vec<EntityInfo> {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.entities()).unwrap());
    h.tick_all();
    rx.recv().unwrap().into_iter().filter(|entity| entity.kind == "minecraft:pillager").collect()
}

#[test]
fn a_template_mob_comes_once_with_its_column_and_stays_with_it() {
    let inner = Arc::new(MemoryStorage::default());
    let mut h = Harness::with(ShardConfig { difficulty: Difficulty::Normal, ..config() }, World::with_storage(Arc::new(Generating(inner.clone()))));
    enter_stored(&mut h, 1);
    let first = pillagers(&mut h);
    assert_eq!(first.len(), 1, "{first:?}");
    assert!((first[0].feet[0] - 8.0).abs() < 0.01, "at its cell's corner: {first:?}");
    h.shard.save_worlds();
    let stored = inner.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("saved with its mob");
    assert!(stored.template_actors.is_empty());
    let kept = sapling_world::stored_actors(&stored.actors).unwrap();
    assert!(kept.iter().any(|(actor, ..)| actor.as_ref().is_some_and(|actor| actor.identifier == "minecraft:pillager")), "{kept:?}");
    h.leave(1);
    (0..350).for_each(|_| h.shard.tick());
    enter_stored(&mut h, 2);
    assert_eq!(pillagers(&mut h).len(), 1, "back from storage, not made again");
}
