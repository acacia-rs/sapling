//! Where a world keeps its players: BDS's `player_<id>` mapping and `player_server_` record, and
//! the records sapling kept under its own key before.

use std::path::Path;

use acacia_proto::nbt::{self, LittleEndian, Nbt, Value};
use bytes::BytesMut;
use sapling_leveldb::{ItemStack, LevelDbWorld, PlayerData};

const ID: &str = "8d0f6b5e-3c1a-4e6f-9b52-1f0e2d3c4b5a";

fn world() -> LevelDbWorld {
    LevelDbWorld::open_in_memory(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/world")).expect("the fixture opens")
}

fn record(fields: Vec<(&str, Value)>) -> Vec<u8> {
    let mut out = BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: Value::Compound(fields.into_iter().map(|(key, value)| (key.into(), value)).collect()) });
    out.to_vec()
}

fn read(bytes: &[u8]) -> Value {
    nbt::read::<LittleEndian>(&mut &bytes[..]).unwrap().value
}

fn traveller() -> PlayerData {
    let diamonds = ItemStack { name: "minecraft:diamond".into(), count: 3, damage: 0, block: None, tag: None };
    PlayerData { main: vec![Some(diamonds)], dimension: 0, position: Some([12.5, 70.0, -3.5]), rotation: Some([90.0, 10.0]), game_mode: Some(2), ..PlayerData::default() }
}

#[test]
fn a_player_is_kept_under_the_vanilla_keys() {
    let world = world();
    assert_eq!(world.load_player(ID).unwrap(), None);
    world.store_player(ID, &traveller()).unwrap();
    assert_eq!(world.load_player(ID).unwrap(), Some(traveller()));

    let mapping = read(&world.db().get(format!("player_{ID}").as_bytes()).unwrap().unwrap());
    let server = format!("player_server_{ID}");
    assert_eq!(mapping.get("ServerId"), Some(&Value::String(server.as_str().into())));
    assert_eq!((mapping.get("MsaId"), mapping.get("SelfSignedId")), (Some(&Value::String(ID.into())), Some(&Value::String(ID.into()))));
    assert_eq!(read(&world.db().get(server.as_bytes()).unwrap().unwrap()).get("PlayerGameMode"), Some(&Value::Int(2)));
}

#[test]
fn a_record_bds_mapped_elsewhere_is_read_and_written_where_it_is() {
    let world = world();
    // BDS names a server record of its own choosing, and keeps fields sapling does not know.
    let server = "player_server_0a1b2c3d-0000-4000-8000-00000000beef";
    world.db().put(format!("player_{ID}").as_bytes(), &record(vec![("MsaId", Value::String(ID.into())), ("SelfSignedId", Value::String("".into())), ("ServerId", Value::String(server.into()))])).unwrap();
    let floats = |values: &[f32]| Value::List(nbt::List { tag: 5, items: values.iter().copied().map(Value::Float).collect() });
    let stored = vec![
        ("UniqueID", Value::Long(-4294967295)),
        ("identifier", Value::String("minecraft:player".into())),
        ("Pos", floats(&[0.5, -58.37999, 4.5])),
        ("Rotation", floats(&[-90.0, 30.0])),
        ("DimensionId", Value::Int(0)),
        ("PlayerGameMode", Value::Int(1)),
        ("SpawnX", Value::Int(i32::MIN)),
        ("SpawnY", Value::Int(i32::MIN)),
        ("SpawnZ", Value::Int(i32::MIN)),
        ("SpawnBlockPositionX", Value::Int(i32::MIN)),
        ("SpawnBlockPositionY", Value::Int(i32::MIN)),
        ("SpawnBlockPositionZ", Value::Int(i32::MIN)),
        ("SpawnDimension", Value::Int(3)),
    ];
    world.db().put(server.as_bytes(), &record(stored)).unwrap();

    let kept = world.load_player(ID).unwrap().unwrap();
    let feet = kept.position.unwrap();
    assert!((feet[1] - -60.0).abs() < 1e-4, "BDS keeps the eyes, 1.62 over the feet: {feet:?}");
    assert_eq!((kept.rotation, kept.game_mode, kept.bed), (Some([-90.0, 30.0]), Some(1), None));

    world.store_player(ID, &traveller()).unwrap();
    assert_eq!(world.db().get(format!("player_server_{ID}").as_bytes()).unwrap(), None, "no second record");
    let written = read(&world.db().get(server.as_bytes()).unwrap().unwrap());
    assert_eq!((written.get("UniqueID"), written.get("PlayerGameMode")), (Some(&Value::Long(-4294967295)), Some(&Value::Int(2))));
    assert_eq!(world.load_player(ID).unwrap(), Some(PlayerData { unique_id: Some(-4294967295), ..traveller() }), "BDS's id read back");
}

#[test]
fn a_record_sapling_kept_before_is_read_and_moved_at_the_next_save() {
    let world = world();
    let legacy = format!("sapling_player_{ID}");
    // Sapling's own record kept the feet in `Pos`.
    let floats = Value::List(nbt::List { tag: 5, items: vec![Value::Float(12.5), Value::Float(70.0), Value::Float(-3.5)] });
    world.db().put(legacy.as_bytes(), &record(vec![("PlayerLevel", Value::Int(9)), ("DimensionId", Value::Int(1)), ("Pos", floats)])).unwrap();
    let kept = world.load_player(ID).unwrap().unwrap();
    assert_eq!((kept.level, kept.dimension, kept.position), (9, 1, Some([12.5, 70.0, -3.5])));

    world.store_player(ID, &kept).unwrap();
    assert_eq!(world.db().get(legacy.as_bytes()).unwrap(), None);
    assert_eq!(world.load_player(ID).unwrap(), Some(kept));
}

#[test]
fn an_unreadable_mapping_is_an_error_not_a_first_visit() {
    let world = world();
    world.db().put(format!("player_{ID}").as_bytes(), &record(vec![("MsaId", Value::String(ID.into()))])).unwrap();
    assert!(world.load_player(ID).is_err());
    assert!(world.store_player(ID, &traveller()).is_err());
}
