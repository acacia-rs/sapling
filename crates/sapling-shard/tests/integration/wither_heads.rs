//! The wither's side heads against research/captures/bds-y3wither.txt (docs/research/wither-lightning-fireworks.md).

use crate::common;

use acacia_proto::packets::{AddEntity, SetEntityData};
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};
use common::asking::ask;
use common::{Harness, all};
use sapling_api::{Difficulty, GameMode, PlayerId};

const ALEX: PlayerId = PlayerId(1);
const STEVE: PlayerId = PlayerId(2);
const WITHER: &str = "minecraft:wither";
const CHARGE: usize = 201;

/// A summoned wither on normal 8 off Alex, its charge over, Alex in `mode`; Alex in `charging` while it charges.
fn summoned_after(charging: GameMode, mode: GameMode) -> Harness {
    let mut h = crate::fourteenth1ah::wide_floor();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_difficulty(Difficulty::Normal);
        // As y3wither: head 1's stray blue skulls would hole the floor under the wither.
        ctx.set_game_rule("mobGriefing", sapling_api::RuleValue::Bool(false));
        ctx.set_game_mode(ALEX, charging);
        // Its blast at the charge's end would kill an unwarded Alex.
        assert!(ctx.give_effect(ALEX, sapling_api::Effect::Resistance, 20000, 4, false));
        assert!(ctx.spawn_entity(WITHER, [0.5, 66.0, 8.5]).is_some());
    });
    (0..CHARGE).for_each(|_| drop(h.tick_all()));
    h.remote.run(move |ctx| ctx.set_game_mode(ALEX, mode));
    h
}

fn summoned(mode: GameMode) -> Harness {
    summoned_after(GameMode::Creative, mode)
}

/// What the side heads were told to look at over `ticks`, `WitherTarget2` and `WitherTarget3`.
fn side_targets(h: &mut Harness, ticks: usize) -> [Vec<i64>; 2] {
    let sent: Vec<_> = (0..ticks).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let told = |key: Key| -> Vec<i64> {
        let items = all::<SetEntityData>(&sent).into_iter().flat_map(|data| data.metadata).filter(|item| item.key == key);
        items.filter_map(|item| if let Value::Default(Plain::Long(long)) = item.value { Some(long) } else { None }).collect()
    };
    [told(Key::WitherTarget2), told(Key::WitherTarget3)]
}

#[test]
fn a_side_head_keeps_a_player_not_the_middle_heads_prey_until_they_turn_creative() {
    // BDS (`WitherBoss` 0xbbbd900; bds-y3wither.txt, bds-f14nwither2.txt): a side head draws among the
    // players in reach less the wither's own target, keeps the one it took (`WitherTarget3` held the
    // actor 52 s) and lets go (-1) as it turns creative. Alex, nearer, is the middle head's prey.
    let mut h = summoned_after(GameMode::Survival, GameMode::Survival);
    h.enter(2, "Steve");
    h.remote.run(|ctx| {
        ctx.set_game_mode(STEVE, GameMode::Survival);
        assert!(ctx.give_effect(STEVE, sapling_api::Effect::Resistance, 20000, 4, false));
        ctx.teleport(STEVE, [0.5, 65.0, -12.5]);
    });
    let held = side_targets(&mut h, 600);
    let took: Vec<usize> = (0..2).filter(|head| !held[*head].is_empty()).collect();
    assert!(!took.is_empty(), "{held:?}");
    let steve = held[took[0]][0];
    // Steve may pass out of sight or reach as the wither swoops (then -1, and taken again): never Alex.
    assert!(steve > 1 && took.iter().all(|head| held[*head][0] == steve && held[*head].iter().all(|told| [steve, -1].contains(told))), "{held:?}");
    h.remote.run(|ctx| ctx.set_game_mode(STEVE, GameMode::Creative));
    let after = side_targets(&mut h, 80);
    let last = |head: usize| after[head].last().or(held[head].last()).copied();
    assert!(took.iter().all(|head| last(*head) == Some(-1)), "{held:?} then {after:?}");
}

#[test]
fn with_nobody_to_shoot_one_side_head_shoots_a_blue_skull_on_its_17th_idle_look() {
    // BDS: blue skulls 231, 242, 264 and 243 ticks apart, one stream (17 looks of 10 to 19 ticks).
    let mut h = summoned(GameMode::Creative);
    let blue: Vec<usize> = (0..1500).filter(|_| all::<AddEntity>(h.tick_all().to(1)).iter().any(|added| added.entity_type == "minecraft:wither_skull_dangerous")).collect();
    let gaps: Vec<usize> = blue.windows(2).map(|pair| pair[1] - pair[0]).collect();
    assert!(gaps.len() >= 3 && gaps.iter().all(|gap| (170..=323).contains(gap)), "{blue:?}");
}

#[test]
fn skulls_are_aimed_half_the_eye_height_over_a_players_eyes_and_spread_about_it() {
    // BDS: 38 skulls passed the standing actor 2.41 over its feet on average (1.62 x 1.5 = 2.43),
    // spread with sd 1.15 (the pack's uncertainty 7.5 less 1 a level); one skull is not the mean.
    let mut h = summoned(GameMode::Survival);
    let (mut overs, mut seen) = (Vec::new(), Vec::new());
    // Unarmoured, its middle head rests through each swoop (wither/swoop.rs): about 4 shots in 8 s.
    for tick in 0..6000 {
        let (feet, wither) = ask(&mut h, |ctx| (ctx.position(ALEX).unwrap().feet, ctx.entities().into_iter().find(|entity| entity.kind == WITHER).map(|entity| entity.feet)));
        if tick % 500 == 0 {
            seen.push((tick, feet, wither, overs.len()));
        }
        for shot in all::<AddEntity>(h.tick_all().to(1)).into_iter().filter(|added| added.entity_type == "minecraft:wither_skull") {
            let ([px, py, pz], [vx, vy, vz]) = ([shot.position.x, shot.position.y, shot.position.z], [shot.velocity.x, shot.velocity.y, shot.velocity.z]);
            let along = ((feet[0] - px) * vx + (feet[2] - pz) * vz) / (vx * vx + vz * vz);
            overs.push(py + vy * along - feet[1]);
        }
        if overs.len() >= 80 {
            break;
        }
    }
    assert!(overs.len() >= 80, "{} skulls; Alex, the wither and the count every 500: {seen:?}", overs.len());
    let mean = overs.iter().sum::<f32>() / overs.len() as f32;
    let sd = (overs.iter().map(|over| (over - mean).powi(2)).sum::<f32>() / overs.len() as f32).sqrt();
    // Within 2.5 standard errors of an sd up to 1.5.
    assert!((mean - 2.43).abs() < 0.42, "mean {mean} of {overs:?}");
    assert!((0.15..2.0).contains(&sd), "sd {sd} of {overs:?}");
}
