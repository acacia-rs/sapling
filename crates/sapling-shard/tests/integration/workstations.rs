//! The workstations' screens driven by inventory requests as a vanilla client sends them
//! (docs/research/workstations.md): the enchanting table, the anvil, the grindstone, the smithing
//! table, the loom and the beacon.

use crate::common;

use std::sync::Arc;

use acacia_proto::nbt::{List, Value};
use acacia_proto::packets::{AddEntity, AddItemEntity, BlockEntityData, ContainerOpen, InventoryContent, MobEffect, PlayerEnchantOptions};
use acacia_proto::types::{
    Action as BlockAction, ContainerSlotType as Container, ItemV4Extra, ItemStackRequestActionsItemContent as Action, ItemStackRequestActionsItemContentBeaconPayment as Payment,
    ItemStackRequestActionsItemContentCraftGrindstoneRequest as Grind, ItemStackRequestActionsItemContentCraftLoomRequest as Loom, ItemStackRequestActionsItemContentDestroy as Destroy,
    ItemStackRequestActionsItemContentCraftRecipe as Craft, ItemStackRequestActionsItemContentOptional as Optional, ItemStackResponsesItemStatus as Status,
    WindowType,
};
use common::requests::{answer, consume, place, renaming, request, slot};
use common::stored::{compound, enter_stored, floor_holding};
use common::{Harness, all, block_actions, click, config};
use sapling_world::{ItemStack, MemoryStorage, PlayerData, World, WorldStorage};

/// Off the column's edge, so that a beacon's pyramid fits in the column.
const STATION: [i32; 3] = [1, 64, 1];

fn stack(name: &str, count: u8, tag: Option<Value>) -> Option<ItemStack> {
    Some(ItemStack { name: name.into(), count, damage: 0, block: None, tag })
}

/// A tag with these enchantments (by id) and fields.
fn tag(levels: &[(i16, i16)], fields: &[(&str, i32)]) -> Option<Value> {
    let entry = |(id, level): &(i16, i16)| compound([("id", Value::Short(*id)), ("lvl", Value::Short(*level))]);
    let mut all: Vec<(acacia_proto::nbt::Str, Value)> = fields.iter().map(|(key, value)| ((*key).into(), Value::Int(*value))).collect();
    if !levels.is_empty() {
        all.push(("ench".into(), Value::List(List { tag: 10, items: levels.iter().map(entry).collect() })));
    }
    Some(Value::Compound(all))
}

/// Steve, as `data` kept him, beside `block` with `around` it; his screen of it open.
fn at(block: &str, around: &[([i32; 3], &str)], data: PlayerData) -> (Harness, Arc<MemoryStorage>, Vec<acacia_proto::RawPacket>) {
    let mut blocks = vec![(STATION, block)];
    blocks.extend_from_slice(around);
    let storage = floor_holding(Vec::new(), &blocks);
    storage.store_player(&acacia_auth::login::offline_identity("Steve"), &data).unwrap();
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    (0..12).for_each(|_| drop(h.tick(1)));
    h.receive(1, &click(STATION, 1, None));
    let opened = h.tick(1);
    assert_eq!(all::<ContainerOpen>(&opened).len(), 1, "{block} opens");
    (h, storage, opened)
}

/// Steve as storage keeps him, once he left.
fn kept(mut h: Harness, storage: &MemoryStorage) -> PlayerData {
    h.leave(1);
    h.shard.tick();
    h.shard.save_worlds();
    storage.load_player(&acacia_auth::login::offline_identity("Steve")).unwrap().unwrap()
}

fn field<'a>(stack: &'a Option<ItemStack>, key: &str) -> Option<&'a Value> {
    stack.as_ref()?.tag.as_ref()?.get(key)
}

fn put(h: &mut Harness, from: u8, count: u8, into: Container, to: u8) -> Vec<acacia_proto::RawPacket> {
    h.receive(1, &request(-1, vec![place(count, slot(Container::Hotbar, from, 0), slot(into, to, 0))]));
    let sent = h.tick(1);
    assert_eq!(answer(&sent).0, Status::Ok);
    sent
}

fn out(count: u8, to: u8) -> Action {
    place(count, slot(Container::CreativeOutput, 50, 0), slot(Container::HotbarAndInventory, to, 0))
}

#[test]
fn an_enchanting_table_offers_three_and_one_is_taken_for_lapis_and_levels() {
    let data = PlayerData { main: vec![stack("minecraft:iron_sword", 1, None), stack("minecraft:lapis_lazuli", 5, None)], level: 30, enchant_seed: Some(1234), ..PlayerData::default() };
    let shelves = [[3, 64, 1], [3, 65, 1], [3, 64, 2]].map(|pos| (pos, "minecraft:bookshelf"));
    let (mut h, storage, opened) = at("minecraft:enchanting_table", &shelves, data);
    assert_eq!(all::<ContainerOpen>(&opened)[0].window_type, WindowType::Enchantment);
    assert!(all::<PlayerEnchantOptions>(&opened).iter().all(|offered| offered.options.is_empty()), "nothing yet");
    let options = all::<PlayerEnchantOptions>(&put(&mut h, 0, 1, Container::EnchantingInput, 14)).remove(0).options;
    assert_eq!(options.len(), 3, "{options:?}");
    assert!(options.iter().all(|option| !option.held_enchants.is_empty() || !option.self_enchants.is_empty()), "{options:?}");
    put(&mut h, 1, 5, Container::EnchantingLapis, 15);

    // Too few levels for the dearest is refused, if it is dearer than Steve's 30.
    let second = &options[1];
    let take = |id: u32, lapis: u8| request(-3, vec![
        Action::CraftRecipe(Craft { recipe_network_id: id, times_crafted: 1 }),
        consume(1, slot(Container::EnchantingInput, 14, 0)),
        place(1, slot(Container::CreativeOutput, 50, 0), slot(Container::EnchantingInput, 14, 0)),
        consume(lapis, slot(Container::EnchantingLapis, 15, 0)),
    ]);
    h.receive(1, &take(second.option_id, 1));
    assert_eq!(answer(&h.tick(1)).0, Status::Error, "the second takes two lapis");
    h.receive(1, &take(second.option_id, 2));
    let sent = h.tick(1);
    assert_eq!(answer(&sent).0, Status::Ok);
    assert!(all::<PlayerEnchantOptions>(&sent).iter().any(|offered| offered.options.is_empty()), "an enchanted sword is offered nothing");
    h.receive(1, &request(-5, vec![place(1, slot(Container::EnchantingInput, 14, 0), slot(Container::Hotbar, 0, 0))]));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);

    let kept = kept(h, &storage);
    assert_eq!(kept.level, 28, "the second offer takes two levels");
    let Some(Value::List(enchants)) = field(&kept.main[0], "ench") else { panic!("{:?}", kept.main[0]) };
    let ids: Vec<u8> = second.held_enchants.iter().chain(&second.self_enchants).chain(&second.equip_enchants).map(|enchant| enchant.id).collect();
    assert_eq!(enchants.items.len(), ids.len(), "what was offered");
    assert!(kept.enchant_seed.is_some_and(|seed| seed != 1234), "a new seed");
    assert_eq!(kept.main[1].as_ref().map(|lapis| lapis.count), Some(3));
}

#[test]
fn an_anvil_repairs_with_material_renames_and_refuses_what_is_too_expensive() {
    let pickaxe = stack("minecraft:iron_pickaxe", 1, tag(&[], &[("Damage", 200)]));
    let worked = stack("minecraft:iron_sword", 1, tag(&[], &[("Damage", 100), ("RepairCost", 39)]));
    let data = PlayerData { main: vec![pickaxe, stack("minecraft:iron_ingot", 9, None), worked], level: 10, ..PlayerData::default() };
    let (mut h, storage, opened) = at("minecraft:anvil", &[], data);
    assert_eq!(all::<ContainerOpen>(&opened)[0].window_type, WindowType::Anvil);
    put(&mut h, 0, 1, Container::AnvilInput, 1);
    put(&mut h, 1, 9, Container::AnvilMaterial, 2);
    let repair = |used: u8| vec![
        Action::Optional(Optional { recipe_network_id: 0, filtered_string_index: 0 }),
        consume(used, slot(Container::AnvilMaterial, 2, 0)),
        consume(1, slot(Container::AnvilInput, 1, 0)),
        out(1, 0),
    ];
    // 200 worn of 250, a quarter a unit: four ingots, a level each.
    h.receive(1, &request(-3, repair(3)));
    assert_eq!(answer(&h.tick(1)).0, Status::Error, "three ingots are not what it takes");
    h.receive(1, &request(-5, repair(4)));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);

    // A rename alone, of a sword worked 39: capped at 39 levels, more than Steve's 6.
    put(&mut h, 2, 1, Container::AnvilInput, 1);
    let rename = vec![Action::Optional(Optional { recipe_network_id: 0, filtered_string_index: 0 }), consume(1, slot(Container::AnvilInput, 1, 0)), out(1, 2)];
    h.receive(1, &renaming(-7, rename.clone(), "Edge"));
    assert_eq!(answer(&h.tick(1)).0, Status::Error, "39 levels");
    // Repaired too, it is too expensive even with levels to spare.
    h.remote.run(|ctx| assert!(ctx.add_levels(sapling_api::PlayerId(1), 100)));
    h.tick(1);
    h.receive(1, &request(-9, vec![Action::Optional(Optional { recipe_network_id: 0, filtered_string_index: 0 }), consume(2, slot(Container::AnvilMaterial, 2, 0)), consume(1, slot(Container::AnvilInput, 1, 0)), out(1, 2)]));
    assert_eq!(answer(&h.tick(1)).0, Status::Error, "too expensive");
    h.receive(1, &request(-11, vec![place(5, slot(Container::AnvilMaterial, 2, 0), slot(Container::Hotbar, 1, 0))]));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);
    h.receive(1, &renaming(-13, rename, "Edge"));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);

    let kept = kept(h, &storage);
    assert_eq!(kept.level, 10 - 4 + 100 - 39);
    assert_eq!((field(&kept.main[0], "Damage"), field(&kept.main[0], "RepairCost")), (None, Some(&Value::Int(1))));
    let name = kept.main[2].as_ref().and_then(|sword| sword.tag.as_ref()?.get("display")?.get("Name").cloned());
    assert_eq!(name, Some(Value::String("Edge".into())));
    assert_eq!(field(&kept.main[2], "RepairCost"), Some(&Value::Int(39)), "a rename alone adds no work");
}

#[test]
fn a_grindstone_takes_off_all_but_curses_and_gives_experience_back() {
    let sword = stack("minecraft:diamond_sword", 1, tag(&[(9, 3), (28, 1)], &[("RepairCost", 3)]));
    let (mut h, storage, _) = at("minecraft:grindstone", &[], PlayerData { main: vec![sword], ..PlayerData::default() });
    let laid = answer(&put(&mut h, 0, 1, Container::GrindstoneInput, 16)).1;
    let id = laid.iter().find(|(container, slot, ..)| (*container, *slot) == (Container::GrindstoneInput, 16)).map(|(.., id)| *id).expect("laid in");
    let grind = |cost: i32| vec![Action::CraftGrindstoneRequest(Grind { recipe_network_id: id, times_crafted: 1, cost }), consume(1, slot(Container::GrindstoneInput, 16, 0)), out(1, 0)];
    h.receive(1, &request(-3, grind(0)));
    assert_eq!(answer(&h.tick(1)).0, Status::Error, "the curse kept leaves work 1, which the request names");
    h.receive(1, &request(-5, grind(1)));
    let sent = h.tick(1);
    assert_eq!(answer(&sent).0, Status::Ok);
    assert!(all::<AddEntity>(&sent).iter().any(|added| added.entity_type == "minecraft:xp_orb"), "experience back");
    let kept = kept(h, &storage);
    let Some(Value::List(enchants)) = field(&kept.main[0], "ench") else { panic!("{:?}", kept.main[0]) };
    assert_eq!(enchants.items.len(), 1, "the curse stays");
    assert_eq!(field(&kept.main[0], "RepairCost"), Some(&Value::Int(1)));
}

#[test]
fn a_smithing_table_upgrades_to_netherite_keeping_the_sword() {
    let sword = stack("minecraft:diamond_sword", 1, tag(&[(9, 2)], &[("Damage", 40)]));
    let data = PlayerData { main: vec![stack("minecraft:netherite_upgrade_smithing_template", 1, None), sword, stack("minecraft:netherite_ingot", 1, None)], ..PlayerData::default() };
    let (mut h, storage, _) = at("minecraft:smithing_table", &[], data);
    put(&mut h, 0, 1, Container::SmithingTableTemplate, 53);
    put(&mut h, 1, 1, Container::SmithingTableInput, 51);
    put(&mut h, 2, 1, Container::SmithingTableMaterial, 52);
    let plain = |name| sapling_data::Stack { name, count: 1, damage: 0, block: None, tag: None };
    let upgrade = (1..20_000).find(|id| sapling_data::smithed(*id, &plain("minecraft:netherite_upgrade_smithing_template"), &plain("minecraft:diamond_sword"), &plain("minecraft:netherite_ingot")).is_some()).expect("the recipe");
    h.receive(1, &request(-3, vec![
        Action::CraftRecipe(Craft { recipe_network_id: upgrade, times_crafted: 1 }),
        consume(1, slot(Container::SmithingTableTemplate, 53, 0)),
        consume(1, slot(Container::SmithingTableInput, 51, 0)),
        consume(1, slot(Container::SmithingTableMaterial, 52, 0)),
        out(1, 1),
    ]));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);
    let kept = kept(h, &storage);
    assert_eq!(kept.main[1].as_ref().map(|sword| sword.name.as_str()), Some("minecraft:netherite_sword"));
    assert_eq!(field(&kept.main[1], "Damage"), Some(&Value::Int(40)), "the sword's wear and enchantments stay");
    assert!(kept.main[0].is_none() && kept.main[2].is_none());
}

#[test]
fn an_anvil_answers_status_9_to_a_rename_to_the_items_own_name() {
    let (mut h, _storage, _) = at("minecraft:anvil", &[], PlayerData { main: vec![stack("minecraft:diamond_sword", 1, None)], ..PlayerData::default() });
    put(&mut h, 0, 1, Container::AnvilInput, 1);
    let rename = vec![Action::Optional(Optional { recipe_network_id: 0, filtered_string_index: 0 }), consume(1, slot(Container::AnvilInput, 1, 0)), out(1, 2)];
    h.receive(1, &renaming(-3, rename, "Diamond Sword"));
    let sent = h.tick(1);
    assert_eq!(answer(&sent).0, Status::Unknown(9));
    assert!(all::<InventoryContent>(&sent).is_empty(), "nothing resent");
}

#[test]
fn a_loom_adds_the_dyes_pattern_after_the_banners_own() {
    let banner = Some(ItemStack { damage: 15, ..stack("minecraft:banner", 1, Some(compound([("Type", Value::Int(0))]))).unwrap() });
    let (mut h, storage, opened) = at("minecraft:loom", &[], PlayerData { main: vec![banner, stack("minecraft:red_dye", 2, None)], ..PlayerData::default() });
    assert_eq!(all::<ContainerOpen>(&opened)[0].window_type, WindowType::Loom);
    put(&mut h, 0, 1, Container::LoomInput, 9);
    put(&mut h, 1, 1, Container::LoomDye, 10);
    let loom = |pattern: &str| vec![Action::CraftLoomRequest(Loom { pattern: pattern.into(), times_crafted: 1 }), consume(1, slot(Container::LoomInput, 9, 0)), consume(1, slot(Container::LoomDye, 10, 0)), out(1, 0)];
    h.receive(1, &request(-3, loom("cre")));
    assert_eq!(answer(&h.tick(1)).0, Status::Error, "a creeper needs its pattern item");
    h.receive(1, &request(-5, loom("bo")));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);
    let kept = kept(h, &storage);
    let Some(Value::List(patterns)) = field(&kept.main[0], "Patterns") else { panic!("{:?}", kept.main[0]) };
    assert_eq!(patterns.items, [compound([("Color", Value::Int(1)), ("Pattern", Value::String("bo".into()))])]);
    assert_eq!(kept.main[0].as_ref().map(|banner| banner.damage), Some(15));
    assert_eq!(kept.main[1].as_ref().map(|dye| dye.count), Some(1));
}

#[test]
fn a_loom_refuses_a_seventh_pattern_with_status_7_and_resends_nothing() {
    let layer = compound([("Color", Value::Int(1)), ("Pattern", Value::String("bo".into()))]);
    let full = compound([("Patterns", Value::List(List { tag: 10, items: vec![layer; 6] })), ("Type", Value::Int(0))]);
    let banner = Some(ItemStack { damage: 15, ..stack("minecraft:banner", 1, Some(full)).unwrap() });
    let (mut h, _storage, _) = at("minecraft:loom", &[], PlayerData { main: vec![banner, stack("minecraft:red_dye", 1, None)], ..PlayerData::default() });
    put(&mut h, 0, 1, Container::LoomInput, 9);
    put(&mut h, 1, 1, Container::LoomDye, 10);
    let loom = vec![Action::CraftLoomRequest(Loom { pattern: "bo".into(), times_crafted: 1 }), consume(1, slot(Container::LoomInput, 9, 0)), consume(1, slot(Container::LoomDye, 10, 0)), out(1, 0)];
    h.receive(1, &request(-3, loom));
    let sent = h.tick(1);
    assert_eq!(answer(&sent).0, Status::Unknown(7));
    assert!(all::<InventoryContent>(&sent).is_empty(), "nothing resent");
}

#[test]
fn a_patterned_banner_keeps_its_patterns_placed_and_broken() {
    let bordered = compound([("Patterns", Value::List(List { tag: 10, items: vec![compound([("Color", Value::Int(1)), ("Pattern", Value::String("bo".into()))])] })), ("Type", Value::Int(0))]);
    let banner = Some(ItemStack { damage: 15, ..stack("minecraft:banner", 1, Some(bordered)).unwrap() });
    let (mut h, _storage, _) = at("minecraft:loom", &[], PlayerData { main: vec![banner], ..PlayerData::default() });
    const SPOT: [i32; 3] = [3, 64, 0];
    h.receive(1, &click([3, 63, 0], 1, None));
    let shown = all::<BlockEntityData>(&h.tick(1)).into_iter().map(|data| data.nbt.decode().unwrap().value).find(|entity| entity.get("id") == Some(&Value::String("Banner".into())));
    let shown = shown.expect("the banner's entity");
    assert!(matches!(shown.get("Patterns"), Some(Value::List(list)) if list.items.len() == 1), "{shown:?}");
    h.receive(1, &block_actions(&[(BlockAction::StartBreak, SPOT)]));
    (0..40).for_each(|_| drop(h.tick(1)));
    h.receive(1, &block_actions(&[(BlockAction::PredictBreak, SPOT)]));
    let left = all::<AddItemEntity>(&h.tick(1));
    assert_eq!(left.len(), 1);
    let ItemV4Extra::Default(Some(extra)) = &left[0].item.extra else { panic!("{:?}", left[0].item) };
    assert!(format!("{:?}", extra.nbt).contains("\"bo\""), "{:?}", extra.nbt);
}

/// A paid beacon on a one-layer pyramid, with `above` over it; what it gives in 80 ticks.
fn beacon_gives(above: &[([i32; 3], &str)]) -> Vec<MobEffect> {
    let mut base: Vec<([i32; 3], &str)> = (0..3).flat_map(|x| (0..3).map(move |z| ([x, 63, z], "minecraft:iron_block"))).collect();
    base.extend_from_slice(above);
    let (mut h, _storage, opened) = at("minecraft:beacon", &base, PlayerData { main: vec![stack("minecraft:iron_ingot", 2, None)], ..PlayerData::default() });
    assert_eq!(all::<ContainerOpen>(&opened)[0].window_type, WindowType::Beacon);
    let pay = |primary: i32| vec![Action::BeaconPayment(Payment { primary_effect: primary, secondary_effect: 0 }), Action::Destroy(Destroy { count: 1, source: slot(Container::BeaconPayment, 27, 0) })];
    put(&mut h, 0, 1, Container::BeaconPayment, 27);
    h.receive(1, &request(-3, pay(5)));
    assert_eq!(answer(&h.tick(1)).0, Status::Error, "strength needs three layers");
    h.receive(1, &request(-5, pay(1)));
    assert_eq!(answer(&h.tick(1)).0, Status::Ok);
    (0..80).flat_map(|_| all::<MobEffect>(&h.tick(1))).collect()
}

#[test]
fn a_beacon_takes_its_payment_for_what_its_pyramid_offers_and_gives_it_around() {
    let given = beacon_gives(&[([1, 70, 1], "minecraft:glass"), ([1, 75, 1], "minecraft:bedrock")]);
    assert_eq!(given.iter().map(|effect| (effect.effect_id, effect.amplifier, effect.duration, effect.ambient)).collect::<Vec<_>>(), [(1, 0, 220, true)]);
}

#[test]
fn a_beacon_under_stone_gives_nothing() {
    assert!(beacon_gives(&[([1, 70, 1], "minecraft:stone")]).is_empty());
}
