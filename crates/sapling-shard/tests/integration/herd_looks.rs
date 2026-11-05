//! Spawn events that set a look or make riders, as BDS showed them (research/captures/bds-h3herds.txt):
//! a horse's coat, a tropical fish's four looks, and the riders of a zombie horse, a camel husk and
//! a drowned's zombie nautilus.

use crate::common;

use acacia_proto::packets::{AddEntity, MobEquipment, SetEntityLink};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValueDefault as Plain, WindowID};
use common::asking::ask;
use common::raising::plain_in;
use common::{Harness, all};
use sapling_api::Difficulty;

fn int(added: &AddEntity, key: Key) -> i32 {
    match plain_in(&added.metadata, key) {
        Some(Plain::Int(value)) => value,
        Some(Plain::Byte(value)) => value.into(),
        _ => 0,
    }
}

#[test]
fn summon_events_set_the_looks_bds_showed() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let summons = [("minecraft:horse", "minecraft:make_black"), ("minecraft:tropicalfish", "minecraft:become_red_snapper"), ("minecraft:tropicalfish", "minecraft:become_dog_fish")];
    h.remote.run(move |ctx| {
        for (n, (kind, event)) in summons.into_iter().enumerate() {
            assert!(ctx.summon_as(None, kind, [n as f32 * 3.0 + 2.5, 65.0, 4.5], 0.0, 0.0, Some(event)).is_some(), "{kind}");
        }
    });
    let sent: Vec<_> = (0..5).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let mut added: Vec<AddEntity> = all::<AddEntity>(&sent).into_iter().filter(|added| summons.iter().any(|(kind, _)| added.entity_type == *kind)).collect();
    added.sort_by_key(|added| added.runtime_id);
    let looks: Vec<[i32; 4]> = added.iter().map(|added| [Key::Variant, Key::MarkVariant, Key::Color, Key::Color2].map(|key| int(added, key))).collect();
    assert_eq!(looks, [[4, 0, 0, 0], [1, 3, 14, 0], [1, 3, 10, 4]]);
}

/// The pack's `entity_spawned` makes one horse in five a foal (BDS: 2 of 4 plain summons).
#[test]
fn a_horse_summoned_with_no_event_may_come_a_foal() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        for n in 0..30 {
            assert!(ctx.summon_as(None, "minecraft:horse", [(n % 6) as f32 * 2.0 + 2.5, 65.0, (n / 6) as f32 * 2.0 + 4.5], 0.0, 0.0, None).is_some());
        }
    });
    let sent: Vec<_> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let foal = |added: &AddEntity| added.metadata.iter().any(|item| matches!(item.value, acacia_proto::types::MetadataDictionaryItemValue::Flags(flags) if flags.contains(acacia_proto::types::MetadataFlags1::BABY)));
    let foals = all::<AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:horse").filter(|added| foal(added)).count();
    assert!((1..15).contains(&foals), "{foals} of 30");
}

#[test]
fn rider_events_make_their_riders_and_mounts() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    ask(&mut h, |ctx| ctx.set_difficulty(Difficulty::Easy));
    let summons = [("minecraft:zombie_horse", "minecraft:spawn_adult_with_rider"), ("minecraft:camel_husk", "minecraft:spawn_with_rider"), ("minecraft:drowned", "minecraft:as_rider")];
    h.remote.run(move |ctx| {
        for (n, (kind, event)) in summons.into_iter().enumerate() {
            assert!(ctx.summon_as(None, kind, [n as f32 * 4.0 + 2.5, 65.0, 4.5], 0.0, 0.0, Some(event)).is_some(), "{kind}");
        }
    });
    let sent: Vec<_> = (0..5).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let kinds = ask(&mut h, |ctx| ctx.entities().into_iter().map(|entity| entity.kind).collect::<Vec<_>>());
    let count = |kind: &str| kinds.iter().filter(|of| *of == kind).count();
    assert_eq!(["minecraft:zombie", "minecraft:husk", "minecraft:parched", "minecraft:zombie_nautilus"].map(count), [1, 1, 1, 1], "{kinds:?}");
    let riders: std::collections::HashSet<_> = all::<SetEntityLink>(&sent).iter().filter(|link| link.link.r#type != 0).map(|link| link.link.rider_entity_id).collect();
    assert_eq!(riders.len(), 4, "each rides");
    // Each rider's equipment table: the zombie's and husk's spear, the parched's bow, the drowned's trident.
    let added = all::<AddEntity>(&sent);
    let held = |kind: &str| {
        let id = added.iter().find(|added| added.entity_type == kind).unwrap_or_else(|| panic!("{kind} shown")).runtime_id;
        all::<MobEquipment>(&sent).into_iter().filter(|held| held.runtime_entity_id == id && held.window_id == WindowID::Inventory).map(|held| held.item.network_id).find(|item| *item != 0)
    };
    let names = ["minecraft:iron_spear", "minecraft:iron_spear", "minecraft:bow", "minecraft:trident"].map(|name| sapling_data::item(&sapling_data::Stack { name, count: 1, damage: 0, block: None, tag: None }, None).map(|item| item.network_id));
    assert_eq!(["minecraft:zombie", "minecraft:husk", "minecraft:parched", "minecraft:drowned"].map(held), names);
}
