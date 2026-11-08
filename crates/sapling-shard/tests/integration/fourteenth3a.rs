//! Group A of the fourteenth wave, mob combat (research/captures/bds-a3hurt.txt, bds-a3fx.txt,
//! bds-a3hunt.txt, bds-provoke2.txt; creatures/hunting/{hurt_by,rear}.rs, creatures/afflictions/moved.rs).

use crate::common;

use acacia_proto::packets::{EntityEvent, EntityEventEventId, MoveEntityDelta, SetEntityData, SetEntityMotion, UpdateAttributes};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, MetadataFlags1 as Flags};
use common::shooting::{ready, spawn, watch};
use common::{all, asking::ask, attack};
use sapling_api::{Difficulty, Effect, EntityId, GameMode, Item, Slot};

const ALEX: sapling_api::PlayerId = sapling_api::PlayerId(1);

/// Each `TargetEid` the mob `id` is told to take in `sent`.
fn targets(sent: &[acacia_proto::RawPacket], id: u64) -> Vec<i64> {
    let target = |data: &SetEntityData| data.metadata.iter().find(|item| item.key == Key::TargetEid).and_then(|item| if let Value::Default(Plain::Long(target)) = item.value { Some(target) } else { None });
    all::<SetEntityData>(sent).iter().filter(|data| data.runtime_entity_id == id).filter_map(target).collect()
}

/// [`ready`] at midnight on `difficulty`, so that monsters stay and do not burn, Steve creative so
/// that none hunts him.
fn ready_at_night(mode: GameMode, difficulty: Difficulty) -> common::Harness {
    let mut h = ready(mode, &[], 0.0);
    h.remote.run(move |ctx| {
        ctx.set_difficulty(difficulty);
        ctx.set_time(18000);
        ctx.set_game_mode(common::shooting::STEVE, GameMode::Creative);
    });
    h
}

fn hold(h: &mut common::Harness, ids: &[u64]) {
    let ids = ids.to_vec();
    assert!(ask(h, move |ctx| ids.iter().all(|id| ctx.give_entity_effect(EntityId(*id), Effect::Slowness, 10_000, 255))));
}

#[test]
fn a_skeleton_a_snow_golems_snowball_hit_takes_the_golem_the_tick_after() {
    let mut h = ready_at_night(GameMode::Creative, Difficulty::Easy);
    let golem = spawn(&mut h, "minecraft:snow_golem", [0.5, 65.0, 6.5]);
    let skeleton = spawn(&mut h, "minecraft:skeleton", [4.5, 65.0, 6.5]);
    hold(&mut h, &[golem, skeleton]);
    let mut hit_at = None;
    for tick in 0..120 {
        let sent = h.tick_all().to(1).to_vec();
        let flinched = all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == skeleton && event.event_id == EntityEventEventId::HurtAnimation);
        if flinched && hit_at.is_none() {
            hit_at = Some(tick);
            assert!(targets(&sent, skeleton).is_empty(), "not in the tick it was hit");
            continue;
        }
        if hit_at.is_some() {
            assert_eq!(targets(&sent, skeleton), [golem as i64], "the tick after the hit");
            return;
        }
    }
    panic!("no snowball hit the skeleton");
}

#[test]
fn a_players_iron_golem_takes_no_player_that_hits_it_and_a_plain_one_does() {
    for (event, takes) in [(Some("minecraft:from_player"), false), (None, true)] {
        let mut h = ready_at_night(GameMode::Survival, Difficulty::Easy);
        h.remote.run(move |ctx| assert!(ctx.summon_as(None, "minecraft:iron_golem", [0.5, 65.0, 2.0], 0.0, 0.0, event).is_some()));
        let golem = (0..5).flat_map(|_| common::shooting::added(h.tick_all().to(1), "minecraft:iron_golem")).next().expect("the golem").runtime_id;
        hold(&mut h, &[golem]);
        h.receive(1, &attack(golem));
        let sent = watch(&mut h, 10);
        assert_eq!(targets(&sent, golem).contains(&1), takes, "{event:?}: {:?}", targets(&sent, golem));
    }
}

#[test]
fn a_piglin_takes_no_player_in_a_golden_helmet() {
    let mut h = ready_at_night(GameMode::Survival, Difficulty::Easy);
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Armor(0), Some(&Item::new("minecraft:golden_helmet", 1)))));
    let piglin = spawn(&mut h, "minecraft:piglin", [0.5, 65.0, 4.5]);
    assert!(targets(&watch(&mut h, 60), piglin).is_empty(), "gold worn");
    let taken_off = ask(&mut h, |ctx| ctx.set_item(ALEX, Slot::Armor(0), None));
    assert_eq!((taken_off, targets(&watch(&mut h, 40), piglin)), (true, vec![1]), "none worn");
}

#[test]
fn a_polar_bear_rears_ten_ticks_before_its_bite_and_drops_with_it() {
    let mut h = ready_at_night(GameMode::Survival, Difficulty::Easy);
    let bear = spawn(&mut h, "minecraft:polar_bear", [0.5, 65.0, 2.0]);
    h.receive(1, &attack(bear));
    let mut rears = Vec::new();
    let mut bites = Vec::new();
    for tick in 0..120 {
        let sent = h.tick_all().to(1).to_vec();
        let flags = all::<SetEntityData>(&sent).iter().filter(|data| data.runtime_entity_id == bear).find_map(|data| data.metadata.iter().find_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0) } else { None }));
        let was = rears.last().is_some_and(|(_, on)| *on);
        if let Some(on) = flags.map(|word| word & Flags::REARING.0 != 0).filter(|on| *on != was) {
            rears.push((tick, on));
        }
        if all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == bear && event.event_id == EntityEventEventId::ArmSwing) {
            bites.push(tick);
        }
    }
    assert!(bites.len() >= 2, "{bites:?}");
    for bite in &bites {
        assert!(rears.contains(&(bite - 10, true)) && rears.contains(&(*bite, false)), "bite at {bite}: {rears:?}");
    }
}

/// The creeper's `IGNITED` flag each time `sent` tells its flags.
fn ignited(sent: &[acacia_proto::RawPacket], id: u64) -> Vec<bool> {
    all::<SetEntityData>(sent).iter().filter(|data| data.runtime_entity_id == id).filter_map(|data| data.metadata.iter().find_map(|item| if let Value::Flags(flags) = item.value { Some(flags.0 & Flags::IGNITED.0 != 0) } else { None })).collect()
}

#[test]
fn a_lit_creeper_goes_out_when_its_prey_hides_behind_a_wall_within_its_range() {
    let mut h = ready_at_night(GameMode::Survival, Difficulty::Easy);
    h.remote.run(|ctx| {
        for x in -1..=1 {
            for y in 65..=67 {
                ctx.set_block([x, y, 1], &sapling_api::Block::new("minecraft:stone"));
            }
        }
    });
    common::shooting::face(&mut h, 1, ALEX, [0.5, 65.0, 0.2], 0.0);
    let creeper = spawn(&mut h, "minecraft:creeper", [0.5, 65.0, 2.3]);
    hold(&mut h, &[creeper]);
    // bds-a3creeper3.txt: hidden 2.1 off, it took no target and was not lit.
    assert!(ignited(&watch(&mut h, 40), creeper).iter().all(|lit| !lit), "lit unseen");
    let beside = common::shooting::face(&mut h, 1, ALEX, [2.4, 65.0, 2.3], 0.0);
    let lit = ignited(&[beside, watch(&mut h, 10)].concat(), creeper);
    assert!(lit.contains(&true), "not lit beside it: {lit:?}");
    // It went out two ticks after the actor hid, and stayed out.
    let hidden = ignited(&[common::shooting::face(&mut h, 1, ALEX, [0.5, 65.0, 0.2], 0.0), watch(&mut h, 40)].concat(), creeper);
    assert!(hidden.last() == Some(&false) && hidden.iter().filter(|lit| !**lit).count() == 1, "hidden: {hidden:?}");
}

#[test]
fn speed_and_slowness_tell_a_mobs_walking_speed_once_for_each() {
    let mut h = ready_at_night(GameMode::Creative, Difficulty::Easy);
    let zombie = spawn(&mut h, "minecraft:zombie", [0.5, 65.0, 6.5]);
    let paces = |sent: &[acacia_proto::RawPacket]| -> Vec<f32> { all::<UpdateAttributes>(sent).iter().filter(|update| update.runtime_entity_id == zombie).flat_map(|update| update.attributes.iter().filter(|a| a.name == "minecraft:movement").map(|a| a.current).collect::<Vec<_>>()).collect() };
    // Given outside `ask`, whose tick's packets are not kept.
    h.remote.run(move |ctx| assert!(ctx.give_entity_effect(EntityId(zombie), Effect::Speed, 200, 1)));
    let quick = paces(&watch(&mut h, 1));
    h.remote.run(move |ctx| assert!(ctx.give_entity_effect(EntityId(zombie), Effect::Slowness, 200, 1)));
    let both = paces(&watch(&mut h, 1));
    h.remote.run(move |ctx| assert!(ctx.clear_entity_effects(EntityId(zombie))));
    let none = paces(&watch(&mut h, 2));
    // bds-a3fx.txt: 0.322, 0.2254, then 0.23 for each taken off.
    assert!(quick.len() == 1 && (quick[0] - 0.322).abs() < 1e-6, "{quick:?}");
    assert!(both.len() == 1 && (both[0] - 0.2254).abs() < 1e-6, "{both:?}");
    assert!(!none.is_empty() && none.iter().all(|pace| (pace - 0.23).abs() < 1e-6), "{none:?}");
}

#[test]
fn strength_and_weakness_scale_a_zombies_hit_as_a_players() {
    // bds-a3fx3.txt, on normal: 3 was 7.37 under strength II and 1.02 under weakness II.
    for (effect, hit) in [(Effect::Strength, 7.37), (Effect::Weakness, 1.02)] {
        let mut h = ready_at_night(GameMode::Survival, Difficulty::Normal);
        let zombie = spawn(&mut h, "minecraft:zombie", [0.5, 65.0, 1.5]);
        assert!(ask(&mut h, move |ctx| ctx.give_entity_effect(EntityId(zombie), effect, 600, 1)));
        let first = ask(&mut h, |ctx| ctx.health(ALEX));
        let left = (0..60).find_map(|_| ask(&mut h, |ctx| ctx.health(ALEX)).filter(|left| Some(*left) != first));
        let harm = left.map(|left| first.unwrap_or(20.0) - left);
        assert!(harm.is_some_and(|harm| (harm - hit).abs() < 1e-3), "{effect:?}: {harm:?}");
    }
}

#[test]
fn levitation_lifts_a_mob_a_fifth_of_the_way_to_its_speed_each_tick() {
    let mut h = ready_at_night(GameMode::Creative, Difficulty::Easy);
    let zombie = spawn(&mut h, "minecraft:zombie", [0.5, 65.0, 5.5]);
    hold(&mut h, &[zombie]);
    watch(&mut h, 5);
    assert!(ask(&mut h, move |ctx| ctx.give_entity_effect(EntityId(zombie), Effect::Levitation, 40, 1)));
    let heights: Vec<f32> = all::<MoveEntityDelta>(&watch(&mut h, 6)).iter().filter(|moved| moved.runtime_entity_id == zombie).filter_map(|moved| moved.y).collect();
    let rises: Vec<f32> = heights.windows(2).map(|pair| pair[1] - pair[0]).collect();
    // bds-a3fx.txt: -60, -59.9804, -59.945435, -59.898422, -59.841965.
    let first = heights[0] - 65.0;
    assert!((first - 0.0196).abs() < 1e-4 && rises.len() >= 3 && (rises[0] - 0.034965).abs() < 1e-4 && (rises[1] - 0.047013).abs() < 1e-4, "{heights:?}");
}

#[test]
fn slow_falling_drops_a_mob_by_a_hundredth_once_it_falls() {
    let mut h = ready_at_night(GameMode::Creative, Difficulty::Easy);
    let zombie = spawn(&mut h, "minecraft:zombie", [0.5, 65.0, 5.5]);
    hold(&mut h, &[zombie]);
    watch(&mut h, 5);
    assert!(ask(&mut h, move |ctx| ctx.give_entity_effect(EntityId(zombie), Effect::SlowFalling, 400, 0) && ctx.move_entity(EntityId(zombie), [0.5, 75.0, 5.5])));
    let heights: Vec<f32> = all::<MoveEntityDelta>(&watch(&mut h, 30)).iter().filter(|moved| moved.runtime_entity_id == zombie).filter_map(|moved| moved.y).collect();
    let falls: Vec<f32> = heights.windows(2).map(|pair| pair[0] - pair[1]).filter(|fall| *fall > 0.0).collect();
    // bds-a3fx2.txt: 0.0784 on the first tick, then each (v + 0.01) x 0.98 (0.2096, 0.21521, 0.22071).
    assert!(falls.len() > 10 && falls.windows(2).skip(3).all(|pair| (pair[1] - (pair[0] + 0.01) * 0.98).abs() < 1e-4) && falls.iter().all(|fall| *fall < 0.4), "{heights:?}");
}

#[test]
fn jump_boost_adds_a_tenth_a_level_to_a_mobs_jump() {
    let mut h = ready_at_night(GameMode::Survival, Difficulty::Easy);
    h.remote.run(|ctx| {
        for x in -3..=3 {
            for z in 4..=10 {
                ctx.set_block([x, 65, z], &sapling_api::Block::new("minecraft:stone"));
            }
        }
    });
    common::shooting::face(&mut h, 1, ALEX, [0.5, 66.0, 7.5], 0.0);
    let zombie = spawn(&mut h, "minecraft:zombie", [0.5, 65.0, 0.5]);
    assert!(ask(&mut h, move |ctx| ctx.give_entity_effect(EntityId(zombie), Effect::JumpBoost, 600, 2)));
    let ups: Vec<f32> = all::<SetEntityMotion>(&watch(&mut h, 100)).iter().filter(|motion| motion.runtime_entity_id == zombie).map(|motion| motion.velocity.y).collect();
    // bds-a3fx.txt: jump boost III, told 0.6272 = (0.72 - 0.08) x 0.98.
    assert!(ups.iter().any(|up| (up - 0.6272).abs() < 1e-4), "{ups:?}");
}
