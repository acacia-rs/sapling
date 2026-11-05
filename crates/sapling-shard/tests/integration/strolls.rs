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
