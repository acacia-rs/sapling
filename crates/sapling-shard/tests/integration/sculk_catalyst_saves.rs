//! A catalyst's cursors kept in its block actor, `cursors`, as BDS saves them (docs/research/sculk.md,
//! "Catalyst"), and taken up again when its column comes back from storage.

use crate::common;

use std::sync::{Arc, mpsc};

use acacia_proto::nbt::{self, LittleEndian, List, Nbt, Value};
use acacia_proto::packets::LevelEventGeneric;
use bytes::BytesMut;
use common::{Harness, all, config};
use sapling_api::{Block, PlayerId};
use sapling_world::{BlockRegistry, Chunk, ChunkPos, Dimension, MemoryStorage, OVERWORLD, StoredColumn, World, WorldStorage};

const ALEX: PlayerId = PlayerId(1);
const CATALYST: [i32; 3] = [2, 65, 2];
/// `LevelEventGeneric` 2037, a cursor's charge shown, zigzagged.
const CHARGE: u32 = 2037 << 1;

fn compound(fields: Vec<(&str, Value)>) -> Value {
    Value::Compound(fields.into_iter().map(|(key, value)| (nbt::Str::from(key), value)).collect())
}

fn saved_cursors(h: &mut Harness) -> Vec<Value> {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.block_entity(CATALYST).and_then(|data| data.get::<Value>().cloned())).unwrap());
    h.tick_all();
    match rx.recv().unwrap().as_ref().and_then(|entity| entity.get("cursors")) {
        Some(Value::List(list)) => list.items.clone(),
        _ => Vec::new(),
    }
}

#[test]
fn a_catalysts_cursors_are_kept_in_its_block_actor() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| ctx.teleport(ALEX, [-5.5, 65.0, -5.5]));
    h.remote.run(|ctx| assert!(ctx.update_block(CATALYST, &Block::new("sculk_catalyst"))));
    (0..4).for_each(|_| _ = h.tick_all());
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.spawn_entity("minecraft:cow", [4.5, 65.0, 4.5]).expect("a cow")).unwrap());
    h.tick_all();
    let cow = rx.recv().unwrap();
    h.remote.run(move |ctx| assert!(ctx.kill_entity(cow)));
    (0..3).for_each(|_| _ = h.tick_all());
    let cursors = saved_cursors(&mut h);
    let charged = |cursor: &Value| matches!(cursor.get("charge"), Some(Value::Short(charge)) if *charge > 0) && matches!(cursor.get("x"), Some(Value::Int(_)));
    assert!(!cursors.is_empty() && cursors.iter().all(charged), "{cursors:?}");
}

/// A stored world: stone at y 64 over x and z -8..8, and a catalyst at (2, 65, 2) holding one
/// cursor of 30 charge at (4, 65, 4).
fn stored() -> Arc<MemoryStorage> {
    let registry = BlockRegistry::vanilla();
    let [stone, catalyst] = ["minecraft:stone", "minecraft:sculk_catalyst"].map(|name| registry.states_of(name).next().unwrap().0);
    let storage = Arc::new(MemoryStorage::default());
    for cx in -1..=0 {
        for cz in -1..=0 {
            let mut chunk = Chunk::empty(cx, cz, Dimension::from_id(OVERWORLD, registry.air_id()));
            (0..16).flat_map(|x| (0..16).map(move |z| (x, z))).for_each(|(x, z)| chunk.set(x, 64, z, 0, stone));
            let mut column = StoredColumn::of(chunk);
            if ChunkPos::new(cx, cz) == ChunkPos::of_block(CATALYST[0], CATALYST[2]) {
                column.chunk.set(CATALYST[0], CATALYST[1], CATALYST[2], 0, catalyst);
                let cursor = compound(vec![("x", Value::Int(4)), ("y", Value::Int(65)), ("z", Value::Int(4)), ("charge", Value::Short(30)), ("update", Value::Short(0)), ("decay", Value::Short(1)), ("facing", Value::Short(-1))]);
                let mut fields = vec![("id", Value::String("SculkCatalyst".into())), ("cursors", Value::List(List { tag: 10, items: vec![cursor] }))];
                fields.extend([("x", Value::Int(CATALYST[0])), ("y", Value::Int(CATALYST[1])), ("z", Value::Int(CATALYST[2]))]);
                let mut bytes = BytesMut::new();
                nbt::write::<LittleEndian>(&mut bytes, &Nbt { name: String::new(), value: compound(fields) });
                column.block_entities = bytes.to_vec();
            }
            storage.store_column(ChunkPos::new(cx, cz), OVERWORLD, &column).unwrap();
        }
    }
    storage
}

#[test]
fn saved_cursors_move_once_their_column_is_loaded() {
    let mut h = Harness::with(config(), World::with_storage(stored()));
    h.enter(1, "Alex");
    h.remote.run(|ctx| ctx.teleport(ALEX, [-5.5, 65.0, -5.5]));
    let shown = (0..200).any(|_| all::<LevelEventGeneric>(&h.tick_all().to(1).to_vec()).iter().any(|event| event.event_id == CHARGE));
    assert!(shown, "the saved charge is shown moving");
}
