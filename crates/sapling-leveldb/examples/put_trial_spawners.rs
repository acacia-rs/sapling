//! Puts trial spawners with inline configs into a world folder no server has open, one per guess at the form of
//! a `spawn_potentials` entry, each naming another kind, to see which a server spawns and how it saves them
//! (drill `f14strial`): `cargo run -p sapling-leveldb --example put_trial_spawners -- <world> <x> <y> <z>`
//! puts them 4 apart along x.
use std::path::PathBuf;

use acacia_proto::nbt::{self, LittleEndian, List, Nbt, Value};
use sapling_leveldb::{ChunkKey, LevelDbWorld};

const COMPOUND: u8 = 10;

fn compound(fields: Vec<(&str, Value)>) -> Value {
    Value::Compound(fields.into_iter().map(|(key, value)| (key.into(), value)).collect())
}

fn text(value: &str) -> Value {
    Value::String(value.into())
}

/// The entry forms tried: a monster spawner's `SpawnData`, Java's `{data: {entity: {id}}, weight}`, and
/// `SpawnData` nested in `data`.
fn potentials() -> [(&'static str, Value); 3] {
    [
        ("pig", compound(vec![("TypeId", text("minecraft:pig")), ("Weight", Value::Int(1))])),
        ("cow", compound(vec![("data", compound(vec![("entity", compound(vec![("id", text("minecraft:cow"))]))])), ("weight", Value::Int(1))])),
        ("sheep", compound(vec![("data", compound(vec![("TypeId", text("minecraft:sheep"))])), ("weight", Value::Int(1))])),
    ]
}

fn spawner([x, y, z]: [i32; 3], potential: Value) -> Value {
    let config = compound(vec![
        ("simultaneous_mobs", Value::Float(1.0)),
        ("total_mobs", Value::Float(2.0)),
        ("ticks_between_spawn", Value::Int(20)),
        ("spawn_potentials", Value::List(List { tag: COMPOUND, items: vec![potential] })),
    ]);
    compound(vec![
        ("BlockEntityVersion", Value::Int(0)),
        ("cooldown_end_at", Value::Long(0)),
        ("current_mobs", Value::List(List { tag: 0, items: Vec::new() })),
        ("id", text("TrialSpawner")),
        ("next_mob_spawns_at", Value::Long(0)),
        ("normal_config", config.clone()),
        ("ominous_config", config),
        ("registered_players", Value::List(List { tag: 0, items: Vec::new() })),
        ("required_player_range", Value::Int(14)),
        ("total_mobs_spawned", Value::Int(0)),
        ("x", Value::Int(x)),
        ("y", Value::Int(y)),
        ("z", Value::Int(z)),
    ])
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [folder, x, y, z] = &args[..] else { return Err("usage: put_trial_spawners <world> <x> <y> <z>".into()) };
    let [x, y, z]: [i32; 3] = [x.parse()?, y.parse()?, z.parse()?];
    let world = LevelDbWorld::open(&PathBuf::from(folder))?;
    let block = world.registry().find("minecraft:trial_spawner", "ominous=0,trial_spawner_state=0").ok_or("no trial spawner state")?;
    for (index, (kind, potential)) in potentials().into_iter().enumerate() {
        let at = [x + 4 * index as i32, y, z];
        let key = ChunkKey { x: at[0] >> 4, z: at[2] >> 4, dimension: 0 };
        let mut column = world.load_column(key)?.ok_or("no column there")?;
        column.chunk.set(at[0] & 15, y, at[2] & 15, 0, block);
        let mut entities = sapling_leveldb::without_block_entity(&column.block_entities, at)?;
        let mut written = bytes::BytesMut::new();
        nbt::write::<LittleEndian>(&mut written, &Nbt { name: String::new(), value: spawner(at, potential) });
        entities.extend_from_slice(&written);
        world.store_column(key, &column.chunk, &entities, &column.actors, &column.pending_ticks)?;
        println!("{kind} spawner at {at:?}");
    }
    world.flush(world.level())?;
    Ok(())
}
