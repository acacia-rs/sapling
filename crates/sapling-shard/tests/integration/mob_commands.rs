//! What plugins, and through them `/effect`, `/damage` and `/kill`, see of and do to a mob's
//! effects and health, and what lying items hold and how they join up.

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::RawPacket;
use acacia_proto::packets::{EntityEvent, EntityEventEventId, MobArmorEquipment, MobEffect, MobEffectEventId, SetEntityData, UpdateAttributes};
use acacia_proto::types::{MetadataDictionaryItemValue as MetaValue, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1};
use common::{Harness, all};
use sapling_api::{Block, DamageCause, Effect, EntityId, GameMode, Item, PlayerId, ShardCtx, Slot};

/// A creative player and a cow 4 off it.
fn cow() -> (Harness, EntityId) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let (id, _) = ask(&mut h, |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        ctx.spawn_entity("minecraft:cow", [4.5, 65.0, 0.5])
    });
    (h, id.expect("a cow"))
}

/// What `call` returns, run on the shard in its next tick, and what that tick sent the player.
fn ask<T: Send + 'static>(h: &mut Harness, call: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> (T, Vec<RawPacket>) {
    let answer = Arc::new(Mutex::new(None));
    let put = answer.clone();
    h.remote.run(move |ctx| *put.lock().unwrap() = Some(call(ctx)));
    let sent = h.tick_all().to(1).to_vec();
    (answer.lock().unwrap().take().expect("the shard ran the call"), sent)
}

#[test]
fn a_mobs_effects_come_off_one_or_all_and_its_viewers_are_told() {
    let (mut h, id) = cow();
    let (given, _) = ask(&mut h, move |ctx| ctx.give_entity_effect(id, Effect::Speed, 600, 0) && ctx.give_entity_effect(id, Effect::Slowness, 600, 0));
    assert!(given);
    let (taken, sent) = ask(&mut h, move |ctx| (ctx.remove_entity_effect(id, Effect::Slowness), ctx.remove_entity_effect(id, Effect::Slowness)));
    assert_eq!(taken, (true, false), "slowness comes off once");
    let removed: Vec<i32> = all::<MobEffect>(&sent).iter().filter(|told| told.event_id == MobEffectEventId::Remove).map(|told| told.effect_id).collect();
    assert_eq!(removed, [Effect::Slowness.id()]);
    let (cleared, _) = ask(&mut h, move |ctx| (ctx.clear_entity_effects(id), ctx.clear_entity_effects(id)));
    assert_eq!(cleared, (true, false), "speed was left, then nothing");
}

/// BDS's cow under health boost V: most 30 over its own 10, the boost's 20 as a modifier
/// (research/captures/bds-hits2.txt, bds-boosts.txt).
#[test]
fn a_health_boost_raises_a_mobs_most_health_until_it_comes_off() {
    let (mut h, id) = cow();
    let health = |sent: &[RawPacket]| -> Vec<[f32; 4]> {
        let told = all::<UpdateAttributes>(sent).into_iter().filter(|told| told.runtime_entity_id == id.0);
        told.filter_map(|told| told.attributes.into_iter().find(|it| it.name == "minecraft:health")).map(|it| [it.current, it.max, it.default_max, it.modifiers.iter().map(|it| it.amount).sum()]).collect()
    };
    let (_, sent) = ask(&mut h, move |ctx| ctx.give_entity_effect(id, Effect::HealthBoost, 600, 4));
    assert_eq!(health(&sent), [[10.0, 30.0, 10.0, 20.0]]);
    let (_, mut sent) = ask(&mut h, move |ctx| ctx.give_entity_effect(id, Effect::InstantHealth, 1, 4));
    sent.extend(ask(&mut h, |_| ()).1);
    assert_eq!(health(&sent), [[30.0, 30.0, 10.0, 20.0]]);
    let (_, sent) = ask(&mut h, move |ctx| ctx.remove_entity_effect(id, Effect::HealthBoost));
    assert_eq!(health(&sent), [[10.0, 10.0, 10.0, 0.0]]);
}

/// BDS's zombie in a diamond chestplate took 5.824 of a diamond sword's 8: its own 2 points with
/// the chestplate's 8 and toughness 2 (research/captures/bds-hits2.txt).
#[test]
fn a_zombie_in_a_diamond_chestplate_shows_it_and_takes_less_of_a_sword() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let (zombie, sent) = ask(&mut h, |ctx| {
        // Peaceful takes monsters away.
        ctx.set_difficulty(sapling_api::Difficulty::Easy);
        ctx.set_time(18_000);
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        assert!(ctx.set_item(PlayerId(1), Slot::Main(0), Some(&Item::new("minecraft:diamond_sword", 1))));
        let zombie = ctx.spawn_entity("minecraft:zombie", [0.5, 65.0, 2.0]).expect("a zombie");
        assert!(ctx.set_entity_armour(zombie, 1, Some(&Item::new("minecraft:diamond_chestplate", 1))));
        assert!(!ctx.set_entity_armour(zombie, 4, None), "no fifth piece");
        zombie
    });
    let worn = all::<MobArmorEquipment>(&sent);
    assert!(worn.iter().any(|worn| worn.runtime_entity_id == zombie.0 && worn.chestplate.network_id != 0), "the chestplate is shown");
    h.receive(1, &common::attack(zombie.0));
    let sent: Vec<RawPacket> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let left: Vec<f32> = all::<UpdateAttributes>(&sent).into_iter().filter(|told| told.runtime_entity_id == zombie.0).flat_map(|told| told.attributes).filter(|it| it.name == "minecraft:health").map(|it| it.current).collect();
    assert!(left.first().is_some_and(|left| (left - 14.176).abs() < 1e-3), "{left:?}");
    // `/damage` goes through it too (research/captures/bds-boosts.txt), once the hit's guard is over.
    (0..12).for_each(|_| drop(h.tick_all()));
    let (_, mut sent) = ask(&mut h, move |ctx| assert!(ctx.hurt_entity(zombie, 8.0, DamageCause::Generic)));
    sent.extend(ask(&mut h, |_| ()).1);
    let left: Vec<f32> = all::<UpdateAttributes>(&sent).into_iter().filter(|told| told.runtime_entity_id == zombie.0).flat_map(|told| told.attributes).filter(|it| it.name == "minecraft:health").map(|it| it.current).collect();
    assert!(left.first().is_some_and(|left| (left - 8.352).abs() < 1e-3), "{left:?}");
}

#[test]
fn a_hurt_mob_flashes_and_a_dead_one_takes_no_more() {
    let (mut h, id) = cow();
    let (hurt, sent) = ask(&mut h, move |ctx| ctx.hurt_entity(id, 4.0, DamageCause::Generic));
    let flashed = all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == id.0 && event.event_id == EntityEventEventId::HurtAnimation);
    assert!(hurt && flashed, "hurt {hurt}, flashed {flashed}");
    let (killed, _) = ask(&mut h, move |ctx| (ctx.hurt_entity(id, 100.0, DamageCause::Generic), ctx.hurt_entity(id, 1.0, DamageCause::Generic)));
    assert_eq!(killed, (true, false), "the harder hit kills it, the next finds it dying");
    assert!(!ask(&mut h, |ctx| ctx.hurt_entity(EntityId(9999), 1.0, DamageCause::Generic)).0, "no such mob");
}

#[test]
fn a_mob_breathes_until_water_covers_its_head_and_holds_its_breath_under_it() {
    let (mut h, cow) = cow();
    let breathing = |sent: &[RawPacket]| -> Vec<(bool, Option<i16>)> {
        let told = all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == cow.0);
        let flags = |data: &SetEntityData| data.metadata.iter().find_map(|item| if let MetaValue::Flags(flags) = item.value { Some(flags.contains(MetadataFlags1::BREATHING)) } else { None });
        let air = |data: &SetEntityData| data.metadata.iter().find_map(|item| if let MetaValue::Default(Plain::Short(air)) = item.value { Some(air) } else { None });
        told.filter_map(|data| flags(&data).map(|on| (on, air(&data)))).collect()
    };
    let (_, sent) = ask(&mut h, |_| ());
    assert_eq!(breathing(&sent), [], "it was told breathing already");
    let (_, sent) = ask(&mut h, |ctx| assert!((65..=67).all(|y| ctx.set_block([4, y, 0], &Block::new("water")))));
    let (_, later) = ask(&mut h, |_| ());
    assert_eq!([breathing(&sent), breathing(&later)].concat(), [(false, Some(299))], "the flag goes with its first held breath");
}

#[test]
fn a_lying_item_tells_what_it_holds() {
    let (mut h, cow) = cow();
    ask(&mut h, |ctx| assert!(ctx.set_block([2, 65, 0], &Block::new("dirt")) && ctx.break_block([2, 65, 0])));
    let (held, _) = ask(&mut h, move |ctx| ctx.entities().into_iter().map(|entity| (entity.id == cow, entity.item.map(|item| item.name))).collect::<Vec<_>>());
    assert_eq!(held, [(true, None), (false, Some("minecraft:dirt".to_owned()))]);
}

#[test]
fn lying_items_of_a_kind_join_up_on_the_25th_tick() {
    let (mut h, _) = cow();
    // One above the other, so that the second block does not push the first's item out; then together, as they slide apart.
    ask(&mut h, |ctx| assert!([65, 66].into_iter().all(|y| ctx.set_block([2, y, 0], &Block::new("dirt")) && ctx.break_block([2, y, 0]))));
    ask(&mut h, |ctx| {
        for (entity, dx) in ctx.entities().into_iter().filter(|entity| entity.item.is_some()).zip([0.0, 0.6]) {
            assert!(ctx.move_entity(entity.id, [2.2 + dx, 65.0, 0.5]) && ctx.set_velocity(entity.id, [0.0; 3]));
        }
    });
    let by_tick: Vec<Vec<_>> = (0..30).map(|_| h.tick_all().to(1).to_vec()).collect();
    let tick_of = |seen: &dyn Fn(&[acacia_proto::RawPacket]) -> bool| by_tick.iter().position(|sent| seen(sent));
    let merged = tick_of(&|sent| all::<EntityEvent>(sent).iter().any(|event| event.event_id == EntityEventEventId::ItemEntityMerge));
    let removed = tick_of(&|sent| !all::<acacia_proto::packets::RemoveEntity>(sent).is_empty());
    assert_eq!(removed, merged.map(|tick| tick + 1), "the one taken in goes a tick later (bds-t2epack.txt)");
    let sent: Vec<_> = by_tick.concat();
    let merges: Vec<i32> = all::<EntityEvent>(&sent).iter().filter(|event| event.event_id == EntityEventEventId::ItemEntityMerge).map(|event| event.data).collect();
    let (held, _) = ask(&mut h, |ctx| ctx.entities().into_iter().filter_map(|entity| entity.item.map(|item| item.count)).collect::<Vec<_>>());
    assert_eq!((merges, held), (vec![2], vec![2]));
}
