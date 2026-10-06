//! Columns as a world folder keeps them, for tests that start from storage.

use std::sync::Arc;
use std::time::{Duration, Instant};

use acacia_proto::RawPacket;
use acacia_proto::nbt::{self, List, LittleEndian, Nbt, Value};
use acacia_proto::packets::{PlayStatus, SetLocalPlayerAsInitialized};
use bytes::BytesMut;
use sapling_world::{BlockRegistry, Chunk, ChunkPos, Dimension, MemoryStorage, StoredColumn, WorldStorage};

use super::{Harness, all};

pub fn compound<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Compound(fields.map(|(key, value)| (key.to_string(), value)).into())
}

/// A stored actor with one field of its kind's own.
pub fn actor(id: i64, identifier: &str, feet: [f32; 3], own: (&str, Value)) -> Vec<u8> {
    let floats = |items: &[f32]| Value::List(List { tag: 5, items: items.iter().map(|&item| Value::Float(item)).collect() });
    let fields = [("UniqueID", Value::Long(id)), ("identifier", Value::String(identifier.into())), ("Pos", floats(&feet)), ("Rotation", floats(&[90.0, 0.0])), own];
    let mut out = BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: compound(fields) });
    out.to_vec()
}

/// Storage holding column (0, 0) with stone at y 63 and `actors`.
pub fn floor_with(actors: Vec<u8>) -> Arc<MemoryStorage> {
    let registry = BlockRegistry::vanilla();
    let mut chunk = Chunk::empty(0, 0, Dimension::overworld(registry.air_id()));
    let stone = registry.states_of("minecraft:stone").next().unwrap().0;
    (0..16).for_each(|x| (0..16).for_each(|z| chunk.set(x, 63, z, 0, stone)));
    let storage = Arc::new(MemoryStorage::default());
    storage.store_column(ChunkPos::new(0, 0), 0, &StoredColumn { actors, ..StoredColumn::of(chunk) }).unwrap();
    storage
}

/// Joins `conn` and ticks until it is in the world; returns everything it was sent.
pub fn enter_stored(h: &mut Harness, conn: u64) -> Vec<RawPacket> {
    h.join_to_radius(conn, 2);
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut sent = Vec::new();
    while all::<PlayStatus>(&sent).is_empty() {
        assert!(Instant::now() < deadline, "the player never spawned");
        sent.extend(h.tick(conn));
        std::thread::sleep(Duration::from_millis(1));
    }
    h.receive(conn, &SetLocalPlayerAsInitialized { runtime_entity_id: 0 });
    (0..3).for_each(|_| sent.extend(h.tick(conn)));
    sent
}
