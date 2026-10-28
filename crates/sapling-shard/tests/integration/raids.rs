//! Raids: bad omen in a village, the raid omen, BDS's waves on easy, the boss bar and the
//! victory's Hero of the Village (docs/research/effects-raids.md).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::Value;
use acacia_proto::packets::{AddEntity, BossEvent, BossEventType, LevelSoundEvent, MobArmorEquipment, MobEffect, MobEffectEventId};
use common::{Harness, all, config};
use sapling_api::{Block, BlockEntityData, Difficulty, Effect, EntityInfo, PlayerId};
use sapling_world::World;

const ALEX: PlayerId = PlayerId(1);
const BELL: [i32; 3] = [3, 65, 3];
const RAIDERS: [&str; 5] = ["minecraft:pillager", "minecraft:vindicator", "minecraft:ravager", "minecraft:witch", "minecraft:evocation_illager"];

/// Stone wide enough for a wave to come 32 out, a bell and a villager by it, and Alex on easy.
fn village() -> Harness {
    let mut level = World::void();
    let stone = level.block_id("minecraft:stone").unwrap();
    level.fill([-48, 64, -48], [47, 64, 47], stone);
    let mut h = Harness::with(config(), level);
    h.enter(1, "Alex");
    let [x, y, z] = BELL;
    let entity = Value::Compound(vec![("id".into(), Value::String("Bell".into())), ("x".into(), Value::Int(x)), ("y".into(), Value::Int(y)), ("z".into(), Value::Int(z))]);
    let done = common::asking::ask(&mut h, move |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        (ctx.set_block(BELL, &Block::new("bell")), ctx.set_block_entity(BELL, &BlockEntityData::new(entity)), ctx.spawn_entity("minecraft:villager_v2", [5.5, 65.0, 5.5]).is_some())
    });
    assert_eq!(done, (true, true, true));
    h
}

fn ticks(h: &mut Harness, n: usize) -> Vec<RawPacket> {
    (0..n).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

fn effects(sent: &[RawPacket]) -> Vec<(MobEffectEventId, i32, i32, i32)> {
    all::<MobEffect>(sent).into_iter().filter(|effect| effect.runtime_entity_id == 1).map(|effect| (effect.event_id, effect.effect_id, effect.amplifier, effect.duration)).collect()
}

fn bars(sent: &[RawPacket]) -> Vec<(BossEventType, String)> {
    all::<BossEvent>(sent).into_iter().map(|event| (event.r#type, event.title)).collect()
}

fn raiders(h: &mut Harness) -> Vec<EntityInfo> {
    common::asking::ask(h, |ctx| ctx.entities().into_iter().filter(|entity| RAIDERS.contains(&entity.kind.as_str())).collect())
}

#[test]
fn bad_omen_in_a_village_brings_a_raid_whose_three_waves_on_easy_won_make_a_hero() {
    let mut h = village();
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::BadOmen, 6000, 1, true)));
    let told: Vec<(MobEffectEventId, i32, i32)> = effects(&ticks(&mut h, 2)).into_iter().map(|(event, id, amplifier, _)| (event, id, amplifier)).collect();
    assert_eq!(told, [(MobEffectEventId::Add, 28, 1), (MobEffectEventId::Remove, 28, 1), (MobEffectEventId::Add, 36, 1)], "the player pack's gain_raid_omen");
    let sent = ticks(&mut h, 600);
    assert_eq!(bars(&sent).first(), Some(&(BossEventType::ShowBar, "%raid.name".to_owned())), "the omen ran out: the raid starts");
    let (mut waves, mut after) = (Vec::new(), Vec::new());
    for _ in 0..3 {
        assert!((0..400).any(|_| !raiders(&mut h).is_empty()), "a wave within 300 ticks");
        let wave = raiders(&mut h);
        let mut kinds: Vec<String> = wave.iter().map(|raider| raider.kind.clone()).collect();
        kinds.sort();
        waves.push(kinds);
        h.remote.run(move |ctx| wave.iter().for_each(|raider| assert!(ctx.kill_entity(raider.id))));
        after = ticks(&mut h, 25);
    }
    let named = |kind: &str, n: usize| vec![format!("minecraft:{kind}"); n];
    assert_eq!(waves, [named("pillager", 4), [named("pillager", 3), named("vindicator", 2)].concat(), [named("pillager", 3), named("ravager", 1)].concat()]);
    assert!(effects(&after).contains(&(MobEffectEventId::Add, 29, 1, 48000)), "{:?}", effects(&after));
    assert!(bars(&after).contains(&(BossEventType::SetBarTitle, "%raid.name - %raid.victory".to_owned())), "{:?}", bars(&after));
    assert_eq!(bars(&ticks(&mut h, 600)), [(BossEventType::HideBar, String::new())], "30 s on");
}

#[test]
fn raiders_make_for_the_village_kill_its_villager_and_celebrate_its_fall() {
    let mut h = village();
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::BadOmen, 6000, 0, true)));
    ticks(&mut h, 602);
    assert!((0..400).any(|_| !raiders(&mut h).is_empty()), "the first wave");
    // Out of the fight: the omen's player is creative only once its raid has begun.
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, sapling_api::GameMode::Creative));
    let off = |h: &mut Harness| raiders(h).iter().map(|raider| (raider.feet[0] - 3.5).hypot(raider.feet[2] - 3.5)).fold(f32::MAX, f32::min);
    let came = off(&mut h);
    ticks(&mut h, 100);
    assert!(off(&mut h) < came - 3.0, "they walk at the bell: {came} then {}", off(&mut h));
    let mut sent = Vec::new();
    for _ in 0..40 {
        sent.extend(ticks(&mut h, 50));
        if bars(&sent).contains(&(BossEventType::SetBarTitle, "%raid.name - %raid.defeat".to_owned())) {
            break;
        }
    }
    assert!(bars(&sent).contains(&(BossEventType::SetBarTitle, "%raid.name - %raid.defeat".to_owned())), "the pillagers shot the villager");
    let cries = all::<LevelSoundEvent>(&ticks(&mut h, 300)).into_iter().filter(|sound| sound.sound_id == "celebrate").count();
    assert!(cries >= 2, "the raiders celebrate: {cries}");
}

#[test]
fn a_captain_wears_the_ominous_banner_and_drops_it() {
    let mut h = village();
    h.remote.run(|ctx| assert!(ctx.summon_as(None, "minecraft:pillager", [20.5, 65.0, 20.5], 0.0, Some("minecraft:spawn_as_illager_captain")).is_some()));
    let sent = ticks(&mut h, 3);
    let captain = all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == "minecraft:pillager").expect("shown").runtime_id;
    let worn: Vec<i16> = all::<MobArmorEquipment>(&sent).into_iter().filter(|worn| worn.runtime_entity_id == captain).map(|worn| worn.chestplate.network_id).collect();
    assert!(worn.iter().any(|id| *id != 0), "its banner shows in its chest slot: {worn:?}");
    h.remote.run(move |ctx| assert!(ctx.kill_entity(sapling_api::EntityId(captain))));
    ticks(&mut h, 30);
    let dropped: Vec<(String, i16)> = common::asking::ask(&mut h, |ctx| ctx.entities().into_iter().filter_map(|entity| entity.item).map(|item| (item.name, item.damage)).collect());
    assert!(dropped.contains(&("minecraft:banner".to_owned(), 15)), "{dropped:?}");
    assert!(!dropped.iter().any(|(name, _)| name == "minecraft:ominous_bottle"), "no player killed it: no bottle");
}

#[test]
fn a_raid_is_kept_with_the_world_and_goes_on_after_a_restart() {
    let storage = common::kept::Kept::new(common::stored::floor_holding(Vec::new(), &[([3, 64, 3], "minecraft:bell")]));
    let mut h = common::kept::opened(&storage);
    common::stored::enter_stored(&mut h, 1);
    let entity = Value::Compound(vec![("id".into(), Value::String("Bell".into())), ("x".into(), Value::Int(3)), ("y".into(), Value::Int(64)), ("z".into(), Value::Int(3))]);
    h.remote.run(move |ctx| {
        ctx.set_difficulty(Difficulty::Normal);
        assert!(ctx.set_block_entity([3, 64, 3], &BlockEntityData::new(entity)));
        assert!(ctx.spawn_entity("minecraft:villager_v2", [5.5, 64.0, 5.5]).is_some());
        assert!(ctx.give_effect(ALEX, Effect::BadOmen, 6000, 0, true));
    });
    ticks(&mut h, 640);
    h.shard.save_worlds();
    let level = storage.level().expect("flushed");
    let [raid] = &level.raids[..] else { panic!("{} raids kept", level.raids.len()) };
    assert!(raid.village.ends_with("-4000-8000-000000000003"), "{}", raid.village);
    drop(h);

    let mut h = common::kept::opened(&storage);
    let mut sent = common::stored::enter_stored(&mut h, 1);
    sent.extend(ticks(&mut h, 25));
    let shown = bars(&sent).into_iter().find(|(event, _)| *event == BossEventType::ShowBar);
    assert_eq!(shown, Some((BossEventType::ShowBar, "%raid.name".to_owned())), "the kept raid goes on");
}

#[test]
fn bad_omen_where_no_villager_lives_stays() {
    let mut h = village();
    h.remote.run(|ctx| {
        let villager = ctx.entities().into_iter().find(|entity| entity.kind == "minecraft:villager_v2").unwrap();
        assert!(ctx.kill_entity(villager.id));
    });
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::BadOmen, 6000, 0, true)));
    assert_eq!(effects(&ticks(&mut h, 40)), [(MobEffectEventId::Add, 28, 0, 6000)], "a bell alone is no village");
}
