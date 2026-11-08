//! Group BF's mob invisibility (research/captures/bds-bf1give.txt, bds-bf1potion.txt, bds-bf1near.txt,
//! bds-bf1armour.txt; creatures/afflictions/invisible.rs).

use crate::common;

use acacia_proto::packets::{AddEntity, LevelSoundEvent, MobEffect, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags};
use common::shooting::{ready, spawn, watch};
use common::{Harness, all, asking::ask};
use sapling_api::{Block, Difficulty, Effect, EntityId, GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

fn flags(data: &SetEntityData) -> Option<i64> {
    data.metadata.iter().find_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0) } else { None })
}

fn visible(data: &SetEntityData) -> Option<i64> {
    data.metadata.iter().find(|item| item.key == Key::VisibleMobEffects).and_then(|item| if let Value::Default(Plain::Long(shown)) = item.value { Some(shown) } else { None })
}

/// The sounds Alex hears in `sent`: name, whose, height over `ground`.
fn heard(sent: &[acacia_proto::RawPacket], ground: f32) -> Vec<(String, i64, f32)> {
    all::<LevelSoundEvent>(sent).into_iter().map(|sound| (sound.sound_id, sound.entity_unique_id, ((sound.position.y - ground) * 100.0).round() / 100.0)).collect()
}

#[test]
fn an_invisible_mob_shows_its_flags_at_the_sync_and_is_heard_the_tick_after() {
    let mut h = ready(GameMode::Creative, &[], 0.0);
    let [sheep, cow] = [("minecraft:sheep", [3.5, 65.0, 0.5]), ("minecraft:cow", [-4.5, 65.0, 2.5])].map(|(kind, at)| spawn(&mut h, kind, at));
    watch(&mut h, 2);
    h.remote.run(move |ctx| [sheep, cow].iter().for_each(|id| assert!(ctx.give_entity_effect(EntityId(*id), Effect::Invisibility, 100, 0))));
    let sent = h.tick_all().to(1).to_vec();
    assert!(all::<MobEffect>(&sent).is_empty(), "BDS tells no MobEffect of a mob's invisibility");
    let shown: Vec<(u64, i64, Option<i64>)> = all::<SetEntityData>(&sent).iter().filter_map(|data| Some((data.runtime_entity_id, flags(data)?, visible(data)))).collect();
    assert_eq!(shown.len(), 2, "{shown:?}");
    assert_eq!(shown.iter().map(|(id, ..)| *id).collect::<Vec<_>>(), [cow, sheep], "the sync's order");
    assert!(shown.iter().all(|(_, word, effects)| word & Flags::INVISIBLE.0 != 0 && word & Flags::CAN_SHOW_NAMETAG.0 == 0 && *effects == Some(28)), "{shown:?}");
    let next = h.tick_all().to(1).to_vec();
    assert_eq!(heard(&next, 65.0), [("disappeared".into(), cow as i64, 0.65), ("disappeared".into(), sheep as i64, 0.65)]);
    h.remote.run(move |ctx| assert!(ctx.give_entity_effect(EntityId(sheep), Effect::Invisibility, 200, 1)));
    let raised = watch(&mut h, 2);
    assert!(all::<SetEntityData>(&raised).iter().all(|data| flags(data).is_none()) && heard(&raised, 65.0).is_empty(), "a raise tells nothing");

    // A hurt rebuilds its announcement; the flags stay in it.
    assert!(ask(&mut h, move |ctx| ctx.hurt_entity(EntityId(sheep), 1.0, sapling_api::DamageCause::Generic)));
    watch(&mut h, 30);
    let mut late: Vec<AddEntity> = all::<AddEntity>(h.enter(3, "Notch").to(3));
    (0..5).for_each(|_| late.extend(all::<AddEntity>(h.tick_all().to(3))));
    let announced = late.iter().find(|added| added.runtime_id == sheep).and_then(|added| added.metadata.iter().find_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0) } else { None }));
    assert!(announced.is_some_and(|word| word & Flags::INVISIBLE.0 != 0), "{announced:?}");

    h.remote.run(move |ctx| assert!(ctx.remove_entity_effect(EntityId(sheep), Effect::Invisibility)));
    let sent = h.tick_all().to(1).to_vec();
    let back: Vec<i64> = all::<SetEntityData>(&sent).iter().filter(|data| data.runtime_entity_id == sheep).filter_map(flags).collect();
    assert!(matches!(back[..], [word] if word & Flags::INVISIBLE.0 == 0), "{back:?}");
    assert_eq!(heard(&h.tick_all().to(1).to_vec(), 65.0), [("reappeared".into(), sheep as i64, 0.65)]);
}

/// Whether a zombie `off` blocks along z from Alex, invisible and wearing `worn` iron pieces, takes Alex in 40 ticks.
fn takes(off: f32, invisible: bool, worn: usize) -> bool {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let feet = [0.5, 65.0, 0.5 + off];
    ask(&mut h, move |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        for x in -3..=3 {
            for z in -3..=20 {
                assert!(ctx.set_block([x, 64, z], &Block::new("stone")));
            }
        }
        for (piece, name) in ["iron_helmet", "iron_chestplate", "iron_leggings", "iron_boots"].into_iter().enumerate().take(worn) {
            assert!(ctx.set_item(ALEX, Slot::Armor(piece as u8), Some(&Item::new(&format!("minecraft:{name}"), 1))));
        }
        if invisible {
            assert!(ctx.give_effect(ALEX, Effect::Invisibility, 2000, 0, false));
        }
        assert!(ctx.spawn_entity("minecraft:zombie", feet).is_some());
    });
    (0..40).any(|_| all::<SetEntityData>(h.tick_all().to(ALEX.0)).iter().any(|data| data.metadata.iter().any(|item| item.key == Key::TargetEid && !matches!(item.value, Value::Default(Plain::Long(0))))))
}

#[test]
fn a_hunter_sees_an_invisible_player_from_a_share_of_its_sight_by_the_armour_worn() {
    // 35 sight: bare 2.45 from the eyes, two pieces 12.25, four 24.5.
    assert!(takes(4.0, false, 0), "seen");
    assert!(!takes(4.0, true, 0), "bare and invisible, 4 off");
    assert!(takes(1.8, true, 0), "bare and invisible, 2.41 from the eyes");
    assert!(takes(9.0, true, 2) && !takes(9.0, true, 1), "two pieces, then one, 9 off");
}
