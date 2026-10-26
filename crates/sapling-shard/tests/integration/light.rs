//! Light as plugins read it: a lamp's, a roof's shade across a column border, and the night's.

use crate::common;

use common::Harness;
use sapling_api::{Block, Light};

#[test]
fn a_plugin_reads_lamps_roofs_and_the_night() {
    let mut h = Harness::new();
    h.remote.run(|ctx| {
        assert!(ctx.set_block([2, 65, 2], &Block::new("minecraft:glowstone")));
        for x in -6..=1 {
            for z in -3..=3 {
                assert!(ctx.set_block([x, 68, z], &Block::new("minecraft:stone")));
            }
        }
    });
    h.tick_all();
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send([[2, 66, 2], [-3, 65, 0], [-1, 65, 0], [-12, 65, 12]].map(|at| ctx.light_at(at))).expect("the test waits"));
    h.tick_all();
    let light = |sky, block| Some(Light { sky, block });
    // Under the roof the sky comes in from its edges, at x -1 from x 2 across the column border.
    assert_eq!(got.recv().expect("ran"), [light(15, 14), light(11, 8), light(12, 10), light(15, 0)]);

    h.remote.run(|ctx| ctx.set_time(18000));
    h.tick_all();
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send([[-12, 65, 12], [2, 66, 3], [2, 65, 2]].map(|at| ctx.brightness_at(at))).expect("the test waits"));
    h.tick_all();
    assert_eq!(got.recv().expect("ran"), [Some(4), Some(13), Some(0)], "the night sky, a lamp brighter than it, inside the lamp");
}
