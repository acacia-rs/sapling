//! Where a world folder keeps the mobs it moves: each in exactly one column at every save.

use crate::common;

use std::sync::{Arc, mpsc};

use acacia_proto::nbt::Value;
use common::stored::{actor, enter_stored};
use common::{Harness, config};
use sapling_api::EntityInfo;
use sapling_world::{BlockRegistry, Chunk, ChunkPos, Dimension, MemoryStorage, StoredColumn, World, WorldStorage};

const HORSE: i64 = -5;

/// Storage with stone floors at y 63 in columns (0, 0) to (3, 0) and the horse in column (2, 0).
fn floors_with_a_horse() -> Arc<MemoryStorage> {
    let registry = BlockRegistry::vanilla();
    let stone = registry.states_of("minecraft:stone").next().unwrap().0;
    let storage = Arc::new(MemoryStorage::default());
    for x in 0..4 {
        let mut chunk = Chunk::empty(x, 0, Dimension::overworld(registry.air_id()));
        (0..16).for_each(|x| (0..16).for_each(|z| chunk.set(x, 63, z, 0, stone)));
        let actors = if x == 2 { actor(HORSE, "minecraft:horse", [40.5, 64.0, 8.5], ("Variant", Value::Int(3))) } else { Vec::new() };
        storage.store_column(ChunkPos::new(x, 0), 0, &StoredColumn { actors, ..StoredColumn::of(chunk) }).unwrap();
    }
    storage
}

fn horse(h: &mut Harness) -> Option<EntityInfo> {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.entities()).unwrap());
    h.tick_all();
    rx.recv().unwrap().into_iter().find(|entity| entity.kind == "minecraft:horse")
}

/// The columns storage keeps the horse in, with where it stands in each.
fn stored_horses(storage: &MemoryStorage) -> Vec<(ChunkPos, [f32; 3])> {
    let columns = (-2..8).flat_map(|x| (-2..3).map(move |z| ChunkPos::new(x, z)));
    let stored = columns.filter_map(|pos| Some((pos, storage.load_column(pos, 0).unwrap()?)));
    let actors = stored.flat_map(|(pos, column)| sapling_world::stored_actors(&column.actors).unwrap().into_iter().map(move |actor| (pos, actor)));
    actors.filter(|(_, (_, id, _))| *id == Some(HORSE)).map(|(pos, (actor, ..))| (pos, actor.unwrap().position)).collect()
}

#[test]
fn a_mob_that_walked_out_of_the_loaded_columns_stays_kept() {
    let storage = floors_with_a_horse();
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    let id = horse(&mut h).expect("the horse arrived with its column").id;
    // Column (6, 0) is far outside the player's view and never loaded.
    h.remote.run(move |ctx| assert!(ctx.move_entity(id, [100.5, 64.0, 8.5])));
    h.tick_all();
    h.shard.save_worlds();
    let kept = stored_horses(&storage);
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!(kept[0].0, ChunkPos::new(2, 0), "kept where it was filed while its own column is not loaded");
}

#[test]
fn a_mob_that_walked_into_an_unseen_column_is_kept_once() {
    let storage = floors_with_a_horse();
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    let id = horse(&mut h).expect("the horse arrived with its column").id;
    h.remote.run(move |ctx| assert!(ctx.move_entity(id, [24.5, 64.0, 8.5])));
    h.tick_all();
    h.leave(1);
    // Sweeps with nobody to see: the columns are saved, then dropped with the horse.
    (0..350).for_each(|_| h.shard.tick());
    assert!(horse(&mut h).is_none(), "the horse left with its column");
    h.shard.save_worlds();
    let kept = stored_horses(&storage);
    assert_eq!(kept.iter().map(|(pos, at)| (*pos, at[0])).collect::<Vec<_>>(), [(ChunkPos::new(1, 0), 24.5)]);
}
