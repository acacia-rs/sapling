//! Special recipes crafted in a grid by inventory requests as a vanilla client sends them
//! (inventory/special.rs; docs/research/crafting-storage.md): a request names the recipe by its
//! network id and only that recipe's rule makes the result.

use crate::common;

use std::sync::Arc;

use acacia_proto::RawPacket;
use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::ClientboundMapItemData;
use acacia_proto::types::{
    ContainerSlotType as Container, ItemStackRequestActionsItemContent as Action, ItemStackRequestActionsItemContentCraftRecipe as Craft,
    ItemStackRequestActionsItemContentCreate as Create, ItemStackResponsesItemStatus as Status,
};
use common::requests::{answer, consume, place, request, slot};
use common::stored::{compound, enter_stored, floor_holding};
use common::{Harness, click, config};
use sapling_world::{ItemStack, MemoryStorage, PlayerData, World, WorldStorage};

const TABLE: [i32; 3] = [1, 64, 1];
const SHIELD_BANNER: &str = "00000000-0000-0000-0000-0000000000c8";
const FIREWORKS: &str = "00000000-0000-0000-0000-000000000002";
const POT: &str = "685a742a-c42e-4a4e-88ea-5eb83fc98e5b";
const BOOK_CLONING: &str = "d1ca6b84-338e-4f2f-9c6b-76cc8b4bd98d";
const EXTENDING: &str = "d392b075-4ba1-40ae-8789-af868d56f6ce";
const REPAIR: &str = "00000000-0000-0000-0000-000000000001";

fn stack(name: &str, count: u8, damage: i16, tag: Option<Value>) -> Option<ItemStack> {
    Some(ItemStack { name: name.into(), count, damage, block: None, tag })
}

/// Steve holding `main`, next to a crafting table, its screen open if `at_table`.
fn steve(main: Vec<Option<ItemStack>>, at_table: bool) -> (Harness, Arc<MemoryStorage>) {
    let storage = floor_holding(Vec::new(), &[(TABLE, "minecraft:crafting_table")]);
    storage.store_player(&acacia_auth::login::offline_identity("Steve"), &PlayerData { main, ..PlayerData::default() }).unwrap();
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    (0..12).for_each(|_| drop(h.tick(1)));
    if at_table {
        h.receive(1, &click(TABLE, 1, None));
        h.tick(1);
    }
    (h, storage)
}

fn kept(mut h: Harness, storage: &MemoryStorage) -> PlayerData {
    h.leave(1);
    h.shard.tick();
    h.shard.save_worlds();
    storage.load_player(&acacia_auth::login::offline_identity("Steve")).unwrap().unwrap()
}

/// Lays hotbar slot `from` into grid slot `cell`, one item.
fn lay(h: &mut Harness, from: u8, cell: u8) {
    h.receive(1, &request(-1, vec![place(1, slot(Container::Hotbar, from, 0), slot(Container::CraftingInput, cell, 0))]));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);
}

/// Crafts the special recipe `uuid` from grid `cells`, taking each of `results` (index, count)
/// into hotbar `to` and on.
fn craft(h: &mut Harness, uuid: &str, cells: &[u8], results: &[(u8, u8)], to: u8) -> Vec<RawPacket> {
    let recipe_network_id = sapling_data::multi_recipe(uuid).expect("the recipe");
    let mut actions = vec![Action::CraftRecipe(Craft { recipe_network_id, times_crafted: 1 })];
    actions.extend(cells.iter().map(|cell| consume(1, slot(Container::CraftingInput, *cell, 0))));
    for (i, (index, count)) in results.iter().enumerate() {
        if results.len() > 1 {
            actions.push(Action::Create(Create { result_slot_id: *index }));
        }
        actions.push(place(*count, slot(Container::CreativeOutput, 50, 0), slot(Container::HotbarAndInventory, to + i as u8, 0)));
    }
    h.receive(1, &request(-9, actions));
    h.tick(1)
}

fn patterned() -> Option<Value> {
    let layer = compound([("Color", Value::Int(1)), ("Pattern", Value::String("bo".into()))]);
    Some(compound([("Patterns", Value::List(List { tag: 10, items: vec![layer] })), ("Type", Value::Int(0))]))
}

#[test]
fn a_banner_goes_on_a_shield_by_its_own_recipe_only() {
    let (mut h, storage) = steve(vec![stack("minecraft:shield", 1, 0, None), stack("minecraft:banner", 1, 4, patterned())], false);
    lay(&mut h, 0, 28);
    lay(&mut h, 1, 29);
    assert_eq!(answer(&craft(&mut h, FIREWORKS, &[28, 29], &[(0, 1)], 2)).0, Status::Error, "the fireworks recipe makes nothing of these");
    assert_eq!(answer(&craft(&mut h, SHIELD_BANNER, &[28, 29], &[(0, 1)], 2)).0, Status::Ok);
    let kept = kept(h, &storage);
    let tag = kept.main[2].as_ref().and_then(|shield| shield.tag.clone()).expect("the decorated shield");
    assert_eq!((tag.get("Base"), tag.get("Patterns")), (Some(&Value::Int(4)), patterned().as_ref().and_then(|tag| tag.get("Patterns"))));
}

#[test]
fn a_table_makes_a_pot_from_sherds() {
    let (sherd, brick) = ("minecraft:angler_pottery_sherd", "minecraft:brick");
    let (mut h, storage) = steve(vec![stack(sherd, 1, 0, None), stack(brick, 3, 0, None)], true);
    lay(&mut h, 0, 33);
    for cell in [35, 37, 39] {
        lay(&mut h, 1, cell);
    }
    assert_eq!(answer(&craft(&mut h, POT, &[33, 35, 37, 39], &[(0, 1)], 2)).0, Status::Ok);
    let kept = kept(h, &storage);
    let Some(Value::List(faces)) = kept.main[2].as_ref().and_then(|pot| pot.tag.as_ref()?.get("sherds").cloned()) else { panic!("{:?}", kept.main[2]) };
    let names: Vec<Value> = [sherd, brick, brick, brick].iter().map(|name| Value::String((*name).into())).collect();
    assert_eq!(faces.items, names);
}

#[test]
fn a_written_book_is_copied_and_given_back() {
    let book = compound([("author", Value::String("Steve".into())), ("generation", Value::Int(0)), ("title", Value::String("Notes".into()))]);
    let (mut h, storage) = steve(vec![stack("minecraft:written_book", 1, 0, Some(book.clone())), stack("minecraft:writable_book", 1, 0, None)], false);
    lay(&mut h, 0, 28);
    lay(&mut h, 1, 29);
    assert_eq!(answer(&craft(&mut h, BOOK_CLONING, &[28, 29], &[(0, 1), (1, 1)], 2)).0, Status::Ok);
    let kept = kept(h, &storage);
    let generation = |slot: usize| kept.main[slot].as_ref().and_then(|book| book.tag.as_ref()?.get("generation").cloned());
    assert_eq!((generation(2), generation(3)), (Some(Value::Int(1)), Some(Value::Int(0))));
}

#[test]
fn eight_paper_round_a_map_extend_it_at_craft_time() {
    let (mut h, storage) = steve(vec![stack("minecraft:empty_map", 1, 0, None), stack("minecraft:paper", 8, 0, None)], false);
    h.receive(1, &common::shooting::click_air());
    let map = common::all::<ClientboundMapItemData>(&h.tick(1)).pop().expect("the map made");
    h.receive(1, &click(TABLE, 1, None));
    h.tick(1);
    let cells: Vec<u8> = (32..41).collect();
    for cell in &cells {
        lay(&mut h, if *cell == 36 { 0 } else { 1 }, *cell);
    }
    let sent = craft(&mut h, EXTENDING, &cells, &[(0, 1)], 2);
    assert_eq!(answer(&sent).0, Status::Ok);
    let zoom = common::all::<ClientboundMapItemData>(&sent).into_iter().find(|packet| packet.map_id == map.map_id + 1).expect("the extended map, whole");
    assert_eq!(zoom.scale, Some(1));
    let kept = kept(h, &storage);
    let tag = kept.main[2].as_ref().and_then(|map| map.tag.clone()).expect("the extended map");
    assert_eq!((tag.get("map_uuid"), tag.get("map_is_init")), (Some(&Value::Long(map.map_id + 1)), Some(&Value::Byte(1))));
}

#[test]
fn two_worn_tools_repair_into_one() {
    let worn = |damage: i32| Some(compound([("Damage", Value::Int(damage))]));
    let (mut h, storage) = steve(vec![stack("minecraft:iron_pickaxe", 1, 0, worn(200)), stack("minecraft:iron_pickaxe", 1, 0, worn(150))], false);
    lay(&mut h, 0, 28);
    lay(&mut h, 1, 31);
    assert_eq!(answer(&craft(&mut h, REPAIR, &[28, 31], &[(0, 1)], 2)).0, Status::Ok);
    let kept = kept(h, &storage);
    // An iron pickaxe lasts 250: 50 + 100 left and 12 more leave 88 worn.
    let pickaxe = kept.main[2].as_ref().expect("the repaired pickaxe");
    assert_eq!((pickaxe.name.as_str(), pickaxe.tag.as_ref().and_then(|tag| tag.get("Damage"))), ("minecraft:iron_pickaxe", Some(&Value::Int(88))));
    assert!(kept.main[0].is_none() && kept.main[1].is_none());
}
