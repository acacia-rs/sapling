//! Group AR: a mob's AddEntity carries what it was made as, with nothing of it told again in its first tick
//! (research/captures/bds-ar2look.txt, bds-f3convert.txt; mobs/folded.rs).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Meta, MetadataDictionaryItemValueDefault as Plain};
use common::stored::{enter_stored, floor_holding};
use common::{Harness, all, config};
use sapling_world::World;

fn harness() -> Harness {
    let mut h = Harness::with(config(), World::with_storage(floor_holding(Vec::new(), &[])));
    enter_stored(&mut h, 1);
    h
}

/// What the tick a mob of `kind` appeared in told of it: its AddEntity and the items of every
/// SetEntityData about it but its first data (the long one BDS sends too).
fn appeared(sent: &[RawPacket], kind: &str) -> (AddEntity, Vec<Key>) {
    let added = all::<AddEntity>(sent).into_iter().find(|added| added.entity_type == kind).expect("shown");
    let told = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == added.runtime_id && data.metadata.len() < 8);
    (added.clone(), told.flat_map(|data| data.metadata.into_iter().map(|item| item.key)).collect())
}

fn value(added: &AddEntity, key: Key) -> Option<Meta> {
    added.metadata.iter().find(|item| item.key == key).map(|item| item.value.clone())
}

/// bds-ar2look.txt: a summoned armorer's AddEntity says `Variant` 8 and its skin, nothing after it does.
#[test]
fn a_summoned_armorer_is_announced_as_an_armorer() {
    let mut h = harness();
    h.remote.run(|ctx| assert!(ctx.summon_as(None, "minecraft:villager_v2", [2.5, 64.0, 0.5], 0.0, 0.0, Some("minecraft:become_armorer")).is_some()));
    let sent: Vec<_> = (0..2).flat_map(|_| h.tick(1)).collect();
    let (added, told) = appeared(&sent, "minecraft:villager_v2");
    assert_eq!(value(&added, Key::Variant), Some(Meta::Default(Plain::Int(8))));
    assert!(told.iter().all(|key| *key == Key::Flags), "only its flags follow: {told:?}");
}

/// bds-f3convert.txt: a villager killed by a zombie on hard comes back a zombie villager whose AddEntity
/// carries its profession, skin and tiers; no SetEntityData of that tick repeats them.
#[test]
fn a_turned_villager_is_announced_with_what_it_was() {
    let mut h = harness();
    h.remote.run(|ctx| assert!(ctx.summon_as(None, "minecraft:villager_v2", [2.5, 64.0, 0.5], 0.0, 0.0, Some("minecraft:become_armorer")).is_some()));
    let sent: Vec<_> = (0..3).flat_map(|_| h.tick(1)).collect();
    let villager = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == "minecraft:villager_v2").expect("shown");
    let skin = all::<SetEntityData>(&sent).into_iter().filter(|data| data.runtime_entity_id == villager.runtime_id).flat_map(|data| data.metadata).chain(villager.metadata.clone()).find(|item| item.key == Key::SkinId).map(|item| item.value);
    let id = villager.runtime_id;
    h.remote.run(move |ctx| {
        ctx.set_difficulty(sapling_api::Difficulty::Hard);
        let zombie = sapling_api::Attacker::Mob { kind: "minecraft:zombie".into() };
        assert!(ctx.hurt_entity_by(sapling_api::EntityId(id), 100.0, sapling_api::DamageCause::Attack, &zombie));
    });
    let sent = h.tick(1);
    let (added, told) = appeared(&sent, "minecraft:zombie_villager_v2");
    assert_eq!(value(&added, Key::Variant), Some(Meta::Default(Plain::Int(8))));
    assert_eq!(value(&added, Key::MaxTradeTier), Some(Meta::Default(Plain::Int(4))));
    assert_eq!(value(&added, Key::SkinId), skin);
    assert!(![Key::Variant, Key::SkinId, Key::MaxTradeTier, Key::TradeTier].iter().any(|key| told.contains(key)), "told again: {told:?}");
}

/// bds-ar2look.txt: a cow summoned `entity_born` is announced a baby, at its scale and box.
#[test]
fn a_born_cow_is_announced_at_its_baby_scale() {
    let mut h = harness();
    h.remote.run(|ctx| assert!(ctx.summon_as(None, "minecraft:cow", [2.5, 64.0, 0.5], 0.0, 0.0, Some("minecraft:entity_born")).is_some()));
    let sent = h.tick(1);
    let (added, told) = appeared(&sent, "minecraft:cow");
    let (_, scale) = sapling_data::mob_look("minecraft:cow", true).expect("a recorded baby");
    assert_eq!(value(&added, Key::Scale), Some(Meta::Default(Plain::Float(scale))));
    assert!(!told.contains(&Key::Scale) && !told.contains(&Key::BoundingboxWidth), "told again: {told:?}");
}
