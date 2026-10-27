use acacia_proto::nbt::{self, LittleEndian, List, Value};
use acacia_world::BlockRegistry;

use super::*;

fn stack(name: &str, count: u8, block: Option<u32>) -> Option<ItemStack> {
    Some(ItemStack { name: name.into(), count, damage: 0, block, tag: None })
}

fn fields(record: &[u8]) -> Value {
    nbt::read::<LittleEndian>(&mut &record[..]).unwrap().value
}

fn player() -> PlayerData {
    let registry = BlockRegistry::vanilla();
    let stone = registry.network_hash(registry.states_of("minecraft:stone").next().unwrap().0);
    let sword = ItemStack { tag: Some(Value::Compound(vec![("Damage".into(), Value::Int(3))])), ..stack("minecraft:iron_sword", 1, None).unwrap() };
    let mut main = vec![None; 36];
    (main[0], main[8], main[35]) = (stack("minecraft:stone", 64, Some(stone)), Some(sword), stack("minecraft:diamond", 5, None));
    let ender = vec![None, None, stack("minecraft:emerald", 12, None)];
    let armor = vec![stack("minecraft:iron_helmet", 1, None), None, None, None];
    PlayerData {
        main, armor, offhand: stack("minecraft:shield", 1, None), selected: 8, ender, level: 17, progress: 0.25, vitals: Some([7.5, 18.0, 0.0, 2.25]),
        bed: Some([[5, 64, -3], [4, 64, -3]]), spawn_dimension: 1, last_death: Some(([-2, 101, 2], 1)), effects: vec![[1, 1, 600, 1], [19, 0, 40, 0]], dimension: 1, position: Some([-14.5, 34.0, 7.5]),
        rotation: Some([135.0, -12.5]), game_mode: Some(1), enchant_seed: Some(-123_456), tags: vec!["vip".into(), "§cred".into()],
        unique_id: Some(-4294967295),
    }
}

#[test]
fn a_player_reads_back_as_it_was_written() {
    let registry = BlockRegistry::vanilla();
    let player = player();
    assert_eq!(PlayerData::decode(&player.encode(&registry)).unwrap(), player);

    let empty = PlayerData::decode(&PlayerData::default().encode(&registry)).unwrap();
    assert_eq!(empty, PlayerData::default());
    assert!(PlayerData::decode(&[1, 2, 3]).is_err());
}

#[test]
fn the_record_has_bds_fields_and_shapes() {
    let record = fields(&player().encode(&BlockRegistry::vanilla()));
    // BDS keeps a player's position at its eyes.
    assert_eq!(record.get("Pos"), Some(&Value::List(List { tag: 5, items: vec![Value::Float(-14.5), Value::Float(34.0 + EYE_HEIGHT), Value::Float(7.5)] })));
    assert_eq!(record.get("Rotation"), Some(&Value::List(List { tag: 5, items: vec![Value::Float(135.0), Value::Float(-12.5)] })));
    assert_eq!((record.get("PlayerGameMode"), record.get("DimensionId"), record.get("SpawnDimension")), (Some(&Value::Int(1)), Some(&Value::Int(1)), Some(&Value::Int(1))));
    let death = ["DeathPositionX", "DeathPositionY", "DeathDimension", "HasDiedBefore"].map(|key| record.get(key).cloned());
    assert_eq!(death, [Value::Int(-2), Value::Int(101), Value::Int(1), Value::Byte(1)].map(Some));
    assert_eq!(record.get("EnchantmentSeed"), Some(&Value::Int(-123_456)));
    let Some(Value::List(effects)) = record.get("ActiveEffects") else { panic!("no effects") };
    let speed = &effects.items[0];
    let parts = ["Id", "Amplifier", "Duration", "DurationEasy", "DurationNormal", "DurationHard", "Ambient", "ShowParticles"].map(|key| speed.get(key).cloned());
    assert_eq!(parts, [Value::Byte(1), Value::Byte(1), Value::Int(600), Value::Int(600), Value::Int(600), Value::Int(600), Value::Byte(0), Value::Byte(1)].map(Some));
    // No spawn point of its own: BDS's empty spawn fields.
    let homeless = fields(&PlayerData::default().encode(&BlockRegistry::vanilla()));
    let spawn = ["SpawnX", "SpawnBlockPositionZ", "SpawnDimension"].map(|key| homeless.get(key).cloned());
    assert_eq!(spawn, [Value::Int(i32::MIN), Value::Int(i32::MIN), Value::Int(3)].map(Some));
}

#[test]
fn writing_over_a_record_keeps_what_sapling_does_not_know() {
    let registry = BlockRegistry::vanilla();
    let attribute = |name: &str, base: f32, current: f32| Value::Compound(vec![("Base".into(), Value::Float(base)), ("Current".into(), Value::Float(current)), ("Name".into(), Value::String(name.into()))]);
    let attributes = vec![attribute("minecraft:health", 20.0, 20.0), attribute("minecraft:movement", 0.1, 0.1)];
    let stored = Value::Compound(vec![
        ("UniqueID".into(), Value::Long(-77)),
        ("Attributes".into(), Value::List(List { tag: 10, items: attributes })),
        ("Pos".into(), Value::List(List { tag: 5, items: vec![Value::Float(1.0); 3] })),
        ("ActiveEffects".into(), Value::List(List { tag: 10, items: vec![parts::effect_fields(&[1, 0, 20, 1])] })),
    ]);
    let mut previous = bytes::BytesMut::new();
    nbt::write::<LittleEndian>(&mut previous, &Nbt { name: String::new(), value: stored });
    let player = PlayerData { vitals: Some([9.0, 20.0, 5.0, 0.0]), ..PlayerData::default() };
    let record = fields(&player.encode_over(&registry, Some(&previous)).unwrap());

    assert_eq!((record.get("UniqueID"), record.get("Tags")), (Some(&Value::Long(-77)), None));
    let tags = Value::List(List { tag: 8, items: vec![Value::String("vip".into()), Value::String("§cred".into())] });
    assert_eq!(fields(&self::player().encode(&registry)).get("Tags"), Some(&tags), "a list of strings, as BDS keeps them");
    let Some(Value::List(kept)) = record.get("Attributes") else { panic!("no attributes") };
    assert_eq!(kept.items[..2], [attribute("minecraft:health", 20.0, 9.0), attribute("minecraft:movement", 0.1, 0.1)]);
    assert_eq!(kept.items.len(), 5, "the vitals it lacked are added");
    // Without a place or effects of its own, none of the old ones stay.
    assert_eq!((record.get("Pos"), record.get("ActiveEffects")), (None, None));
    assert!(player.encode_over(&registry, Some(&[1, 2, 3])).is_err(), "an unreadable record is never written over");
}

#[test]
fn a_legacy_record_has_its_position_at_the_feet() {
    let registry = BlockRegistry::vanilla();
    let player = player();
    let record = player.encode(&registry);
    assert_eq!(PlayerData::decode_legacy(&record).unwrap().position, Some([-14.5, 34.0 + EYE_HEIGHT, 7.5]));
}
