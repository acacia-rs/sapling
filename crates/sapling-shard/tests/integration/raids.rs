//! Raids: bad omen in a village, the raid omen, BDS's waves on easy, the boss bar and the
//! victory's Hero of the Village (docs/research/effects-raids.md).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::nbt::Value;
use acacia_proto::packets::{BossEvent, BossEventType, MobEffect, MobEffectEventId};
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
