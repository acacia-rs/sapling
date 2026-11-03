//! A player's effects as it and those who see it are shown them: `VisibleMobEffects`, as BDS's
//! (research/captures/bds-effects.txt).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all};
use sapling_api::{Effect, PlayerId};

const ALEX: PlayerId = PlayerId(1);

/// The `VisibleMobEffects` of Alex in `sent`.
fn shown(sent: &[RawPacket]) -> Vec<i64> {
    let items = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == 1).flat_map(|data| data.metadata);
    items.filter(|item| item.key == Key::VisibleMobEffects).filter_map(|item| if let Value::Default(Plain::Long(packed)) = item.value { Some(packed) } else { None }).collect()
}

#[test]
fn a_players_effects_with_particles_show_to_it_and_its_viewers() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::Regeneration, 40, 0, true) && ctx.give_effect(ALEX, Effect::Poison, 40, 0, true) && ctx.give_effect(ALEX, Effect::Speed, 40, 0, false)));
    let sent = h.tick_all();
    assert_eq!((shown(sent.to(1)), shown(sent.to(2))), (vec![20, 20 << 7 | 38], vec![20, 20 << 7 | 38]), "the lower id higher; speed hides its particles");
    h.remote.run(|ctx| assert!(ctx.remove_effect(ALEX, Effect::Speed) && ctx.remove_effect(ALEX, Effect::Regeneration)));
    assert_eq!(shown(h.tick_all().to(2)), [38], "no change for the hidden one");
    let gone: Vec<RawPacket> = (0..42).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    assert_eq!(shown(&gone), [0]);
}

#[test]
fn a_viewer_coming_later_finds_a_mobs_effects_in_its_announcement() {
    // research/captures/bds-lateeffects.txt: the rejoined actor's AddEntity of a cow under speed carried 2.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let id = common::asking::ask(&mut h, |ctx| ctx.spawn_entity("minecraft:cow", [3.5, 64.0, 0.5]).expect("a cow"));
    // Slowness with its particles hidden is not shown.
    h.remote.run(move |ctx| assert!(ctx.give_entity_effect(id, Effect::Speed, 600, 1) && ctx.give_entity_effect_shown(id, Effect::Slowness, 600, 255, false)));
    h.tick_all();
    let mut sent: Vec<RawPacket> = h.enter(2, "Steve").to(2).to_vec();
    (0..5).for_each(|_| sent.extend(h.tick_all().to(2).to_vec()));
    let cow = all::<acacia_proto::packets::AddEntity>(&sent).into_iter().find(|added| added.entity_type == "minecraft:cow").expect("the cow announced");
    let visible = cow.metadata.iter().find(|item| item.key == Key::VisibleMobEffects).map(|item| item.value.clone());
    assert_eq!(visible, Some(Value::Default(Plain::Long(2))));
}
