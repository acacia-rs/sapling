//! Group BG: a player's invisibility, named mobs' flags and late viewers' live flags
//! (research/captures/bds-bg2player.txt, bds-bg2names.txt; vitals/effects/invisible.rs, mobs/live.rs).

use std::sync::{Arc, Mutex};

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, AddPlayer, LevelSoundEvent, MobEffect, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags};
use common::{Harness, Sent, all, asking::ask};
use sapling_api::{Difficulty, Effect, EntityId, PlayerId, ShardCtx};

const ALEX: PlayerId = PlayerId(1);

/// [`ask`], with what the tick it is answered in sent.
fn ask_sent<T: Send + 'static>(h: &mut Harness, ask: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> (T, Sent) {
    let found = Arc::new(Mutex::new(None));
    let out = found.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = Some(ask(ctx)));
    let sent = h.tick_all();
    (found.lock().unwrap().take().expect("the handler ran"), sent)
}

fn flags_of(metadata: &[MetadataDictionaryItem]) -> Option<i64> {
    metadata.iter().find_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0) } else { None })
}

fn keys(data: &SetEntityData) -> Vec<Key> {
    data.metadata.iter().map(|item| item.key).collect()
}

/// Alex's SetEntityData in `sent`.
fn alex_data(sent: &[RawPacket]) -> Vec<SetEntityData> {
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == 1).collect()
}

/// The sounds about Alex in `sent`: name and height over its feet at 65.
fn heard(sent: &[RawPacket]) -> Vec<(String, f32)> {
    all::<LevelSoundEvent>(sent).into_iter().filter(|sound| sound.entity_unique_id == 1).map(|sound| (sound.sound_id, ((sound.position.y - 65.0) * 100.0).round() / 100.0)).collect()
}

#[test]
fn an_invisible_player_shows_its_flags_at_the_sync_and_is_heard_the_tick_after() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let ((), sent) = ask_sent(&mut h, |ctx| assert!(ctx.give_effect(ALEX, Effect::Invisibility, 600, 0, false)));
    for viewer in [1, 2] {
        let told = sent.to(viewer);
        let effect = told.iter().position(|packet| packet.decode::<MobEffect>().is_ok()).expect("the MobEffect");
        let data = alex_data(told);
        assert!(matches!(&data[..], [data] if keys(data) == [Key::Flags]), "flags alone with hidden particles: {:?}", data.iter().map(keys).collect::<Vec<_>>());
        let word = flags_of(&data[0].metadata).expect("flags");
        assert!(word & Flags::INVISIBLE.0 != 0 && word & Flags::CAN_SHOW_NAMETAG.0 == 0, "{word}");
        let at = told.iter().position(|packet| packet.decode::<SetEntityData>().is_ok_and(|data| data.runtime_entity_id == 1)).expect("data");
        assert!(effect < at, "the effect first, the flags at the sync");
    }
    assert!(heard(sent.to(1)).is_empty());
    let next = h.tick_all();
    assert_eq!((heard(next.to(1)), heard(next.to(2))), (vec![("disappeared".into(), 0.9)], vec![("disappeared".into(), 0.9)]));

    let ((), first) = ask_sent(&mut h, |ctx| assert!(ctx.give_effect(ALEX, Effect::Invisibility, 800, 1, false)));
    let mut raised = first.to(2).to_vec();
    (0..2).for_each(|_| raised.extend(h.tick_all().to(2).iter().cloned()));
    assert!(alex_data(&raised).is_empty() && heard(&raised).is_empty(), "a raise tells nothing");

    let late = h.enter(3, "Notch").to(3).to_vec();
    let added = all::<AddPlayer>(&late).into_iter().find(|added| added.runtime_id == 1).expect("Alex announced");
    assert!(flags_of(&added.metadata).is_some_and(|word| word & Flags::INVISIBLE.0 != 0), "a late viewer sees Alex invisible");

    let ((), back) = ask_sent(&mut h, |ctx| assert!(ctx.remove_effect(ALEX, Effect::Invisibility)));
    let word = alex_data(back.to(2)).iter().find_map(|data| flags_of(&data.metadata)).expect("flags");
    assert!(word & Flags::INVISIBLE.0 == 0 && word & Flags::CAN_SHOW_NAMETAG.0 != 0, "seen again, its name shows for good: {word}");
    assert_eq!(heard(h.tick_all().to(2)), [("reappeared".into(), 0.9)]);

    let ((), shown) = ask_sent(&mut h, |ctx| assert!(ctx.give_effect(ALEX, Effect::Invisibility, 40, 0, true)));
    let shown = alex_data(shown.to(2));
    assert!(matches!(&shown[..], [data] if keys(data) == [Key::Flags, Key::VisibleMobEffects]), "flags then effects in one: {:?}", shown.iter().map(keys).collect::<Vec<_>>());
}

/// A sheep summoned at the platform's middle, named `name` in the summoning call where it is given, and
/// what was sent in the tick it came.
fn summoned(h: &mut Harness, name: Option<&'static str>) -> (u64, Sent) {
    ask_sent(h, move |ctx| {
        let sheep = ctx.summon_as(None, "minecraft:sheep", [2.5, 65.0, 2.5], 0.0, 0.0, None).expect("summoned");
        if let Some(name) = name {
            assert!(ctx.name_entity(sheep, name));
        }
        sheep.0
    })
}

fn nametag(metadata: &[MetadataDictionaryItem]) -> Option<String> {
    metadata.iter().find(|item| item.key == Key::Nametag).and_then(|item| if let Value::Default(Plain::String(name)) = &item.value { Some(name.clone()) } else { None })
}

#[test]
fn a_mob_named_as_it_is_summoned_shows_its_name_in_its_announcement_and_flags() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let (sheep, first) = summoned(&mut h, Some("a"));
    let sent: Vec<RawPacket> = first.to(1).iter().cloned().chain((0..2).flat_map(|_| h.tick_all().to(1).to_vec())).collect();
    let added = all::<AddEntity>(&sent).into_iter().find(|added| added.runtime_id == sheep).expect("announced");
    assert_eq!(nametag(&added.metadata).as_deref(), Some("a"));
    assert!(flags_of(&added.metadata).is_some_and(|word| word & Flags::CAN_SHOW_NAMETAG.0 != 0), "bds-bg2names.txt: 844424934866944");
    let data: Vec<SetEntityData> = all::<SetEntityData>(&sent).into_iter().filter(|data| data.runtime_entity_id == sheep).collect();
    assert!(data.iter().all(|data| !data.metadata.iter().any(|item| item.key == Key::AlwaysShowNametag && item.value != Value::Default(Plain::Byte(0)))), "no plugin label");
    let words: Vec<i64> = data.iter().filter(|data| data.metadata.len() == 1).filter_map(|data| flags_of(&data.metadata)).collect();
    assert!(words.iter().all(|word| word & Flags::CAN_SHOW_NAMETAG.0 != 0) && !words.is_empty(), "its breath keeps the name: {words:?}");
}

#[test]
fn a_late_viewer_is_announced_a_mob_with_the_flags_it_has_now() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let (sheep, _) = summoned(&mut h, Some("a"));
    let (plain, _) = summoned(&mut h, None);
    let creeper = ask(&mut h, |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.summon_as(None, "minecraft:creeper", [-2.5, 65.0, 2.5], 0.0, 0.0, None).expect("summoned").0
    });
    (0..5).for_each(|_| drop(h.tick_all()));
    assert!(ask(&mut h, move |ctx| ctx.fire_entity_event(EntityId(creeper), "minecraft:become_charged")));
    (0..5).for_each(|_| drop(h.tick_all()));
    let late = h.enter(3, "Notch").to(3).to_vec();
    let mut added = all::<AddEntity>(&late);
    (0..3).for_each(|_| added.extend(all::<AddEntity>(h.tick_all().to(3))));
    let word = |id: u64| added.iter().find(|added| added.runtime_id == id).and_then(|added| flags_of(&added.metadata)).expect("announced");
    // bds-lateeffects.txt, bds-bg2names.txt: BREATHING as it breathes now, a charge, a name.
    assert!([sheep, plain, creeper].iter().all(|id| word(*id) & Flags::BREATHING.0 != 0), "breathing");
    assert!(word(sheep) & Flags::CAN_SHOW_NAMETAG.0 != 0 && word(plain) & Flags::CAN_SHOW_NAMETAG.0 == 0);
    assert!(word(creeper) & Flags::POWERED.0 != 0, "charged");
}
