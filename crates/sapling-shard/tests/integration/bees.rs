//! Bees and their hives, against research/captures/bds-keephive.txt and bds-bees.txt: a bee
//! shelters by night and comes out by day, takes pollen from a flower home and makes honey, and a
//! broken hive's bees go for the nearest player.

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, EntityEvent, EntityEventEventId, LevelSoundEvent, MobEffect, SetEntityData};
use acacia_proto::types::{Action, MetadataDictionaryItemKey as Key,MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::{Harness, all, block_actions};
use sapling_api::{Block, Difficulty, EntityId, GameMode, PlayerId};

const HIVE: [i32; 3] = [3, 65, 0];
const BEE: &str = "minecraft:bee";

/// Alex in creative and Steve in survival by a beehive facing east at `time`, a bee summoned at
/// `bee`; the bee's id.
fn by_a_hive(time: i64, bee: [f32; 3]) -> (Harness, u64) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let id = Arc::new(Mutex::new(0));
    let kept = id.clone();
    h.remote.run(move |ctx| {
        ctx.set_time(time);
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        // A set hive would make bees of its own (`a_set_nest_makes_two_or_three_bees_...`).
        assert!(ctx.set_game_rule("doMobSpawning", sapling_api::RuleValue::Bool(false)));
        assert!(ctx.set_block(HIVE, &Block { name: "minecraft:beehive".into(), properties: "direction=3,honey_level=0".into() }));
        *kept.lock().unwrap() = ctx.spawn_entity(BEE, bee).expect("a bee").0;
    });
    h.tick_all();
    let id = *id.lock().unwrap();
    (h, id)
}

#[test]
fn a_set_nest_makes_two_or_three_bees_at_once_by_night_that_come_out_100_ticks_on_by_day() {
    // research/captures/bds-f14snest.txt (noon, 2 or 3 out of each hive ~100 ticks after, heard `spawn`), bds-f14snest2.txt
    // (midnight: three made inside at once, kept as `SaveData` {}, `ShouldSpawnBees` 0).
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_time(18_000);
        assert!(ctx.set_block(HIVE, &Block { name: "minecraft:bee_nest".into(), properties: "direction=3,honey_level=0".into() }));
    });
    (0..3).for_each(|_| drop(h.tick_all()));
    let inside = read(&mut h, |ctx| ctx.block_entity(HIVE).and_then(|entity| entity.get::<acacia_proto::nbt::Value>().cloned()));
    let inside = inside.expect("the nest's entity");
    let acacia_proto::nbt::Value::List(bees) = inside.get("Occupants").expect("bees inside") else { panic!("a list") };
    assert!((2..=3).contains(&bees.items.len()), "{} made", bees.items.len());
    assert_eq!(inside.get("ShouldSpawnBees"), Some(&acacia_proto::nbt::Value::Byte(0)));
    assert!(bees.items.iter().all(|bee| bee.get("SaveData") == Some(&acacia_proto::nbt::Value::Compound(Vec::new()))));
    h.remote.run(|ctx| ctx.set_time(6_000));
    // Each comes out at 1 in 20 a tick once its 100 are up (bds-f14snest.txt: 3, 4 and 16 ticks after).
    let (_, sent) = until(&mut h, 400, |sent| bee_added(sent).len() == bees.items.len());
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "spawn" && sound.entity_type == BEE), "heard as new bees");
}

#[test]
fn a_placed_trees_nest_makes_its_bees_too() {
    // research/captures/bds-f14snest.txt: each `/place feature` tree's nest let out 2 or 3 bees.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_time(6_000);
        assert!(ctx.set_block([-8, 64, -8], &Block { name: "minecraft:grass_block".into(), properties: String::new() }));
        ctx.place_feature("minecraft:birch_tree_with_beehive_feature", [-8, 65, -8]).expect("a birch");
    });
    let (_, sent) = until(&mut h, 400, |sent| bee_added(sent).len() >= 2);
    assert!(sounds(&sent).contains(&"block.beehive.exit".to_owned()));
}

fn sounds(sent: &[RawPacket]) -> Vec<String> {
    all::<LevelSoundEvent>(sent).into_iter().map(|sound| sound.sound_id).filter(|name| name.starts_with("block.beehive") && name != "block.beehive.work").collect()
}

/// Ticks `most` times or until `done` holds of what Alex was sent; the ticks it took and what was sent.
fn until(h: &mut Harness, most: usize, done: impl Fn(&[RawPacket]) -> bool) -> (usize, Vec<RawPacket>) {
    let mut sent = Vec::new();
    for tick in 1..=most {
        sent.extend(h.tick_all().to(1).to_vec());
        if done(&sent) {
            return (tick, sent);
        }
    }
    panic!("not within {most} ticks");
}

/// What `read` finds through the context, a tick on.
fn read<T: Default + Clone + Send + 'static>(h: &mut Harness, read: impl FnOnce(&mut sapling_api::ShardCtx<'_>) -> T + Send + 'static) -> T {
    let out = Arc::new(Mutex::new(T::default()));
    let kept = out.clone();
    h.remote.run(move |ctx| *kept.lock().unwrap() = read(ctx));
    h.tick_all();
    let found = out.lock().unwrap().clone();
    found
}

fn honey(h: &mut Harness) -> String {
    let properties = read(h, |ctx| ctx.block(HIVE).map(|block| block.properties).unwrap_or_default());
    properties.split(',').find_map(|pair| pair.strip_prefix("honey_level=")).unwrap_or("?").to_owned()
}

fn gone(h: &mut Harness, id: u64) -> bool {
    read(h, move |ctx| ctx.entity(EntityId(id)).is_none())
}

fn bee_added(sent: &[RawPacket]) -> Vec<AddEntity> {
    all::<AddEntity>(sent).into_iter().filter(|added| added.entity_type == BEE).collect()
}

#[test]
fn a_bee_shelters_in_a_hive_by_night_and_comes_out_in_front_by_day_600_ticks_after() {
    let (mut h, bee) = by_a_hive(18000, [0.5, 66.0, 0.5]);
    let (_, sent) = until(&mut h, 200, |sent| !sounds(sent).is_empty());
    assert_eq!(sounds(&sent), ["block.beehive.enter"]);
    assert!(gone(&mut h, bee));
    h.remote.run(|ctx| ctx.set_time(6000));
    let (ticks, sent) = until(&mut h, 700, |sent| !bee_added(sent).is_empty());
    assert!((595..=900).contains(&ticks), "{ticks}");
    let out = bee_added(&sent).remove(0);
    assert_eq!(([out.position.x, out.position.y, out.position.z], out.yaw.rem_euclid(360.0)), ([4.5, 65.5, 0.5], 270.0), "in the cell before its east face, turned east");
    assert_eq!(sounds(&sent), ["block.beehive.exit"]);
    assert_eq!(honey(&mut h), "0", "it brought no pollen");
}

#[test]
fn a_bee_takes_pollen_from_a_flower_into_its_hive_and_makes_honey() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_time(6000);
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        assert!(ctx.set_game_rule("doMobSpawning", sapling_api::RuleValue::Bool(false)));
        assert!(ctx.set_block([0, 64, 3], &Block::new("grass_block")) && ctx.set_block([0, 65, 3], &Block::new("poppy")));
        assert!(ctx.set_block(HIVE, &Block { name: "minecraft:beehive".into(), properties: "direction=3,honey_level=0".into() }));
        assert!(ctx.spawn_entity(BEE, [0.5, 66.0, 1.5]).is_some());
    });
    // 20 s at the poppy first (bds-g14bee.txt: 20.05, 20.15 and 23.3 s from bees summoned over poppies).
    let (pollinated, sent) = until(&mut h, 1200, |sent| !sounds(sent).is_empty());
    assert!(pollinated > 400, "{pollinated}");
    assert_eq!(sounds(&sent), ["block.beehive.enter"]);
    let (ticks, _) = until(&mut h, 2800, |sent| !bee_added(sent).is_empty());
    assert!((2395..=2700).contains(&ticks), "{ticks}");
    assert_eq!(honey(&mut h), "1");
}

#[test]
fn a_bee_hit_on_normal_stings_its_hitter_with_10_s_of_poison_and_dies_of_it() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let id = Arc::new(Mutex::new(0));
    let kept = id.clone();
    h.remote.run(move |ctx| {
        ctx.set_time(6000);
        ctx.set_difficulty(Difficulty::Normal);
        *kept.lock().unwrap() = ctx.spawn_entity(BEE, [0.5, 66.0, 2.0]).expect("a bee").0;
    });
    h.tick_all();
    let bee = *id.lock().unwrap();
    h.receive(1, &common::attack(bee));
    let poisoned = |sent: &[RawPacket]| all::<MobEffect>(sent).iter().any(|effect| effect.runtime_entity_id == 1 && effect.effect_id == 19 && effect.duration == 200);
    let (stung, _) = until(&mut h, 200, poisoned);
    let (perished, _) = until(&mut h, 1300, |sent| all::<EntityEvent>(sent).iter().any(|event| event.runtime_entity_id == bee && event.event_id == EntityEventEventId::DeathAnimation));
    assert!(stung > 1 && (200..=1201).contains(&perished), "stung after {stung}, dead {perished} after");
}

#[test]
fn a_hive_out_of_the_tick_region_keeps_its_bees_and_one_in_a_ticking_area_lets_them_out() {
    // bds-hivekept.txt: by day the bees of a hive 5 chunks off stayed in for 45 s, and some s after the
    // bots came back; those of a hive in a ticking area 6 chunks off came out.
    for kept in [false, true] {
        let (mut h, _) = by_a_hive(18000, [0.5, 66.0, 0.5]);
        until(&mut h, 200, |sent| !sounds(sent).is_empty());
        h.remote.run(move |ctx| {
            ctx.force_chunk([0, 0], kept);
            ctx.set_time(6000);
            for player in [PlayerId(1), PlayerId(2)] {
                ctx.teleport(player, [96.5, 65.0, 0.5]);
            }
        });
        (0..700).for_each(|_| drop(h.tick_all()));
        let bees = read(&mut h, |ctx| ctx.entities().iter().filter(|entity| entity.kind == BEE).count());
        assert_eq!(bees, usize::from(kept), "kept {kept}");
    }
}

#[test]
fn a_broken_hives_bees_come_out_and_go_for_the_nearest_survival_player() {
    let (mut h, _) = by_a_hive(18000, [0.5, 66.0, 0.5]);
    until(&mut h, 200, |sent| !sounds(sent).is_empty());
    h.receive(1, &block_actions(&[(Action::PredictBreak, HIVE)]));
    let sent = h.tick_all().to(1).to_vec();
    let out = bee_added(&sent).remove(0);
    assert_eq!([out.position.x, out.position.y, out.position.z], [3.5, 65.5, 0.5], "out of the hive's middle");
    let bee = out.runtime_id;
    let target = |data: SetEntityData| (data.runtime_entity_id == bee).then(|| data.metadata.into_iter().find(|item| item.key == Key::TargetEid)).flatten();
    let told: Vec<_> = sent.into_iter().chain((0..3).flat_map(|_| h.tick_all().to(1).to_vec())).collect();
    let targets: Vec<Value> = common::data::changes(&told).into_iter().filter_map(target).map(|item| item.value).collect();
    assert_eq!(targets, [Value::Default(Plain::Long(2))], "Steve, in survival; Alex is in creative");
}
