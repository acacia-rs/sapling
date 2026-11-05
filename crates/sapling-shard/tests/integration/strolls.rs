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

/// A strolling cow's feet over `ticks` ticks on the open floor.
fn cow_feet(ticks: usize) -> Vec<[f32; 2]> {
    let mut h = Harness::with(sapling_shard::ShardConfig { mobs_stroll: true, ..common::config() }, common::platform());
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        let _ = ctx.set_game_mode(sapling_api::PlayerId(1), sapling_api::GameMode::Spectator);
        assert!(ctx.spawn_entity("minecraft:cow", [0.5, 65.0, 0.5]).is_some());
    });
    let feet = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    for _ in 0..ticks {
        let kept = feet.clone();
        h.remote.run(move |ctx| kept.lock().unwrap().extend(ctx.entities().first().map(|cow| [cow.feet[0], cow.feet[2]])));
        h.tick_all();
    }
    std::sync::Arc::try_unwrap(feet).unwrap().into_inner().unwrap()
}

#[test]
fn a_strolling_cow_is_done_with_a_cell_0_6_off_its_middle() {
    // bds-ad3strolls-nav.txt: BDS's cows passed a node, the last too, under 0.6 off its middle, so
    // a walk's last wanted tick was 0.61 or 0.62 off, the creep and slide after under 0.05.
    let feet = cow_feet(3000);
    let step = |pair: &[[f32; 2]]| (pair[1][0] - pair[0][0]).hypot(pair[1][1] - pair[0][1]);
    let off_middle = |at: [f32; 2]| (at[0] - at[0].floor() - 0.5).hypot(at[1] - at[1].floor() - 0.5);
    let rests: Vec<f32> = feet.windows(22).filter(|run| step(&run[..2]) > 0.005 && run[2..].windows(2).all(|pair| step(pair) <= 0.005) && run[11..].windows(2).all(|pair| step(pair) < 0.001)).map(|run| off_middle(run[21])).collect();
    assert!(!rests.is_empty(), "it never finished a walk");
    // A walk that came to its end at full speed slides on past it.
    let at_edge = rests.iter().filter(|off| (0.5..0.62).contains(*off)).count();
    assert!(at_edge * 4 >= rests.len() * 3, "walks ended this far off a middle: {rests:?}");
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
