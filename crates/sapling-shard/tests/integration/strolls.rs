//! Strolls along a way over whole cells (docs/research/mob-attacks.md "Strolling").

use crate::common;

use common::Harness;
use sapling_api::Block;

#[test]
fn a_strolling_cow_goes_round_a_wall_to_a_place_behind_it() {
    // research/captures/bds-strolls.txt: BDS's walks went from cell middle to cell middle, not straight.
    let mut h = Harness::with(sapling_shard::ShardConfig { mobs_stroll: true, ..common::config() }, common::platform());
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        let _ = ctx.set_game_mode(sapling_api::PlayerId(1), sapling_api::GameMode::Spectator);
        for x in -8..=5 {
            for y in 65..=66 {
                ctx.set_block([x, y, 2], &Block::new("minecraft:stone"));
            }
        }
        assert!(ctx.spawn_entity("minecraft:cow", [-5.5, 65.0, -3.5]).is_some());
    });
    let behind = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    for _ in 0..4000 {
        let flag = behind.clone();
        h.remote.run(move |ctx| {
            if ctx.entities().first().is_some_and(|cow| cow.feet[2] > 3.0) {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        });
        h.tick_all();
        if behind.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
    }
    assert!(behind.load(std::sync::atomic::Ordering::Relaxed), "it never went round the wall's end");
}

#[test]
fn a_strolling_cow_slows_to_half_its_speed_on_its_last_block() {
    // research/captures/bds-strolls.txt: a cow's walk ends 0.086, 0.076, 0.062, 0.049, 0.038, 0.031,
    // 0.027, 0.024, 0.023, 0.022 a tick, then stands (MoveControl::calculateMoveSpeed).
    let mut h = Harness::with(sapling_shard::ShardConfig { mobs_stroll: true, ..common::config() }, common::platform());
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        let _ = ctx.set_game_mode(sapling_api::PlayerId(1), sapling_api::GameMode::Spectator);
        assert!(ctx.spawn_entity("minecraft:cow", [0.5, 65.0, 0.5]).is_some());
    });
    let feet = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    for _ in 0..3000 {
        let kept = feet.clone();
        h.remote.run(move |ctx| kept.lock().unwrap().extend(ctx.entities().first().map(|cow| [cow.feet[0], cow.feet[2]])));
        h.tick_all();
    }
    let feet = feet.lock().unwrap();
    let steps: Vec<f32> = feet.windows(2).map(|pair| (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1])).collect();
    let creeping = |run: &[f32]| run.iter().all(|step| (0.019..0.032).contains(step));
    assert!(steps.iter().any(|step| *step > 0.08), "it never walked at full speed");
    assert!(steps.windows(3).any(creeping), "no walk ended creeping at about 0.022 a tick: {steps:?}");
}
