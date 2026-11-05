//! A horse-family mob grazing, bit 32 of its data 16 (research/captures/bds-horseeat.txt;
//! BDS's `Horse::postAiStep`, docs/research/leftovers.md).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::asking::ask;
use common::{Harness, all};
use sapling_api::{Block, EntityId};

/// The ticks at which data 16 of `id` was told, with its value.
fn told(each: &[Vec<RawPacket>], id: EntityId) -> Vec<(usize, i64)> {
    let values = |sent: &[RawPacket]| -> Vec<i64> {
        let items = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id.0).flat_map(|data| data.metadata);
        items.filter(|item| item.key == Key::MinecartDisplayBlock).filter_map(|item| if let Value::Default(Plain::Long(value)) = item.value { Some(value) } else { None }).collect()
    };
    each.iter().enumerate().flat_map(|(tick, sent)| values(sent).into_iter().map(move |value| (tick, value))).collect()
}

#[test]
fn a_horse_on_grass_grazes_for_50_ticks_now_and_then_and_one_on_stone_never() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| (1..=3).for_each(|x| (1..=3).for_each(|z| assert!(ctx.set_block([x, 64, z], &Block::new("grass_block"))))));
    let on_grass = ask(&mut h, |ctx| ctx.summon_as(None, "minecraft:horse", [2.5, 65.0, 2.5], 0.0, 0.0, None)).expect("a horse");
    let on_stone = ask(&mut h, |ctx| ctx.summon_as(None, "minecraft:donkey", [-3.5, 65.0, -3.5], 0.0, 0.0, None)).expect("a donkey");
    // Past their first data, which holds data 16 too.
    (0..2).for_each(|_| drop(h.tick_all()));
    // One tick in 300: none in 3000 ticks is a chance of e^-10.
    let each: Vec<Vec<RawPacket>> = (0..3000).map(|_| h.tick_all().to(1).to_vec()).collect();
    let mut grazed = told(&each, on_grass);
    // A meal begun in the last 50 ticks ends past the watch.
    if grazed.len() % 2 == 1 && grazed.last().is_some_and(|&(tick, value)| value == 32 && tick + 50 >= each.len()) {
        grazed.pop();
    }
    assert!(!grazed.is_empty() && grazed.len() % 2 == 0, "{grazed:?}");
    for meal in grazed.chunks(2) {
        assert_eq!((meal[0].1, meal[1].1, meal[1].0 - meal[0].0), (32, 0, 50), "{grazed:?}");
    }
    assert_eq!(told(&each, on_stone), [], "stone is not grass");
}

#[test]
fn a_hurt_horse_heals_one_now_and_then_and_a_cow_never() {
    use acacia_proto::packets::UpdateAttributes;
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let horse = ask(&mut h, |ctx| ctx.summon_as(None, "minecraft:horse", [2.5, 65.0, 2.5], 0.0, 0.0, None)).expect("a horse");
    let cow = ask(&mut h, |ctx| ctx.summon_as(None, "minecraft:cow", [-3.5, 65.0, -3.5], 0.0, 0.0, None)).expect("a cow");
    (0..2).for_each(|_| drop(h.tick_all()));
    h.remote.run(move |ctx| assert!(ctx.hurt_entity(horse, 6.0, sapling_api::DamageCause::Generic) && ctx.hurt_entity(cow, 6.0, sapling_api::DamageCause::Generic)));
    // One tick in 900 (research/captures/bds-e1heal.txt): none in 4000 ticks is a chance of e^-4.4.
    let sent: Vec<RawPacket> = (0..4000).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let healths = |id: EntityId| -> Vec<f32> { all::<UpdateAttributes>(&sent).into_iter().filter(|told| told.runtime_entity_id == id.0).flat_map(|told| told.attributes).filter(|item| item.name == "minecraft:health").map(|item| item.current).collect() };
    let horse = healths(horse);
    assert!(horse.len() > 1 && horse.windows(2).all(|pair| pair[1] == pair[0] + 1.0), "{horse:?}");
    assert_eq!(healths(cow).len(), 1, "the cow's hurt alone");
}
