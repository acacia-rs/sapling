//! Group K of the fourteenth wave: summoned babies, the mule, a wandering trader's llamas, a
//! crossbow's loading, mobs in powder snow and the sniffer's search (research/captures/bds-k2*.txt,
//! fourteenth2k_drill.rs).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddItemEntity, EntityEvent, EntityEventEventId as Event, LevelSoundEvent, RemoveEntity, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemValue as MetaValue, MetadataFlags1 as Flags};
use common::asking::ask;
use common::raising::{click_on, has_flag};
use common::shooting::{added, ready, spawn, watch};
use common::{Harness, all};
use sapling_api::{Block, Difficulty, Effect, EntityId, GameMode, ShardCtx};
use std::sync::{Arc, Mutex};

const TRADER: &str = "minecraft:wandering_trader";
const LLAMA: &str = "minecraft:trader_llama";

/// Alex alone on the floor.
fn floor() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h
}

/// What `make` returned on the shard's next tick, and what Alex was told from that tick on, `ticks` in all.
fn made<T: Send + 'static>(h: &mut Harness, ticks: usize, make: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> (T, Vec<RawPacket>) {
    let answer = Arc::new(Mutex::new(None));
    let put = answer.clone();
    h.remote.run(move |ctx| *put.lock().unwrap() = Some(make(ctx)));
    let sent = watch(h, ticks);
    (answer.lock().unwrap().take().expect("the shard ran it"), sent)
}

/// The runtime ids of `count` `kind`s summoned in a grid, and what Alex was told of them.
fn summoned(h: &mut Harness, kind: &'static str, count: usize) -> (Vec<u64>, Vec<RawPacket>) {
    made(h, 3, move |ctx| (0..count).filter_map(|n| ctx.summon_as(None, kind, [(n % 10) as f32 - 4.5, 65.0, (n / 10 % 10) as f32 - 4.5], 0.0, 0.0, None)).map(|id| id.0).collect::<Vec<u64>>())
}

#[test]
fn a_summoned_cow_is_now_and_then_a_baby_and_a_cat_a_quarter_of_the_time() {
    // bds-k2babies.txt: of 100 summons, cows 4 babies, pigs 6, chickens 6, sheep 3, mooshrooms 3, cats 24.
    for (kind, least, most) in [("minecraft:cow", 1, 14), ("minecraft:pig", 1, 14), ("minecraft:cat", 12, 40)] {
        let mut h = floor();
        let (mut ids, mut sent) = summoned(&mut h, kind, 100);
        assert!(ask(&mut h, move |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == kind).count()) == 100, "{kind}: all came");
        let (more, told) = summoned(&mut h, kind, 100);
        ids.extend(more);
        sent.extend(told);
        let babies = ids.iter().filter(|id| has_flag(&sent, **id, Flags::BABY) == Some(true)).count();
        assert!((least * 2..=most * 2).contains(&babies), "{kind}: {babies} of 200");
    }
}

#[test]
fn a_tamed_horse_and_donkey_fed_golden_carrots_have_a_mule() {
    // bds-k2mounts.txt: a mule at their feet with 3 orbs.
    let mut h = ready(GameMode::Survival, &[("minecraft:golden_carrot", 8, 0)], 0.0);
    let parents = ask(&mut h, |ctx| {
        ["minecraft:horse", "minecraft:donkey"].iter().zip([0.5, 1.5]).map(|(kind, z)| ctx.summon_as(None, kind, [2.5, 65.0, z], 0.0, 0.0, Some("minecraft:spawn_tame_adult")).expect("summoned").0).collect::<Vec<u64>>()
    });
    let held = parents.clone();
    assert!(ask(&mut h, move |ctx| held.iter().all(|parent| ctx.give_entity_effect(EntityId(*parent), Effect::Slowness, 400, 255))));
    watch(&mut h, 3);
    parents.iter().for_each(|parent| drop(click_on(&mut h, *parent)));
    let sent = watch(&mut h, 200);
    assert_eq!(added(&sent, "minecraft:mule").len(), 1, "a mule");
    assert!(added(&sent, "minecraft:horse").is_empty() && added(&sent, "minecraft:donkey").is_empty());
}

/// A wandering trader summoned on `h`'s floor at `feet`: its id and its llamas'.
fn trader(h: &mut Harness, feet: [f32; 3]) -> (u64, Vec<u64>) {
    let (trader, sent) = made(h, 3, move |ctx| ctx.summon_as(None, TRADER, feet, 0.0, 0.0, None).expect("summoned").0);
    let llamas: Vec<u64> = added(&sent, LLAMA).iter().map(|llama| llama.runtime_id).collect();
    assert_eq!(llamas.len(), 2);
    (trader, llamas)
}

fn unleashed(sent: &[RawPacket], id: u64) -> Vec<i32> {
    all::<EntityEvent>(sent).into_iter().filter(|event| event.runtime_entity_id == id && event.event_id == Event::RemoveLeash).map(|event| event.data).collect()
}

#[test]
fn a_killed_traders_llamas_drop_their_leads_and_tame_themselves() {
    // bds-k2trader.txt: RemoveLeash 1 and LEASHED off, two leads with `lead.break`, then TAMED.
    let mut h = floor();
    let (trader, llamas) = trader(&mut h, [3.5, 65.0, 3.5]);
    assert!(ask(&mut h, move |ctx| ctx.kill_entity(EntityId(trader))));
    let sent = watch(&mut h, 6);
    for llama in &llamas {
        assert_eq!(unleashed(&sent, *llama), [1], "llama {llama}");
        assert_eq!((has_flag(&sent, *llama, Flags::LEASHED), has_flag(&sent, *llama, Flags::TAMED)), (Some(false), Some(true)), "llama {llama}");
    }
    assert_eq!(all::<AddItemEntity>(&sent).len(), 2, "two leads");
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "lead.break"));
}

#[test]
fn a_trader_sent_start_despawn_goes_with_its_llamas_once_no_player_is_near() {
    // bds-k2trader.txt: RemoveLeash 0 to each llama, then all three removed in that tick.
    let mut level = common::platform();
    let stone = level.block_id("minecraft:stone").unwrap();
    level.fill([24, 64, -4], [40, 64, 4], stone);
    let mut h = Harness::with(common::config(), level);
    h.enter(1, "Alex");
    let (trader, llamas) = trader(&mut h, [30.5, 65.0, 0.5]);
    assert!(ask(&mut h, move |ctx| ctx.fire_entity_event(EntityId(trader), "minecraft:start_despawn")));
    let ticks: Vec<Vec<RawPacket>> = (0..25).map(|_| h.tick_all().to(1).to_vec()).collect();
    let gone = ticks.iter().find(|sent| all::<RemoveEntity>(sent).len() == 3).expect("the three went together");
    for llama in &llamas {
        assert_eq!(unleashed(gone, *llama), [0], "llama {llama}");
    }
    assert!(ticks.iter().flat_map(|sent| all::<AddItemEntity>(sent)).next().is_none(), "no leads dropped");
}

#[test]
fn a_pillagers_crossbow_is_heard_loading_from_where_it_is_by_then() {
    // bds-k2shot.txt: each of the three stages from the pillager's place of that tick, 0.95 up.
    let mut h = floor();
    let pillager = ask(&mut h, |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        let pillager = ctx.spawn_entity("minecraft:pillager", [0.5, 65.0, 6.5]).expect("spawned");
        assert!(ctx.give_entity_effect(pillager, Effect::Slowness, 1000, 255));
        pillager.0
    });
    let mut moved = false;
    let mut heard = Vec::new();
    for _ in 0..120 {
        h.receive(1, &common::input_at([0.5, 65.0, 0.5]));
        let sent = h.tick_all().to(1).to_vec();
        if !moved && !added(&sent, "minecraft:arrow").is_empty() {
            moved = ask(&mut h, move |ctx| ctx.move_entity(EntityId(pillager), [3.5, 65.0, 6.5]));
        }
        heard.extend(all::<LevelSoundEvent>(&sent).into_iter().filter(|sound| moved && sound.sound_id.starts_with("crossbow.loading.")).map(|sound| [sound.position.x, sound.position.y, sound.position.z]));
    }
    assert!(heard.len() >= 3 && heard[..3].iter().all(|at| (at[0] - 3.5).abs() < 0.3 && (at[1] - 65.95).abs() < 1e-3), "{heard:?}");
}

/// Ticks from now until the mob `id` shows `flag` on, and the hurts it showed by freezing on the way.
fn frozen_in(h: &mut Harness, id: u64, flag: Flags, ticks: usize) -> (Option<usize>, Vec<usize>) {
    let (mut on, mut hurts) = (None, Vec::new());
    for tick in 0..ticks {
        let sent = h.tick_all().to(1).to_vec();
        if on.is_none() && has_flag(&sent, id, flag) == Some(true) {
            on = Some(tick);
        }
        if all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == id && event.event_id == Event::HurtAnimation && event.data == 27) {
            hurts.push(tick);
        }
    }
    (on, hurts)
}

/// A `kind` summoned at night into a pit of powder snow, held still; its runtime id.
fn snowed(h: &mut Harness, kind: &'static str) -> u64 {
    ask(h, move |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_game_mode(sapling_api::PlayerId(1), GameMode::Creative);
        ctx.set_time(18000);
        for [x, y, z] in (3..=5).flat_map(|x| (65..=67).flat_map(move |y| (3..=5).map(move |z| [x, y, z]))) {
            assert!(ctx.set_block([x, y, z], &Block::new("powder_snow")));
        }
        let mob = ctx.summon_as(None, kind, [4.5, 65.0, 4.5], 0.0, 0.0, None).expect("summoned");
        assert!(ctx.give_entity_effect(mob, Effect::Slowness, 1000, 255));
        mob.0
    })
}

#[test]
fn a_cow_in_powder_snow_freezes_in_seven_seconds_and_is_hurt_every_two_after() {
    // bds-k2freeze.txt: flag 40 140 ticks after its announcement, hurt 1 (freezing) 19 ticks later
    // (its age a multiple of 40), then every 40.
    let mut h = floor();
    let cow = snowed(&mut h, "minecraft:cow");
    let (on, hurts) = frozen_in(&mut h, cow, Flags::VIBRATING, 210);
    let on = on.expect("frozen");
    assert!((138..=141).contains(&on), "{on}");
    assert_eq!(hurts, [on + 19, on + 59], "{on}");
}

#[test]
fn a_polar_bear_never_freezes_and_a_skeleton_shakes_then_becomes_a_stray() {
    // bds-k2freeze.txt: the bear unflagged and unhurt; the skeleton flagged a tick after it came,
    // a stray 402 ticks later with `convert_to_stray`.
    let mut h = floor();
    let bear = snowed(&mut h, "minecraft:polar_bear");
    assert_eq!(frozen_in(&mut h, bear, Flags::VIBRATING, 200), (None, vec![]));
    let mut h = floor();
    let skeleton = snowed(&mut h, "minecraft:skeleton");
    assert!(frozen_in(&mut h, skeleton, Flags::VIBRATING, 3).0.is_some(), "shaking at once");
    let sent = watch(&mut h, 405);
    assert_eq!(added(&sent, "minecraft:stray").len(), 1, "a stray");
    assert_eq!(all::<RemoveEntity>(&sent).len(), 1, "the skeleton went");
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "convert_to_stray"));
}

const SEARCHING: i64 = 1 << 49;
const DIGGING: i64 = 1 << 42;
const RISING: i64 = 1 << 52;
const HAPPY: i64 = 1 << 53;

/// A grown sniffer summoned on the stone floor with one dirt block at `dirt`, held still if `held`.
fn sniffer(h: &mut Harness, dirt: [i32; 3], held: bool) -> u64 {
    ask(h, move |ctx| {
        assert!(ctx.set_block(dirt, &Block::new("dirt")));
        let sniffer = ctx.summon_as(None, "minecraft:sniffer", [0.5, 65.0, 0.5], 0.0, 0.0, Some("minecraft:spawn_adult")).expect("summoned");
        assert!(!held || ctx.give_entity_effect(sniffer, Effect::Slowness, 2000, 255));
        sniffer.0
    })
}

/// The sniffer `id` sent `on_scenting_success`: its extended flags each of the next `ticks` ticks.
fn scented(h: &mut Harness, id: u64, ticks: usize) -> Vec<i64> {
    assert!(ask(h, move |ctx| ctx.fire_entity_event(EntityId(id), "on_scenting_success")));
    let mut word = 0;
    (0..ticks)
        .map(|_| {
            let sent = h.tick_all().to(1).to_vec();
            let told = all::<SetEntityData>(&sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.metadata);
            word = told.filter_map(|item| if let MetaValue::FlagsExtended(flags) = item.value { Some(flags.0) } else { None }).last().unwrap_or(word);
            word
        })
        .collect()
}

fn first(words: &[i64], bit: i64) -> Option<usize> {
    words.iter().position(|word| word & bit != 0)
}

#[test]
fn a_scented_sniffer_finds_the_one_dirt_block_and_digs_it_again_when_sent_again() {
    // bds-k2sniff.txt: searching 14 to 62 ticks after each event, each time dug at the same block.
    let mut h = floor();
    let id = sniffer(&mut h, [5, 64, 3], false);
    watch(&mut h, 2);
    for round in 0..2 {
        let words = scented(&mut h, id, 700);
        let (search, dig) = (first(&words, SEARCHING), first(&words, DIGGING));
        assert!(search.is_some_and(|search| search < 200) && dig > search, "round {round}: {search:?} {dig:?}");
        assert!(first(&words, HAPPY).is_some(), "round {round}: dug up something");
        assert_eq!(words.last().map(|word| word & (SEARCHING | DIGGING | RISING)), Some(0), "round {round}: done");
    }
}

#[test]
fn a_scented_sniffer_held_still_gives_up_its_search_after_seven_seconds_unhappy() {
    // bds-k2sniff.txt: searching 141 ticks, then standing up (timer_flag_2) without timer_flag_3.
    let mut h = floor();
    let id = sniffer(&mut h, [6, 64, -5], true);
    watch(&mut h, 2);
    let words = scented(&mut h, id, 400);
    let search = first(&words, SEARCHING).expect("searched");
    let gave_up = words[search..].iter().position(|word| word & SEARCHING == 0).expect("gave up");
    assert!((139..=142).contains(&gave_up) && words[search + gave_up] & RISING != 0, "{gave_up}");
    assert!(first(&words, DIGGING | HAPPY).is_none());
}
