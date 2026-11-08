//! What a world folder keeps of a mob beyond its body, as BDS saved it (research/captures/bds-f14nkeep-dump.txt):
//! its entity properties, a villager's pantry, a wandering trader's leaving time; and a kept mob that burns.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::AddEntity;
use acacia_proto::types::MetadataFlags1;
use common::asking::ask;
use common::raising::has_flag;
use sapling_api::{Item, MobSlot};
use common::stored::{actor_with, compound, enter_stored, floor_with};
use common::{Harness, all, config};
use sapling_world::{ChunkPos, MemoryStorage, World, WorldStorage, actor_chest, actor_field, stored_actors};

/// The compound storage keeps of the one actor of `kind` in column (0, 0).
fn saved(storage: &MemoryStorage, kind: &str) -> Vec<u8> {
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let mut found = stored_actors(&column.actors).unwrap().into_iter().filter(|(actor, ..)| actor.as_ref().is_some_and(|actor| actor.identifier == kind));
    found.next().map(|(.., compound)| compound).unwrap_or_else(|| panic!("no {kind} saved"))
}

/// The player leaves and the column is saved and let go.
fn save(h: &mut Harness) {
    h.leave(1);
    (0..150).for_each(|_| h.shard.tick());
}

#[test]
fn a_kept_mob_comes_with_its_properties_and_on_fire() {
    let properties = compound([("minecraft:climate_variant", Value::String("warm".into())), ("minecraft:sound_variant", Value::String("moody".into()))]);
    let cow = actor_with(-901, "minecraft:cow", [5.5, 64.0, 5.5], vec![("properties", properties), ("Fire", Value::Short(100))]);
    let mut h = Harness::with(config(), World::with_storage(floor_with(cow)));
    let sent: Vec<RawPacket> = enter_stored(&mut h, 1);
    let added = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == "minecraft:cow").expect("the cow shown");
    let climate = sapling_data::entity_property("minecraft:cow", "minecraft:climate_variant").unwrap();
    let shown = added.properties.ints.iter().find(|item| item.index == climate.index).map(|item| item.value);
    assert_eq!(shown, climate.value_of("warm"), "its kept climate: {:?}", added.properties);
    assert_eq!(has_flag(&sent, added.runtime_id, MetadataFlags1::ONFIRE), Some(true), "shown burning");
}

#[test]
fn a_spawned_mob_is_saved_with_its_properties() {
    let storage = floor_with(Vec::new());
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:pig", [5.5, 64.0, 5.5]).is_some()));
    (0..5).for_each(|_| drop(h.tick(1)));
    save(&mut h);
    let kept = actor_field(&saved(&storage, "minecraft:pig"), "properties").expect("properties kept");
    assert!(matches!(kept.get("minecraft:climate_variant"), Some(Value::String(climate)) if climate == "temperate"), "{kept:?}");
    assert!(matches!(kept.get("minecraft:sound_variant"), Some(Value::String(_))), "{kept:?}");
}

#[test]
fn a_villager_keeps_its_pantry() {
    let bread = compound([("Count", Value::Byte(3)), ("Damage", Value::Short(0)), ("Name", Value::String("minecraft:bread".into())), ("Slot", Value::Byte(2)), ("WasPickedUp", Value::Byte(0))]);
    let pantry = ("ChestItems", Value::List(List { tag: 10, items: vec![bread] }));
    let villager = actor_with(-902, "minecraft:villager_v2", [5.5, 64.0, 5.5], vec![pantry]);
    let storage = floor_with(villager);
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    let villager = ask(&mut h, |ctx| ctx.entities().into_iter().find(|entity| entity.kind == "minecraft:villager_v2").map(|entity| entity.id)).expect("the villager");
    assert!(ask(&mut h, move |ctx| ctx.set_mob_item(villager, MobSlot::Inventory(0), Some(&Item::new("minecraft:emerald", 5)))).is_ok());
    // Saved while Alex is there: a column nobody sees keeps its record as it was (actors.rs `write_back`).
    h.shard.save_worlds();
    let held: Vec<(u8, String, u8)> = actor_chest(&saved(&storage, "minecraft:villager_v2")).into_iter().map(|(slot, stack)| (slot, stack.name, stack.count)).collect();
    assert_eq!(held, [(0, "minecraft:emerald".into(), 5), (2, "minecraft:bread".into(), 3)]);
}

#[test]
fn a_wandering_trader_keeps_the_tick_it_leaves_at() {
    let trader = actor_with(-903, "minecraft:wandering_trader", [5.5, 64.0, 5.5], vec![("TimeStamp", Value::Long(50_000))]);
    let storage = floor_with(trader);
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:wandering_trader", [9.5, 64.0, 9.5]).is_some()));
    (0..5).for_each(|_| drop(h.tick(1)));
    save(&mut h);
    let column = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let mut stamps: Vec<(i64, i64)> = stored_actors(&column.actors)
        .unwrap()
        .into_iter()
        .filter(|(actor, ..)| actor.as_ref().is_some_and(|actor| actor.identifier == "minecraft:wandering_trader"))
        .filter_map(|(_, id, compound)| Some((id?, if let Some(Value::Long(stamp)) = actor_field(&compound, "TimeStamp") { stamp } else { -1 })))
        .collect();
    stamps.sort();
    assert_eq!(stamps.len(), 2, "{stamps:?}");
    assert_eq!(stamps.iter().find(|(id, _)| *id == -903).map(|(_, stamp)| *stamp), Some(50_000), "the kept one's stamp as it was: {stamps:?}");
    let new = stamps.iter().find(|(id, _)| *id != -903).map(|(_, stamp)| *stamp).unwrap();
    assert!((48_000..=72_400).contains(&new), "a new one's is 2400 or 3600 s on: {new}");
}
