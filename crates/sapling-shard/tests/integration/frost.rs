//! Powder snow's freeze on a player, against research/captures/bds-freeze.txt.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{EntityEvent, EntityEventEventId, SetEntityData, UpdateAttributes};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all};
use sapling_api::{Block, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);
const SNOW: [[i32; 3]; 2] = [[3, 65, 3], [3, 66, 3]];
/// The `Flags` bit of a full freeze.
const FROZEN: i64 = 1 << 40;

fn snowed_in() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| {
        assert!(SNOW.iter().all(|at| ctx.set_block(*at, &Block::new("powder_snow"))));
        ctx.teleport(ALEX, [3.5, 65.0, 3.5]);
    });
    // The teleport lands within a tick or two; the first tick in the snow tells 1/140.
    let first: Vec<f32> = (0..3).map(|_| strengths(h.tick_all().to(1))).find(|told| !told.is_empty()).expect("the freeze began");
    assert!((first[0] - 1.0 / 140.0).abs() < 1e-6, "{first:?}");
    h
}

fn strengths(sent: &[RawPacket]) -> Vec<f32> {
    let data = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == 1).flat_map(|data| data.metadata);
    data.filter(|item| item.key == Key::FreezingEffectStrength).filter_map(|item| if let Value::Default(Plain::Float(strength)) = item.value { Some(strength) } else { None }).collect()
}

fn frozen_flags(sent: &[RawPacket]) -> Vec<bool> {
    let data = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == 1).flat_map(|data| data.metadata);
    data.filter_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0 & FROZEN != 0) } else { None }).collect()
}

fn told<T>(h: &mut Harness, ticks: usize, to: u64, read: fn(&[RawPacket]) -> Vec<T>) -> Vec<T> {
    (0..ticks).flat_map(|_| read(h.tick_all().to(to))).collect()
}

fn health(sent: &[RawPacket]) -> Vec<f32> {
    all::<UpdateAttributes>(sent).iter().flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:health").map(|attribute| attribute.current).collect()
}

fn walking(sent: &[RawPacket]) -> Vec<f32> {
    all::<UpdateAttributes>(sent).iter().flat_map(|update| &update.attributes).filter(|attribute| attribute.name == "minecraft:movement").map(|attribute| attribute.current).collect()
}

#[test]
fn powder_snow_freezes_a_player_in_seven_seconds_then_hurts_it_every_two() {
    let mut h = snowed_in();
    let climb = told(&mut h, 138, 1, strengths);
    assert_eq!(climb.len(), 138, "the strength is told each tick it climbs");
    assert!((climb[0] - 2.0 / 140.0).abs() < 1e-6 && climb[137] < 1.0, "{:?}", &climb[..3]);
    let sent = h.tick_all();
    let sent = sent.to(1);
    assert!(strengths(sent).len() == 1 && !frozen_flags(sent).contains(&true), "no frozen flag before it is full");
    let full = h.tick_all();
    assert_eq!((strengths(full.to(1)), frozen_flags(full.to(1)), health(full.to(1))), (vec![1.0], vec![true], vec![19.0]), "full: flagged and hurt at once");
    assert_eq!(strengths(full.to(2)), [1.0], "viewers see the strength too");
    let hurt: Vec<_> = all::<EntityEvent>(full.to(2)).into_iter().map(|event| (event.event_id, event.data)).collect();
    assert!(hurt.contains(&(EntityEventEventId::HurtAnimation, 27)), "{hurt:?}");
    assert_eq!(told(&mut h, 1, 1, walking), [0.05], "the walk slows a tick behind the strength");
    assert_eq!(told(&mut h, 39, 1, health), [18.0], "then 1 every 40 ticks");
}

#[test]
fn out_of_powder_snow_or_in_any_leather_the_freeze_thaws_twice_as_fast() {
    let mut h = snowed_in();
    told(&mut h, 13, 1, strengths);
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Armor(0), Some(&Item::new("minecraft:leather_helmet", 1)))));
    let thaw = told(&mut h, 12, 1, strengths);
    let steps: Vec<f32> = thaw.windows(2).map(|pair| pair[0] - pair[1]).collect();
    assert!(thaw.last() == Some(&0.0) && steps[..steps.len() - 1].iter().all(|step| (step - 2.0 / 140.0).abs() < 1e-5), "{thaw:?}");
    assert!(told(&mut h, 20, 1, strengths).is_empty(), "nothing told while it stays thawed");
}
