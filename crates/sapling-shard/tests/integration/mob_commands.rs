//! What plugins, and through them `/effect`, `/damage` and `/kill`, see of and do to a mob's
//! effects and health, and what a lying item holds.

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::RawPacket;
use acacia_proto::packets::{EntityEvent, EntityEventEventId, MobEffect, MobEffectEventId, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemValue as MetaValue, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1};
use common::{Harness, all};
use sapling_api::{Block, DamageCause, Effect, EntityId, GameMode, PlayerId, ShardCtx};

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
