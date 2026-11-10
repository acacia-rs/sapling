//! Group AM of the fourteenth wave: a zombie knocked against a ladder climbs it, and one summoned in
//! scaffolding stays where it was summoned (bds-am2push).

use crate::common;

use common::asking::ask;
use sapling_api::{Block, Difficulty, EntityId, GameMode, PlayerId};

const FLOOR: f32 = 65.0;

/// bds-am2push.txt: a zombie in an open ladder cell, hit from the west, was stopped by the ladder's
/// plate on its knock tick and rose 0.2 the next (0.4, 0.2, 0.1176, 0.0368 up), where a knock in the
/// open falls back from its 0.4 at once.
#[test]
fn a_zombie_knocked_against_a_ladder_climbs_it_the_next_tick() {
    let mut h = common::Harness::new();
    h.enter(1, "Alex");
    (0..12).for_each(|_| drop(h.tick(1)));
    let zombie = ask(&mut h, |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        for y in 65..=70 {
            for z in -1..=1 {
                assert!(ctx.set_block([3, y, z], &Block::new("minecraft:stone")));
            }
            assert!(ctx.set_block([2, y, 0], &Block::new("minecraft:ladder").with("facing_direction=4")));
        }
        ctx.spawn_entity("minecraft:zombie", [2.5, FLOOR, 0.5]).expect("a zombie").0
    });
    h.receive(1, &common::attack(zombie));
    drop(h.tick_all());
    let mut feet = Vec::new();
    for _ in 0..6 {
        feet.push(ask(&mut h, move |ctx| ctx.entity(EntityId(zombie)).expect("still there").feet));
    }
    // The knock's tick (0.4 up) went by in `tick_all`; BDS's next three stood 0.6, 0.7176 and 0.7544 up.
    let climbed = feet.iter().zip([0.6, 0.7176, 0.7544]).all(|(at, up)| (at[1] - FLOOR - up).abs() < 1e-3);
    assert!(climbed, "did not climb as BDS's: {feet:?}");
}

/// bds-am2push.txt: zombies summoned at x 2.3, 2.5 and 2.7 in a walled scaffolding cell stayed where
/// summoned, where a ladder's plate or a wall shoves a box out.
#[test]
fn a_zombie_summoned_in_scaffolding_is_not_shoved_out() {
    let mut h = common::Harness::new();
    h.enter(1, "Alex");
    let zombie = ask(&mut h, |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        ctx.set_difficulty(Difficulty::Easy);
        ctx.teleport(PlayerId(1), [-6.5, FLOOR, -6.5]);
        for y in 65..=70 {
            for [x, z] in [[1, -1], [1, 0], [1, 1], [2, -1], [2, 1], [3, -1], [3, 0], [3, 1]] {
                assert!(ctx.set_block([x, y, z], &Block::new("minecraft:stone")));
            }
            assert!(ctx.set_block([2, y, 0], &Block::new("minecraft:scaffolding")));
        }
        ctx.summon_as(None, "minecraft:zombie", [2.3, FLOOR, 0.45], 90.0, 0.0, None).expect("a zombie").0
    });
    let at = ask(&mut h, move |ctx| ctx.entity(EntityId(zombie)).expect("still there").feet);
    assert!((at[0] - 2.3).abs() < 1e-3 && (at[2] - 0.45).abs() < 1e-3, "shoved out: {at:?}");
}
