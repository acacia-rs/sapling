//! Slimes: sized, hopping at their prey and hurting it by touch, split as they die
//! (research/captures/bds-kinds2.txt).

use crate::common;

use acacia_proto::packets::{AddEntity, EntityEvent, EntityEventEventId, LevelSoundEvent, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all};
use sapling_api::{Difficulty, EntityId};

fn width(added: &AddEntity) -> Option<f32> {
    added.metadata.iter().find(|item| item.key == Key::BoundingboxWidth).and_then(|item| match item.value {
        Value::Default(Plain::Float(width)) => Some(width),
        _ => None,
    })
}

fn summon(h: &mut Harness, event: &'static str, feet: [f32; 3]) {
    h.remote.run(move |ctx| assert!(ctx.summon_as(None, "minecraft:slime", feet, 0.0, Some(event)).is_some()));
}

#[test]
fn a_medium_slime_takes_its_prey_squishes_as_it_hops_at_it_and_hurts_it_for_2_by_touch() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| ctx.set_difficulty(Difficulty::Easy));
    summon(&mut h, "spawn_medium", [0.5, 65.0, 5.5]);
    let (mut shown, mut targeted, mut ages, mut hurt, mut moves) = (None, false, Vec::new(), Vec::new(), Vec::new());
    for _ in 0..120 {
        let sent = h.tick_all().to(1).to_vec();
        shown = shown.or(all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == "minecraft:slime").map(|added| (added.runtime_id, width(&added), added.attributes.iter().find(|a| a.name == "minecraft:health").map(|a| a.max))));
        let Some((slime, ..)) = shown else { continue };
        for data in all::<SetEntityData>(&sent).iter().filter(|data| data.runtime_entity_id == slime) {
            targeted |= data.metadata.iter().any(|item| item.key == Key::TargetEid && matches!(item.value, Value::Default(Plain::Long(1))));
            ages.extend(data.metadata.iter().filter(|item| item.key == Key::EntityAge).filter_map(|item| match item.value {
                Value::Default(Plain::Byte(age)) => Some(age),
                _ => None,
            }));
        }
        hurt.extend(all::<EntityEvent>(&sent).iter().filter(|event| event.runtime_entity_id == 1 && event.event_id == EntityEventEventId::HurtAnimation).map(|event| event.data));
        moves.extend(all::<acacia_proto::packets::MoveEntityDelta>(&sent).iter().filter(|m| m.runtime_entity_id == slime).map(|m| (m.y, m.z)));
    }
    assert_eq!(shown.map(|(_, width, max)| (width, max)), Some((Some(1.04), Some(4.0))));
    assert!(targeted, "takes the player");
    // The window may end with it in the air, squished and not yet landed.
    assert!(ages.len() >= 4 && ages.chunks_exact(2).all(|pair| pair == [2, 1]) && (ages.len() % 2 == 0 || ages.last() == Some(&2)), "{ages:?}");
    assert!(!hurt.is_empty(), "touched: {moves:?}");
}

#[test]
fn a_large_slime_killed_leaves_two_to_four_medium_ones_heard_born_and_a_small_one_none() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| ctx.set_difficulty(Difficulty::Easy));
    summon(&mut h, "spawn_large", [0.5, 65.0, 8.5]);
    summon(&mut h, "spawn_small", [4.5, 65.0, 8.5]);
    let ids: Vec<u64> = all::<AddEntity>(h.tick_all().to(1)).iter().filter(|added| added.entity_type == "minecraft:slime").map(|added| added.runtime_id).collect();
    assert_eq!(ids.len(), 2);
    h.remote.run(move |ctx| assert!(ids.iter().all(|&id| ctx.kill_entity(EntityId(id)))));
    let (mut born, mut sizes) = (0, Vec::new());
    for _ in 0..40 {
        let sent = h.tick_all().to(1).to_vec();
        sizes.extend(all::<AddEntity>(&sent).iter().filter(|added| added.entity_type == "minecraft:slime").map(width));
        born += all::<LevelSoundEvent>(&sent).iter().filter(|sound| sound.sound_id == "born").count();
    }
    assert!((2..=4).contains(&sizes.len()) && sizes.iter().all(|width| *width == Some(1.04)), "{sizes:?}");
    assert_eq!(born, sizes.len());
}
