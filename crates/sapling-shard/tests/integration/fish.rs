//! Fish hopping on land and fleeing players (creatures/swimming/ashore.rs), against
//! research/captures/bds-flop.txt.

use crate::common;

use common::{Harness, input_at};
use sapling_api::{Block, GameMode, PlayerId};

/// A cod's feet and speed each tick for `ticks` ticks, summoned at `at` with Alex in `mode` at `alex`.
fn watched(at: [f32; 3], alex: [f32; 3], mode: GameMode, pool: bool, ticks: usize) -> Vec<([f32; 3], [f32; 3])> {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.receive(1, &input_at(alex));
    h.remote.run(move |ctx| {
        let _ = ctx.set_game_mode(PlayerId(1), mode);
        if pool {
            for (x, y, z) in (-7..=7).flat_map(|x| (65..=66).flat_map(move |y| (-7..=7).map(move |z| (x, y, z)))) {
                assert!(ctx.set_block([x, y, z], &Block::new("water")));
            }
        }
        assert!(ctx.spawn_entity("minecraft:cod", at).is_some());
    });
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    for _ in 0..ticks {
        h.tick_all();
        let seen = seen.clone();
        h.remote.run(move |ctx| seen.lock().unwrap().extend(ctx.entities().into_iter().find(|mob| mob.kind == "minecraft:cod").map(|cod| (cod.feet, cod.velocity))));
    }
    seen.lock().unwrap().clone()
}

#[test]
fn a_cod_on_land_hops_half_a_block_up_and_falls_a_tenth_faster_a_tick() {
    let seen = watched([0.5, 65.0, 0.5], [6.5, 65.0, 6.5], GameMode::Survival, false, 60);
    let rises: Vec<f32> = seen.iter().map(|(_, speed)| speed[1]).filter(|rise| *rise > 0.0).collect();
    let falls_linearly = |run: &[f32]| run[0] >= 0.39 && run.windows(2).all(|pair| (pair[0] - pair[1] - 0.1).abs() < 1e-4);
    assert!(rises.windows(4).any(falls_linearly), "{rises:?}");
    let first_ticks: Vec<f32> = seen.windows(2).filter(|pair| pair[0].1[1] <= 0.0 && (pair[1].1[1] - 0.5).abs() < 1e-4).map(|pair| pair[1].1[0]).collect();
    let next_ticks: Vec<f32> = seen.windows(2).filter(|pair| (pair[0].1[1] - 0.5).abs() < 1e-4).map(|pair| pair[1].1[0] / pair[0].1[0]).collect();
    assert!(!first_ticks.is_empty() && next_ticks.iter().all(|kept| (kept - 0.91).abs() < 1e-3), "0.91 kept across from the hop's first tick: {next_ticks:?}");
    let hops = seen.windows(2).filter(|pair| pair[0].1[1] <= 0.0 && pair[1].1[1] > 0.0).count();
    assert!(hops >= 3, "it hops again a tick after each landing: {hops}");
    let across = seen.iter().map(|(feet, _)| (feet[0] - 0.5).hypot(feet[2] - 0.5)).fold(0.0, f32::max);
    assert!(across > 0.0 && across < 3.0, "{across}");
}

#[test]
fn a_cod_flees_a_player_near_it_at_twice_its_pace_but_not_a_spectator() {
    let fastest = |mode| watched([0.5, 65.2, 0.5], [2.5, 67.0, 0.5], mode, true, 60).iter().map(|(_, speed)| speed[0].hypot(speed[2])).fold(0.0, f32::max);
    let (fled, swam) = (fastest(GameMode::Survival), fastest(GameMode::Spectator));
    assert!(fled > 1.8 * swam && swam > 0.0, "{fled} {swam}");
}
