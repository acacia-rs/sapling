//! Group AN's villager rules (docs/research/villagers.md): fear's look box and spot search, a baby's
//! look, sharing food, and the cats a village brings.

use crate::common;
use crate::villager_days::village;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{AddEntity, AddItemEntity, SetEntityData, UpdateTrade};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1};
use common::raising::{feet_of, has_flag, plain_in, set_in};
use common::all;
use sapling_api::{Effect, EntityId, Item, MobSlot};

const UNSKILLED: &str = "minecraft:become_unskilled";

/// How much farther from a still zombie put `off` west of it a villager is 20 ticks on; the spot
/// it flees to may lie aside.
fn fled_from(off: f32) -> f32 {
    let feet = [20.5, 65.0, 0.5];
    let (mut h, id) = village(2_000, feet, UNSKILLED);
    let at = [feet[0] - off, feet[1], feet[2]];
    common::asking::ask(&mut h, move |ctx| {
        let zombie = ctx.spawn_entity("minecraft:zombie", at).expect("zombie");
        assert!(ctx.give_entity_effect(zombie, Effect::Slowness, 2000, 255));
    });
    (0..20).for_each(|_| drop(h.tick_all()));
    let now = feet_of(&mut h, id);
    (now[0] - at[0]).hypot(now[2] - at[2]) - off
}

/// What viewer 1 is told in the 3 ticks after the server summons `kind` at `feet` by `event`.
fn summoned(h: &mut common::Harness, kind: &'static str, feet: [f32; 3], event: Option<&'static str>) -> Vec<acacia_proto::RawPacket> {
    h.remote.run(move |ctx| assert!(ctx.summon_as(None, kind, feet, 0.0, 0.0, event).is_some()));
    (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

#[test]
fn a_villager_flees_a_still_zombie_3_off_but_not_one_5_off() {
    // research/captures/bds-an1fear.txt: 6 of 8 villagers 3 off moved within 1.5 s, none 4 off within 2.8 s.
    let near = fled_from(3.0);
    assert!(near > 0.5, "it fled: {near}");
    let far = fled_from(5.0);
    assert!(far.abs() < 0.3, "it stayed: {far}");
}

#[test]
fn a_baby_villager_is_announced_at_half_scale_with_its_own_box_and_stays_a_baby() {
    // research/captures/bds-g11share.txt: scale 0.5, box 0.98 by 1.96, BABY on the tick after too.
    let (mut h, _) = village(2_000, [0.5, 65.0, 0.5], UNSKILLED);
    let sent = summoned(&mut h, "minecraft:villager_v2", [3.5, 65.0, 0.5], Some("minecraft:entity_born"));
    let added = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == "minecraft:villager_v2").expect("announced");
    // Sapling's AddEntity carries the grown look and a SetEntityData that tick the baby's.
    let look = [Key::Scale, Key::BoundingboxWidth, Key::BoundingboxHeight].map(|key| set_in(&sent, added.runtime_id, key).or_else(|| plain_in(&added.metadata, key)));
    assert_eq!(look, [Some(Plain::Float(0.5)), Some(Plain::Float(0.98)), Some(Plain::Float(1.96))]);
    assert_eq!(has_flag(&sent, added.runtime_id, MetadataFlags1::BABY), Some(true));
}

#[test]
fn a_villager_with_bread_to_spare_throws_three_to_one_with_none() {
    // research/captures/bds-g11share.txt: 24 bread given to one, 3 thrown to its mate.
    let (mut h, first) = village(2_000, [-4.5, 65.0, 4.5], UNSKILLED);
    common::asking::ask(&mut h, |ctx| ctx.summon_as(None, "minecraft:villager_v2", [-3.0, 65.0, 4.5], 0.0, 0.0, Some(UNSKILLED)).expect("summoned"));
    let given = common::asking::ask(&mut h, move |ctx| ctx.set_mob_item(EntityId(first), MobSlot::Inventory(0), Some(&Item::new("minecraft:bread", 24))));
    assert_eq!(given, Ok(()));
    let thrown = (0..400).flat_map(|_| all::<AddItemEntity>(&h.tick_all().to(1).to_vec())).map(|added| added.item.count).next();
    assert_eq!(thrown, Some(3));
}

#[test]
fn a_cartographer_offers_explorer_maps_each_with_its_own_map() {
    // research/captures/bds-an1maps.txt: tier 1 a taiga (8) and a savanna (10) village map, tier 2
    // a monument (3) and trial chambers (14) map, tier 4 a mansion (4) map, ids 5 apart.
    let (mut h, id) = village(2_000, [4.5, 65.0, 4.5], "minecraft:become_cartographer");
    let screen = all::<UpdateTrade>(&common::raising::click_on(&mut h, id)).into_iter().next().expect("its screen");
    let offers = screen.offers.decode().unwrap().value;
    let Some(Value::List(recipes)) = offers.get("Recipes") else { panic!("no recipes") };
    let maps: Vec<(i16, i64)> = recipes
        .items
        .iter()
        .filter_map(|recipe| recipe.get("sell"))
        .filter(|sell| sell.get("Name") == Some(&Value::String("minecraft:filled_map".into())))
        .map(|sell| {
            let Some(Value::Short(damage)) = sell.get("Damage") else { panic!("damage") };
            let Some(Value::Long(map)) = sell.get("tag").and_then(|tag| tag.get("map_uuid")) else { panic!("map_uuid") };
            (*damage, *map)
        })
        .collect();
    assert!(maps.iter().any(|(damage, _)| [3, 14].contains(damage)) && maps.iter().any(|(damage, _)| *damage == 4), "{maps:?}");
    assert!(maps.windows(2).all(|pair| pair[1].1 - pair[0].1 == 5), "{maps:?}");
}

#[test]
fn a_wandering_traders_llamas_come_on_its_leads() {
    // research/captures/bds-trade2.txt: both llamas' LeadHolderEid is the trader's.
    let (mut h, _) = village(2_000, [0.5, 65.0, 0.5], UNSKILLED);
    let sent = summoned(&mut h, "minecraft:wandering_trader", [3.5, 65.0, 3.5], None);
    let added = all::<AddEntity>(&sent);
    let trader = added.iter().find(|added| added.entity_type == "minecraft:wandering_trader").expect("the trader").runtime_id;
    let llamas: Vec<u64> = added.iter().filter(|added| added.entity_type == "minecraft:trader_llama").map(|added| added.runtime_id).collect();
    assert_eq!(llamas.len(), 2);
    let data = all::<SetEntityData>(&sent);
    for llama in llamas {
        let announced = added.iter().filter(|added| added.runtime_id == llama).map(|added| &added.metadata);
        let set = data.iter().filter(|data| data.runtime_entity_id == llama).map(|data| &data.metadata);
        let holder = announced.chain(set).filter_map(|metadata| plain_in(metadata, Key::LeadHolderEid)).last();
        assert_eq!(holder, Some(Plain::Long(trader as i64)), "llama {llama}");
        assert_eq!(has_flag(&sent, llama, MetadataFlags1::LEASHED), Some(true));
    }
}

#[test]
fn a_village_of_four_brings_a_cat() {
    // Village::tick 0x142a23bc0: fewer cats than a quarter of the villagers brings one.
    let (mut h, _) = village(2_000, [0.5, 65.0, 0.5], UNSKILLED);
    common::asking::ask(&mut h, |ctx| {
        for x in 0..3 {
            assert!(ctx.summon_as(None, "minecraft:villager_v2", [2.5 + x as f32, 65.0, 0.5], 0.0, 0.0, Some(UNSKILLED)).is_some());
        }
        assert!(ctx.set_game_rule("doMobSpawning", sapling_api::RuleValue::Bool(true)));
    });
    let cats = (0..2500).flat_map(|_| all::<AddEntity>(&h.tick_all().to(1).to_vec())).filter(|added| added.entity_type == "minecraft:cat").count();
    assert_eq!(cats, 1, "one cat for four villagers in two looks");
}
