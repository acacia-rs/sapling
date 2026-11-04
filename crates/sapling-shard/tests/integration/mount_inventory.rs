//! What a summoned mount shows of the groups its spawn event adds (research/captures/
//! bds-s7mounts.txt): the inventory of a tamed or grown one, and the box of one given no age group.

use crate::common;

use acacia_proto::packets::{AddEntity, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags};
use common::asking::ask;
use common::{Harness, all};
use sapling_api::Difficulty;

fn container(items: &[MetadataDictionaryItem]) -> (Option<i8>, Option<i32>) {
    let kind = items.iter().find(|item| item.key == Key::ContainerType).and_then(|item| if let Value::Default(Plain::Byte(of)) = item.value { Some(of) } else { None });
    let size = items.iter().find(|item| item.key == Key::ContainerBaseSize).and_then(|item| if let Value::Default(Plain::Int(size)) = item.value { Some(size) } else { None });
    (kind, size)
}

fn float(items: &[MetadataDictionaryItem], key: Key) -> Option<f32> {
    items.iter().find(|item| item.key == key).and_then(|item| if let Value::Default(Plain::Float(value)) = item.value { Some(value) } else { None })
}

fn word(items: &[MetadataDictionaryItem]) -> Option<i64> {
    items.iter().find_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0) } else { None })
}

#[test]
fn a_summoned_mount_shows_the_inventory_and_box_its_event_gives() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    ask(&mut h, |ctx| ctx.set_difficulty(Difficulty::Easy));
    let cases: [(&'static str, &'static str, (i8, i32), [f32; 2], bool); 8] = [
        ("minecraft:trader_llama", "minecraft:spawn_tame_adult", (0, 0), [0.9, 1.87], false),
        ("minecraft:skeleton_horse", "minecraft:spawn_adult", (0, 0), [0.6, 1.8], true),
        ("minecraft:zombie_horse", "minecraft:horse_saddled", (0, 0), [0.6, 1.8], false),
        ("minecraft:horse", "minecraft:on_tame", (12, 2), [1.4, 1.6], true),
        ("minecraft:mule", "minecraft:on_tame", (12, 16), [1.4, 1.6], true),
        ("minecraft:camel", "minecraft:camel_saddled", (0, 0), [1.7, 2.375], true),
        ("minecraft:camel", "minecraft:spawn_adult", (12, 5), [1.7, 2.375], true),
        ("minecraft:horse", "minecraft:spawn_adult", (0, 0), [1.4, 1.6], false),
    ];
    for (index, (kind, event, inventory, size, tamed)) in cases.into_iter().enumerate() {
        let z = index as f32 * 4.0 + 0.5;
        h.remote.run(move |ctx| _ = ctx.summon_as(None, kind, [4.5, 65.0, z], 0.0, Some(event)));
        let sent = h.tick(1);
        let added = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == kind).expect("announced");
        let shown = (container(&added.metadata), [float(&added.metadata, Key::BoundingboxWidth), float(&added.metadata, Key::BoundingboxHeight)]);
        assert_eq!(shown, ((Some(inventory.0), Some(inventory.1)), size.map(Some)), "{kind} {event}");
        assert_eq!(word(&added.metadata).map(|word| word & Flags::TAMED.0 != 0), Some(tamed), "{kind} {event}");
        let next = h.tick(1);
        let later: Vec<_> = all::<SetEntityData>(&next).into_iter().filter(|data| data.runtime_entity_id == added.runtime_id && container(&data.metadata).0.is_some()).collect();
        if kind == "minecraft:zombie_horse" {
            // Its pack's upgrade_to_1_21_130 tames it: the word and the inventory alone.
            let [upgrade] = later.try_into().expect("one upgrade");
            assert_eq!(upgrade.metadata.iter().map(|item| item.key).collect::<Vec<_>>(), [Key::Flags, Key::ContainerType, Key::ContainerBaseSize]);
            assert_eq!((container(&upgrade.metadata), word(&upgrade.metadata).map(|word| word & Flags::TAMED.0 != 0)), ((Some(12), Some(2)), Some(true)));
        } else {
            assert!(later.is_empty(), "{kind} {event}: {later:?}");
        }
    }
}
