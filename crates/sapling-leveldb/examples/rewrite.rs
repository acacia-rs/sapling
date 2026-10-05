//! Copies a world folder and writes every overworld column of the copy back through
//! `LevelDbWorld::store_column`, with a marker in each: the world a vanilla server is then started
//! on to see whether it reads what sapling writes (`bds_world_check` in the sapling crate).
//! `cargo run -p sapling-leveldb --example rewrite -- <world> <new world>`
//!
//! Generated terrain cannot tell a column the game read from one it threw away and generated
//! again, so each column gets blocks no generator places, at local (8, 200.., 8): a diamond block,
//! waterlogged stairs sealed in glass (two layers; a byte and an int state), a log (a string
//! state) and a composter (an int state), none of them in its block's default state. Its actors
//! are marked too (`marked_actors`; `bds_entity_probe` lists the names a server then announces).
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use acacia_proto::nbt::{self, List, LittleEndian, Nbt, Value};
use acacia_world::BlockRegistry;
use bytes::BytesMut;
use sapling_leveldb::LevelDbWorld;

const MARKER_Y: i32 = 200;
/// The name every actor the column already had is given.
const KEPT: &str = "sapling kept";
/// The name of the sheep each column gets on top of its marker, in [`MADE_WOOL`].
const MADE: &str = "sapling made";
const MADE_WOOL: i8 = 6;
/// Far from the ids a game hands out, which count up from a small session number in the high half.
const FIRST_MADE_ID: i64 = -(900 << 32);

/// The column's actors, each renamed [`KEPT`], and a sheep, an armour stand and a painting with
/// no more than sapling's `Actor` reads: whether the game takes an actor that sparse is part of
/// what is being checked.
fn marked_actors(record: &[u8], made: i64, feet: [f32; 3]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut out = BytesMut::new();
    let mut rest = record;
    while !rest.is_empty() {
        let mut actor = nbt::read::<LittleEndian>(&mut rest)?;
        if let Value::Compound(fields) = &mut actor.value {
            fields.retain(|(key, _)| key != "CustomName");
            fields.push(("CustomName".into(), Value::String(KEPT.into())));
        }
        nbt::write::<LittleEndian>(&mut out, &actor);
    }
    let floats = |items: &[f32]| Value::List(List { tag: 5, items: items.iter().map(|&item| Value::Float(item)).collect() });
    let sheep = [
        ("UniqueID", Value::Long(FIRST_MADE_ID - made)),
        ("identifier", Value::String("minecraft:sheep".into())),
        ("Pos", floats(&feet)),
        ("Rotation", floats(&[90.0, 0.0])),
        ("Color", Value::Byte(MADE_WOOL)),
        ("CustomName", Value::String(MADE.into())),
    ];
    let compound = |fields: Vec<(&str, Value)>| Value::Compound(fields.into_iter().map(|(key, value)| (key.to_string(), value)).collect());
    let stack = |name: &str| compound(vec![("Name", Value::String(name.into())), ("Count", Value::Byte(if name.is_empty() { 0 } else { 1 })), ("Damage", Value::Short(0))]);
    let stacks = |names: &[&str]| Value::List(List { tag: 10, items: names.iter().map(|name| stack(name)).collect() });
    // Which slot BDS takes each list entry for is part of what is being checked.
    let stand = vec![
        ("UniqueID", Value::Long(FIRST_MADE_ID - (1 << 20) - made)),
        ("identifier", Value::String("minecraft:armor_stand".into())),
        ("Pos", floats(&feet)),
        ("Rotation", floats(&[90.0, 0.0])),
        ("Armor", stacks(&["minecraft:diamond_helmet", "", "", "minecraft:golden_boots"])),
        ("Mainhand", stacks(&["minecraft:iron_sword"])),
        ("Offhand", stacks(&["minecraft:shield"])),
    ];
    // On the south face of the marker's diamond block.
    let painting = vec![
        ("UniqueID", Value::Long(FIRST_MADE_ID - (2 << 20) - made)),
        ("identifier", Value::String("minecraft:painting".into())),
        ("Pos", floats(&[feet[0], MARKER_Y as f32 + 0.5, feet[2] + 0.53125])),
        ("Rotation", floats(&[0.0, 0.0])),
        ("Direction", Value::Byte(0)),
        ("Motif", Value::String("Kebab".into())),
        ("Motive", Value::String("Kebab".into())),
    ];
    for actor in [compound(sheep.into()), compound(stand), compound(painting)] {
        nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: actor });
    }
    Ok(out.to_vec())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [from, to] = &args[..] else { return Err("usage: rewrite <world> <new world>".into()) };
    let (from, to) = (PathBuf::from(from), PathBuf::from(to));
    std::fs::create_dir_all(to.join("db"))?;
    for entry in std::fs::read_dir(from.join("db"))? {
        let file = entry?.file_name();
        std::fs::copy(from.join("db").join(&file), to.join("db").join(&file))?;
    }
    std::fs::copy(from.join("level.dat"), to.join("level.dat"))?;

    let registry = BlockRegistry::vanilla();
    let last = |name: &str| registry.states_of(name).last().map(|(id, _)| id).ok_or(format!("no block {name}"));
    // The game recomputes a stair's corner from its neighbours: alone, anything but `none` is corrected.
    let stairs = registry.find("minecraft:oak_stairs", "minecraft:corner=none,upside_down_bit=1,weirdo_direction=3").ok_or("no such stairs")?;
    let (diamond, log, composter) = (last("minecraft:diamond_block")?, last("minecraft:oak_log")?, last("minecraft:composter")?);
    let (glass, water) = (last("minecraft:glass")?, registry.find("minecraft:water", "liquid_depth=0").ok_or("no water source")?);

    let world = LevelDbWorld::open(&to)?;
    let columns: Vec<_> = world.columns()?.into_iter().filter(|key| key.dimension == 0).collect();
    let mut made = 0;
    for key in &columns {
        let mut column = world.load_column(*key)?.ok_or("a listed column")?;
        let (x, z) = (key.x * 16 + 8, key.z * 16 + 8);
        for (dy, id) in [diamond, stairs, log, composter].into_iter().enumerate() {
            column.chunk.set(x, MARKER_Y + dy as i32, z, 0, id);
        }
        column.chunk.set(x, MARKER_Y + 1, z, 1, water);
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            column.chunk.set(x + dx, MARKER_Y + 1, z + dz, 0, glass);
        }
        let actors = marked_actors(&column.actors, made, [x as f32 + 0.5, (MARKER_Y + 4) as f32, z as f32 + 0.5])?;
        made += 1;
        world.store_column(*key, &column.chunk, &column.block_entities, &actors)?;
    }
    let mut level = world.level().clone();
    level.set_last_played(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64);
    world.flush(&level)?;
    println!("rewrote {} columns into {}", columns.len(), to.display());
    Ok(())
}
