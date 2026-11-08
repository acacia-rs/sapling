//! Group J of the fourteenth wave, mob knockback and more mob lines (research/captures/bds-j3knock.txt,
//! bds-j3omen.txt, bds-j3creak.txt, bds-j3sheep.txt; creatures/{hunting/prey,omens,creaking/trail}.rs,
//! husbandry/{fired,products}.rs).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, EntityEventEventId as Event, LevelEvent, LevelEventEvent, LevelSoundEvent, SetEntityMotion};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::asking::ask;
use common::raising::showed;
use common::shooting::{spawn, watch};
use common::{Harness, all};
use sapling_api::{Block, Difficulty, Effect, EntityId, GameMode, PlayerId, RuleValue};

const HEART: [i32; 3] = [6, 66, 0];

fn motions(sent: &[RawPacket], id: u64) -> Vec<[f32; 3]> {
    all::<SetEntityMotion>(sent).into_iter().filter(|told| told.runtime_entity_id == id).map(|told| [told.velocity.x, told.velocity.y, told.velocity.z]).collect()
}

/// Alex in creative at midnight on easy.
fn night() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    ask(&mut h, |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
    });
    h
}

#[test]
fn an_iron_golems_hit_throws_a_zombie_0_52_across_and_0_82_up_told_in_the_hit_tick() {
    let mut h = night();
    spawn(&mut h, "minecraft:iron_golem", [0.5, 65.0, 0.5]);
    let zombie = spawn(&mut h, "minecraft:zombie", [3.5, 65.0, 0.5]);
    let ticks: Vec<Vec<RawPacket>> = (0..200).map(|_| watch(&mut h, 1)).collect();
    let hit = ticks.iter().position(|sent| motions(sent, zombie).iter().any(|[_, up, _]| *up > 0.8)).expect("the golem threw it");
    let [x, up, z] = motions(&ticks[hit], zombie).into_iter().find(|[_, up, _]| *up > 0.8).unwrap();
    assert!((x.hypot(z) - 0.52).abs() < 1e-3 && (up - 0.820_099_06).abs() < 1e-5, "{x} {up} {z}");
    assert!(showed(&ticks[hit], zombie, Event::HurtAnimation), "told in the tick it was hurt");
    // The golem ticks first, yet the zombie moves only a tick on (bds-j3knock.txt, sapling-j3slime.txt).
    assert_eq!(motions(&ticks[hit], zombie).len(), 1, "told once in the hit's tick");
    let [_, moved, _] = *motions(&ticks[hit + 1], zombie).first().expect("moved a tick on");
    assert!((moved - (0.820_099_06 - 0.08) * 0.98).abs() < 1e-4, "{moved}");
}

#[test]
fn a_wind_charged_burst_pushes_a_cow_by_1_less_its_distance_over_7_5() {
    let mut h = night();
    let cow = spawn(&mut h, "minecraft:cow", [7.5, 65.0, 0.5]);
    let zombie = ask(&mut h, |ctx| {
        let id = ctx.spawn_entity("minecraft:zombie", [4.5, 65.0, 0.5]).expect("a zombie");
        assert!(ctx.give_entity_effect(id, Effect::WindCharged, 1200, 0));
        id
    });
    watch(&mut h, 2);
    h.remote.run(move |ctx| assert!(ctx.kill_entity(zombie)));
    let ticks: Vec<Vec<RawPacket>> = (0..3).map(|_| watch(&mut h, 1)).collect();
    let burst = ticks.iter().position(|sent| all::<LevelEvent>(sent).iter().any(|event| event.event == LevelEventEvent::ParticleWindExplosion)).expect("a burst");
    let [x, _, z] = *motions(&ticks[burst], cow).first().expect("the cow told at once");
    // 3 across and 0.975 down from the zombie's middle: 1 - 3.1545 / 7.5, nearly all of it across.
    assert!((x - 0.578).abs() < 0.003 && z.abs() < 1e-4, "{x} {z}");
}

#[test]
fn sheep_spawned_in_plains_come_in_their_temperate_colours() {
    let mut h = night();
    h.remote.run(|ctx| {
        for at in 0..64 {
            assert!(ctx.spawn_entity("minecraft:sheep", [(at % 8) as f32 * 2.0 - 7.5, 65.0, (at / 8) as f32 * 2.0 - 7.5]).is_some());
        }
    });
    let added: Vec<AddEntity> = (0..3).flat_map(|_| all::<AddEntity>(h.tick_all().to(1))).filter(|added| added.entity_type == "minecraft:sheep").collect();
    let colours: Vec<i8> = added.iter().map(|added| added.metadata.iter().find(|item| item.key == Key::Color).and_then(|item| if let Value::Default(Plain::Byte(colour)) = item.value { Some(colour) } else { None }).unwrap_or(-1)).collect();
    assert_eq!(colours.len(), 64);
    assert!(colours.iter().all(|colour| [0, 15, 8, 7, 12, 6].contains(colour)) && colours.iter().any(|colour| *colour != 0), "{colours:?}");
}

#[test]
fn a_lamb_eats_the_grass_under_it_36_ticks_after_its_animation() {
    let mut h = night();
    ask(&mut h, |ctx| {
        for x in -4..=4 {
            for z in -4..=4 {
                assert!(ctx.set_block([x, 64, z], &Block::new("grass_block")));
            }
        }
    });
    let lamb = ask(&mut h, |ctx| ctx.summon_as(None, "minecraft:sheep", [0.5, 65.0, 0.5], 0.0, 0.0, Some("minecraft:spawn_baby")).expect("a lamb")).0;
    let ticks: Vec<Vec<RawPacket>> = (0..600).map(|_| watch(&mut h, 1)).collect();
    let ate = ticks.iter().position(|sent| showed(sent, lamb, Event::EatGrassAnimation)).expect("it grazed");
    let turned = ticks.iter().position(|sent| common::block_changes(sent).iter().any(|(at, _)| at[1] == 64)).expect("the grass went");
    assert_eq!(turned, ate + 36);
}

/// A natural heart between pale oak logs, awake, and its creaking: the harness and the creaking's id.
fn creaking_by_its_heart() -> (Harness, u64, [f32; 3]) {
    let mut h = night();
    h.remote.run(|ctx| {
        assert!(ctx.set_game_rule("doMobSpawning", RuleValue::Bool(true)));
        assert!(ctx.update_block([6, 65, 0], &Block::new("pale_oak_log")));
        assert!(ctx.update_block([6, 67, 0], &Block::new("pale_oak_log")));
        assert!(ctx.update_block(HEART, &Block::new("creaking_heart").with("creaking_heart_state=uprooted,natural=1,pillar_axis=y")));
    });
    let creaking = (0..200).flat_map(|_| all::<AddEntity>(h.tick_all().to(1))).find(|added| added.entity_type == "minecraft:creaking").expect("a creaking called");
    let at = creaking.position;
    (h, creaking.runtime_id, [at.x, at.y, at.z])
}

#[test]
fn a_creakings_shrugged_off_hit_grows_resin_and_sounds_a_trail_to_its_heart() {
    let (mut h, creaking, at) = creaking_by_its_heart();
    h.receive(1, &common::input_at([at[0] + 1.5, at[1], at[2]]));
    h.tick_all();
    h.receive(1, &common::attack_from(creaking, [at[0] + 1.5, at[1], at[2]]));
    let ticks: Vec<Vec<RawPacket>> = (0..110).map(|_| watch(&mut h, 1)).collect();
    let trail: Vec<(usize, i32)> = ticks.iter().enumerate().flat_map(|(tick, sent)| all::<LevelSoundEvent>(sent).into_iter().filter(|sound| sound.sound_id == "block.creaking_heart.trail").map(move |sound| (tick, sound.extra_data))).collect();
    let first = trail.first().expect("a trail").0;
    assert_eq!(trail, [-600, -440, -279, -120, 39, 200, 360, 520, 679, 840].iter().enumerate().map(|(step, data)| (first + 10 * step, *data)).collect::<Vec<_>>());
    let placed = all::<LevelSoundEvent>(&ticks[first - 1]).iter().filter(|sound| sound.sound_id == "place").count();
    assert!((2..=3).contains(&placed), "{placed} clumps a tick before the trail");
    let clumps = ask(&mut h, |ctx| {
        let around = (4..=8).flat_map(|x| (64..=68).flat_map(move |y| (-2..=2).map(move |z| [x, y, z])));
        around.filter(|cell| ctx.block(*cell).is_some_and(|block| block.name == "minecraft:resin_clump")).count()
    });
    assert!((2..=3).contains(&clumps), "{clumps} clumps by the logs");
}

#[test]
fn a_creaking_moved_36_from_its_heart_is_torn_down_at_once() {
    let (mut h, creaking, at) = creaking_by_its_heart();
    // Out of Alex's sight there: asked of, not watched.
    assert!(ask(&mut h, move |ctx| ctx.move_entity(EntityId(creaking), [HEART[0] as f32 + 36.5, at[1], HEART[2] as f32 + 0.5])));
    let gone = (1..10).find(|_| ask(&mut h, move |ctx| ctx.entity(EntityId(creaking)).is_none()));
    let gone = gone.unwrap_or_else(|| panic!("not torn down: {:?}", ask(&mut h, move |ctx| ctx.entity(EntityId(creaking)).map(|info| info.feet))));
    // BDS's went 3 ticks after its /tp; a remote move lands after the hearts' tick, so one more here.
    assert_eq!(gone, 4, "no 47 ticks of crumbling");
}
