//! Frosted ice set by command ages and melts only where it is lit, as BDS's did
//! (research/captures/bds-u1frost.txt).

use crate::common;

use common::Harness;
use sapling_api::{Block, GameMode, PlayerId};

const ALEX: PlayerId = PlayerId(1);
/// A corner of the 3x3, the cell farthest from the glowstone that BDS still melted.
const CORNER: [i32; 3] = [1, 65, 1];

/// A 3x3 of frosted ice at `time`, glowstone two over its middle if `lit` (as the drill's): the
/// corner cell after 300 ticks.
fn corner_after(time: i64, lit: bool) -> String {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        ctx.set_time(time);
        for (x, z) in (1..=3).flat_map(|x| (1..=3).map(move |z| (x, z))) {
            assert!(ctx.set_block([x, 65, z], &Block::new("minecraft:frosted_ice")));
        }
        if lit {
            assert!(ctx.set_block([2, 67, 2], &Block::new("minecraft:glowstone")));
        }
    });
    (0..300).for_each(|_| drop(h.tick_all()));
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(ctx.block(CORNER).map(|block| format!("{}[{}]", block.name, block.properties)).unwrap_or_default()).expect("the test waits"));
    h.tick_all();
    got.recv().expect("ran")
}

#[test]
fn frosted_ice_melts_by_day_and_by_a_light_but_not_in_the_dark() {
    assert!(corner_after(6000, false).starts_with("minecraft:water"), "noon");
    assert_eq!(corner_after(18000, false), "minecraft:frosted_ice[age=0]", "midnight, unlit");
    assert!(corner_after(18000, true).starts_with("minecraft:water"), "midnight, under glowstone");
}
