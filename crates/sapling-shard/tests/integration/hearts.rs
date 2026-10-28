//! Creaking hearts and their creakings (creatures/creaking/heart.rs), against
//! research/captures/bds-heart.txt.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, LevelEvent, LevelEventEvent, LevelSoundEvent, RemoveEntity, SetEntityData, UpdateAttributes};
use common::asking::ask;
use common::{Harness, all, attack, block_changes, input_at};
use sapling_api::{Block, Difficulty, GameMode, PlayerId, RuleValue};

const HEART: [i32; 3] = [6, 66, 0];

fn heart_state(h: &mut Harness) -> String {
    let state = ask(h, |ctx| ctx.block(HEART).map(|block| block.properties).unwrap_or_default());
    state.split(',').find_map(|pair| pair.strip_prefix("creaking_heart_state=")).unwrap_or("").to_owned()
}

/// A natural heart between two pale oak logs by Alex at `time`; the harness after the top log's tick.
fn heart_at(time: i64) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(time);
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        assert!(ctx.set_game_rule("doMobSpawning", RuleValue::Bool(true)));
        assert!(ctx.update_block([6, 65, 0], &Block::new("pale_oak_log")));
        assert!(ctx.update_block(HEART, &Block::new("creaking_heart").with("creaking_heart_state=uprooted,natural=1,pillar_axis=y")));
    });
    h.tick_all();
    assert_eq!(heart_state(&mut h), "uprooted", "a log below only");
    h.remote.run(|ctx| assert!(ctx.update_block([6, 67, 0], &Block::new("pale_oak_log"))));
    h
}

fn properties(sent: &[RawPacket], id: u64) -> Vec<(u32, i32)> {
    all::<SetEntityData>(sent).into_iter().filter(|data| data.runtime_entity_id == id).flat_map(|data| data.properties.ints).map(|int| (int.index, int.value)).collect()
}

#[test]
fn an_awake_heart_calls_a_creaking_that_sways_off_hits_and_crumbles_when_the_heart_breaks() {
    let mut h = heart_at(18000);
    let ticks: Vec<Vec<RawPacket>> = (0..200).map(|_| h.tick_all().to(1).to_vec()).collect();
    assert_eq!(heart_state(&mut h), "awake");
    let woke = ticks.iter().position(|sent| block_changes(sent).iter().any(|(at, _)| *at == HEART)).expect("the heart wakes");
    let spawn_particle = |sent: &Vec<RawPacket>| all::<LevelEvent>(sent).iter().any(|event| event.event == LevelEventEvent::ParticleSpawn && event.data == 1 + (3 << 8));
    let called = ticks.iter().position(spawn_particle).expect("a creaking called");
    assert!(called >= woke + 18 && (called - woke - 18).is_multiple_of(20), "18 ticks after it woke, then every 20: {woke} {called}");
    let creaking = ticks[called..].iter().take(2).flat_map(|sent| all::<AddEntity>(sent)).find(|added| added.entity_type == "minecraft:creaking").expect("shown");
    let heard = ticks[called..].iter().take(3).flat_map(|sent| all::<LevelSoundEvent>(sent)).filter(|sound| sound.sound_id == "creaking_heart_spawn").count();
    assert_eq!(heard, 2, "at the creaking and at the heart");

    let at = creaking.position;
    h.receive(1, &input_at([at.x + 1.5, at.y, at.z]));
    h.tick_all();
    h.receive(1, &attack(creaking.runtime_id));
    let swayed: Vec<RawPacket> = (0..9).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let health = all::<UpdateAttributes>(&swayed).iter().filter(|update| update.runtime_entity_id == creaking.runtime_id).count();
    assert_eq!((properties(&swayed, creaking.runtime_id), health), ((1..=6).chain([0]).map(|value| (1, value)).collect(), 0), "swayed a tick at a time, unharmed");

    h.remote.run(|ctx| assert!(ctx.update_block(HEART, &Block::air())));
    let crumbling: Vec<Vec<RawPacket>> = (0..52).map(|_| h.tick_all().to(1).to_vec()).collect();
    let state = |index: usize| properties(&crumbling[index], creaking.runtime_id);
    let started = (0..crumbling.len()).find(|index| state(*index).contains(&(0, 3))).expect("it crumbles");
    let event = crumbling.iter().position(|sent| all::<LevelEvent>(sent).iter().any(|event| event.event == LevelEventEvent::Unknown(16478))).expect("crumble particles");
    let died = crumbling.iter().position(|sent| all::<LevelSoundEvent>(sent).iter().any(|sound| sound.sound_id == "death")).expect("death heard");
    let gone = crumbling.iter().position(|sent| !all::<RemoveEntity>(sent).is_empty()).expect("gone");
    assert_eq!((event - started, died - started, gone - started), (47, 48, 49));
    assert!(state(died).contains(&(0, 4)));
}

#[test]
fn a_heart_sleeps_by_day_and_calls_nothing() {
    let mut h = heart_at(6000);
    h.tick_all();
    assert_eq!(heart_state(&mut h), "dormant");
    let sent: Vec<RawPacket> = (0..60).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    assert!(all::<AddEntity>(&sent).iter().all(|added| added.entity_type != "minecraft:creaking"));
}
