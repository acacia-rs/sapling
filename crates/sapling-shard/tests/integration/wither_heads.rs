//! The wither's side heads against research/captures/bds-y3wither.txt (docs/research/wither-lightning-fireworks.md).

use crate::common;

use acacia_proto::packets::AddEntity;
use common::asking::ask;
use common::{Harness, all};
use sapling_api::{Difficulty, GameMode, PlayerId};

const ALEX: PlayerId = PlayerId(1);
const WITHER: &str = "minecraft:wither";
const CHARGE: usize = 201;

/// A summoned wither on normal 8 off Alex, its charge over, Alex in `mode`.
fn summoned(mode: GameMode) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(Difficulty::Normal);
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.spawn_entity(WITHER, [0.5, 66.0, 8.5]).is_some());
    });
    (0..CHARGE).for_each(|_| drop(h.tick_all()));
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, mode);
        assert!(ctx.give_effect(ALEX, sapling_api::Effect::Resistance, 1000, 4, false));
    });
    h
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
fn a_skull_is_aimed_half_the_eye_height_over_a_players_eyes() {
    // BDS: 38 skulls passed the standing actor 2.41 over its feet on average (1.62 x 1.5 = 2.43).
    let mut h = summoned(GameMode::Survival);
    let (shot, feet) = (0..200)
        .find_map(|_| {
            let feet = ask(&mut h, |ctx| ctx.position(ALEX).unwrap().feet);
            all::<AddEntity>(h.tick_all().to(1)).into_iter().find(|added| added.entity_type == "minecraft:wither_skull").map(|added| (added, feet))
        })
        .expect("a skull");
    let ([px, py, pz], [vx, vy, vz]) = ([shot.position.x, shot.position.y, shot.position.z], [shot.velocity.x, shot.velocity.y, shot.velocity.z]);
    let along = ((feet[0] - px) * vx + (feet[2] - pz) * vz) / (vx * vx + vz * vz);
    let over = py + vy * along - feet[1];
    assert!((over - 2.43).abs() < 0.05, "{over}");
}
