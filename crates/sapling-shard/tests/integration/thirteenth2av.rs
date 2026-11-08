//! Group AV: zombie villagers summoned plain or as babies (research/captures/bds-ar2look.txt), a cat's clicks.

use crate::common;

use acacia_proto::packets::{AddEntity, EntityEventEventId as Event};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Meta, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags};
use common::raising::{click_on, has_flag, showed};
use common::shooting::{ready, spawn};
use common::stored::{enter_stored, floor_holding};
use common::{Harness, all, config};
use sapling_api::{Difficulty, GameMode};
use sapling_world::World;

const ZOMBIE_VILLAGER: &str = "minecraft:zombie_villager_v2";

fn harness() -> Harness {
    let mut h = Harness::with(config(), World::with_storage(floor_holding(Vec::new(), &[])));
    enter_stored(&mut h, 1);
    h
}

fn value(added: &AddEntity, key: Key) -> Option<Meta> {
    added.metadata.iter().find(|item| item.key == key).map(|item| item.value.clone())
}

fn int(added: &AddEntity, key: Key) -> i32 {
    match value(added, key) {
        Some(Meta::Default(Plain::Int(value))) => value,
        other => panic!("{key:?}: {other:?}"),
    }
}

fn float(added: &AddEntity, key: Key) -> f32 {
    match value(added, key) {
        Some(Meta::Default(Plain::Float(value))) => value,
        other => panic!("{key:?}: {other:?}"),
    }
}

/// Summons `count` zombie villagers by `event` a block apart; their announcements.
fn summoned(event: Option<&'static str>, count: usize) -> Vec<AddEntity> {
    let mut h = harness();
    let mut shown = Vec::new();
    for n in 0..count {
        let at = [2.5 + n as f32, 64.0, 2.5];
        h.remote.run(move |ctx| {
            ctx.set_difficulty(Difficulty::Easy);
            assert!(ctx.summon_as(None, ZOMBIE_VILLAGER, at, 0.0, 0.0, event).is_some());
        });
        let sent: Vec<_> = (0..2).flat_map(|_| h.tick(1)).collect();
        shown.extend(all::<AddEntity>(&sent).into_iter().filter(|added| added.entity_type == ZOMBIE_VILLAGER));
    }
    shown
}

/// BDS's plain-summoned zombie villager came an armorer (`Variant` 8) with a rolled skin and no tiers: its
/// `entity_spawned` rolls one of fifteen professions. Sapling's were all unskilled.
#[test]
fn a_plain_zombie_villager_rolls_a_profession_and_a_skin() {
    let shown = summoned(None, 12);
    assert_eq!(shown.len(), 12, "each announced");
    let professions: Vec<i32> = shown.iter().map(|added| int(added, Key::Variant)).collect();
    let skins: Vec<i32> = shown.iter().map(|added| int(added, Key::SkinId)).collect();
    assert!(professions.iter().any(|variant| *variant != 0) && professions.iter().all(|variant| (0..=14).contains(variant)), "{professions:?}");
    assert!(skins.iter().all(|skin| (0..6).contains(skin)) && skins.iter().any(|skin| *skin != skins[0]), "{skins:?}");
    assert!(shown.iter().all(|added| int(added, Key::MaxTradeTier) == 0), "no trades of its own");
}

/// bds-av2cat.txt: the cod click that put a tamed cat in love left it sitting; each cod click after it, while
/// it was in love, stood it up or sat it down.
#[test]
fn a_tamed_cat_in_love_sits_or_stands_at_each_cod_click() {
    let mut h = ready(GameMode::Survival, &[("minecraft:cod", 64, 0)], 0.0);
    let cat = spawn(&mut h, "minecraft:cat", [3.5, 65.0, 3.5]);
    (0..60).map(|_| click_on(&mut h, cat)).find(|sent| showed(sent, cat, Event::TameSuccess)).expect("a third of cod tame");
    let loved = click_on(&mut h, cat);
    assert_ne!(has_flag(&loved, cat, Flags::INLOVE), Some(false), "in love");
    assert_ne!(has_flag(&loved, cat, Flags::SITTING), Some(false), "still sitting");
    assert_eq!(has_flag(&click_on(&mut h, cat), cat, Flags::SITTING), Some(false), "stood up");
    assert_eq!(has_flag(&click_on(&mut h, cat), cat, Flags::SITTING), Some(true), "sat down");
}

/// BDS's `as_baby` zombie villager showed a baby villager's 0.5 and 0.98 by 1.96 and stayed unskilled.
#[test]
fn a_baby_zombie_villager_is_announced_at_a_babys_look() {
    let shown = summoned(Some("minecraft:as_baby"), 1);
    let baby = shown.first().expect("announced");
    assert_eq!((float(baby, Key::Scale), float(baby, Key::BoundingboxWidth), float(baby, Key::BoundingboxHeight)), (0.5, 0.98, 1.96));
    assert_eq!(int(baby, Key::Variant), 0);
}
