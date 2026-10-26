//! What animals give a player's tools and containers, what they lay and graze, and name tags
//! (creatures/husbandry/products.rs, docs/research/husbandry.md).

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::nbt::Value;
use acacia_proto::packets::{AddEntity, EntityEventEventId as Event, RemoveEntity};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags};
use common::raising::{click_on, has_flag, set_in, showed};
use common::shooting::{ALEX, added, held, ready, spawn, watch};
use common::stored::{compound, enter_stored, floor_holding};
use common::{Harness, all, config};
use sapling_api::{Block, GameMode, Item, Slot};
use sapling_world::{ChunkPos, ItemStack, PlayerData, World, WorldStorage};

/// Alex holds `count` of `name` in the slot it uses.
fn hold(h: &mut Harness, name: &'static str, count: u8) {
    h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new(name, count)))));
    h.tick_all();
}

/// The items lying about, by name and count.
fn lying(h: &mut Harness) -> Vec<(String, u8)> {
    let found = Arc::new(Mutex::new(Vec::new()));
    let out = found.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = ctx.entities().into_iter().filter_map(|entity| entity.item).map(|item| (item.name, item.count)).collect());
    h.tick_all();
    std::mem::take(&mut found.lock().unwrap())
}

fn count_of(items: &[(String, u8)], name: &str) -> u32 {
    items.iter().filter(|(lying, _)| lying == name).map(|(_, count)| u32::from(*count)).sum()
}

#[test]
fn a_dyed_sheep_is_shorn_of_wool_of_its_colour_and_grazing_grows_it_back() {
    let mut h = ready(GameMode::Survival, &[("minecraft:red_dye", 1, 0)], 0.0);
    h.remote.run(|ctx| assert!(ctx.set_block([3, 64, 0], &Block::new("minecraft:grass_block"))));
    let sheep = spawn(&mut h, "minecraft:sheep", [3.5, 65.0, 0.5]);
    let dyed = click_on(&mut h, sheep);
    assert_eq!(set_in(&dyed, sheep, Key::Color), Some(Plain::Byte(14)), "red");
    assert!(held(&mut h, Slot::Main(0)).is_none(), "the dye was used");

    hold(&mut h, "minecraft:shears", 1);
    let shorn = [click_on(&mut h, sheep), watch(&mut h, 2)].concat();
    assert_eq!(has_flag(&shorn, sheep, Flags::SHEARED), Some(true));
    let wool = count_of(&lying(&mut h), "minecraft:red_wool");
    assert!(matches!(wool, 1..=3), "{wool} red wool");
    click_on(&mut h, sheep);
    watch(&mut h, 2);
    assert_eq!(count_of(&lying(&mut h), "minecraft:red_wool"), wool, "a shorn sheep gives no more");

    let grazed = watch(&mut h, 8000);
    assert!(showed(&grazed, sheep, Event::EatGrassAnimation), "it grazed");
    assert_eq!(has_flag(&grazed, sheep, Flags::SHEARED), Some(false), "its wool grew back");
    h.remote.run(|ctx| assert_eq!(ctx.block([3, 64, 0]).map(|block| block.name), Some("minecraft:dirt".into())));
    h.tick_all();
}

#[test]
fn a_bucket_milks_a_cow_a_bowl_takes_a_mooshrooms_stew_and_shears_leave_it_a_cow() {
    let mut h = ready(GameMode::Survival, &[("minecraft:bucket", 1, 0)], 0.0);
    let cow = spawn(&mut h, "minecraft:cow", [2.5, 65.0, 0.5]);
    click_on(&mut h, cow);
    assert_eq!(held(&mut h, Slot::Main(0)).map(|milk| milk.name), Some("minecraft:milk_bucket".into()));

    let mooshroom = spawn(&mut h, "minecraft:mooshroom", [-2.5, 65.0, 0.5]);
    hold(&mut h, "minecraft:bowl", 1);
    click_on(&mut h, mooshroom);
    assert_eq!(held(&mut h, Slot::Main(0)).map(|stew| stew.name), Some("minecraft:mushroom_stew".into()), "one that ate no flower");

    hold(&mut h, "minecraft:shears", 1);
    let shorn = [click_on(&mut h, mooshroom), watch(&mut h, 2)].concat();
    assert!(all::<RemoveEntity>(&shorn).iter().any(|removed| removed.entity_id_self as u64 == mooshroom), "the mooshroom went");
    assert_eq!(added(&shorn, "minecraft:cow").len(), 1, "a cow in its place");
    assert_eq!(count_of(&lying(&mut h), "minecraft:red_mushroom"), 5);
}

#[test]
fn a_hen_lays_an_egg_and_a_brushed_armadillo_drops_a_scute() {
    let mut h = ready(GameMode::Survival, &[("minecraft:brush", 1, 0)], 0.0);
    let armadillo = spawn(&mut h, "minecraft:armadillo", [2.5, 65.0, 0.5]);
    click_on(&mut h, armadillo);
    assert_eq!(count_of(&lying(&mut h), "minecraft:armadillo_scute"), 1);
    (0..4).for_each(|_| drop(click_on(&mut h, armadillo)));
    assert!(held(&mut h, Slot::Main(0)).is_none(), "a brush lasts 64, worn 16 a scute");
    assert_eq!(count_of(&lying(&mut h), "minecraft:armadillo_scute"), 5);

    spawn(&mut h, "minecraft:chicken", [-2.5, 65.0, 0.5]);
    let before = count_of(&lying(&mut h), "minecraft:egg");
    watch(&mut h, 12_001);
    assert_eq!(count_of(&lying(&mut h), "minecraft:egg") - before, 1, "one egg in 5 to 10 minutes");
}

#[test]
fn a_named_name_tag_names_a_mob_which_is_kept_with_its_name() {
    let display = compound([("Name", Value::String("Dolly".into()))]);
    let tag = ItemStack { name: "minecraft:name_tag".into(), count: 1, damage: 0, block: None, tag: Some(compound([("display", display)])) };
    let storage = floor_holding(Vec::new(), &[]);
    storage.store_player(&acacia_auth::login::offline_identity("Steve"), &PlayerData { main: vec![Some(tag)], ..PlayerData::default() }).unwrap();
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    h.remote.run(|ctx| assert!(ctx.spawn_entity("minecraft:sheep", [2.5, 64.0, 0.5]).is_some()));
    let sheep = (0..3).flat_map(|_| all::<AddEntity>(&h.tick(1))).find(|added| added.entity_type == "minecraft:sheep").expect("the sheep").runtime_id;
    h.receive(1, &common::raising::use_on(sheep));
    let named = h.tick(1);
    assert_eq!(set_in(&named, sheep, Key::Nametag), Some(Plain::String("Dolly".into())));

    h.shard.save_worlds();
    let stored = storage.load_column(ChunkPos::new(0, 0), 0).unwrap().expect("the column");
    let kept = sapling_world::actors(&stored.actors).unwrap();
    assert!(kept.iter().any(|actor| actor.identifier == "minecraft:sheep" && actor.name == "Dolly"));
}
