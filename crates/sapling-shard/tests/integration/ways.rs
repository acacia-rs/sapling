//! A hunting zombie's way over drops and through water, against research/captures/bds-e1path.txt.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::UpdateAttributes;
use common::{Harness, all};
use sapling_api::{Block, Difficulty, GameMode, PlayerId};

/// Alex by night, Steve watching in creative off to the side, `blocks` set and a zombie at `zombie`.
fn hunted_over(blocks: Vec<([i32; 3], &'static str)>, zombie: [f32; 3]) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.receive(2, &common::input_at([0.5, 65.0, -6.5]));
    h.remote.run(move |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        ctx.set_game_mode(PlayerId(2), GameMode::Creative);
        for (pos, name) in blocks {
            assert!(ctx.set_block(pos, &Block::new(name)));
        }
        assert!(ctx.spawn_entity("minecraft:zombie", zombie).is_some());
    });
    h
}

fn alex_hurt(sent: &[RawPacket]) -> bool {
    all::<UpdateAttributes>(sent).iter().any(|update| update.runtime_entity_id == 1)
}

#[test]
fn a_zombie_on_a_ledge_three_up_takes_its_prey_below_and_drops_to_it() {
    // BDS's took the actor 1.6 s after it came and hit it at 6 s.
    let ledge = (-3..=3).flat_map(|x| (65..=67).flat_map(move |y| (4..=7).map(move |z| ([x, y, z], "stone")))).collect();
    let mut h = hunted_over(ledge, [0.5, 68.0, 6.5]);
    let sent: Vec<RawPacket> = (0..300).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    assert!(alex_hurt(&sent), "never hit");
}

#[test]
fn a_zombie_wades_a_pool_at_the_pace_its_legs_have_in_the_air() {
    // BDS's crossed a pool's floor at 0.45 blocks a second: 0.02 x 0.98 x 0.23 a tick over water's drag of 0.8.
    let pool = (-3..=3).flat_map(|x| (2..=7).map(move |z| ([x, 65, z], "water"))).collect();
    let mut h = hunted_over(pool, [0.5, 65.0, 6.5]);
    let mut zs = Vec::new();
    // Each ask is a tick.
    for _ in 0..60 {
        zs.extend(common::asking::ask(&mut h, |ctx| ctx.entities().into_iter().find(|found| found.kind == "minecraft:zombie").map(|found| found.feet[2])));
    }
    let fastest = zs.windows(2).skip(20).map(|pair| (pair[0] - pair[1]).abs()).fold(0.0, f32::max);
    assert!(zs.len() == 60 && zs[59] < zs[20] && fastest < 0.03, "{fastest} {zs:?}");
}
