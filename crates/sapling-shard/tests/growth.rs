//! What random ticks grow and dry, against research/captures/bds-grows.txt.

mod common;

use common::Harness;
use sapling_api::Block;

fn block(name: &str, properties: &str) -> Block {
    Block { name: format!("minecraft:{name}"), properties: properties.into() }
}

/// The names of the blocks at `cells` after `ticks` more ticks.
fn names(h: &mut Harness, ticks: usize, cells: &'static [[i32; 3]]) -> Vec<String> {
    for _ in 0..ticks {
        h.tick_all();
    }
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(cells.iter().map(|at| ctx.block(*at).map(|block| block.name.replace("minecraft:", "")).unwrap_or_default()).collect()).expect("the test waits"));
    h.tick_all();
    got.recv().expect("ran")
}

#[test]
fn cane_cactus_and_bamboo_grow_a_stem_bears_and_farmland_dries() {
    // BDS's, with the random ticks two hundred times as fast: cane and cactus three high in 14
    // seconds, a melon in 4, bamboo thirteen high in 28, farmland dirt in about one.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_game_rule("randomTickSpeed", sapling_api::RuleValue::Int(200)));
        let set: [([i32; 3], Block); 13] = [
            ([3, 64, -6], block("sand", "")),
            ([3, 65, -6], block("reeds", "")),
            ([3, 64, -4], block("sand", "")),
            ([3, 65, -4], block("cactus", "")),
            ([3, 64, -2], block("farmland", "moisturized_amount=7")),
            ([3, 65, -2], block("melon_stem", "growth=7")),
            ([4, 64, -2], block("dirt", "")),
            ([2, 64, -2], block("dirt", "")),
            ([3, 64, -3], block("dirt", "")),
            ([3, 64, -1], block("dirt", "")),
            ([3, 64, 1], block("dirt", "")),
            ([3, 65, 1], block("bamboo", "")),
            ([3, 64, 4], block("farmland", "moisturized_amount=7")),
        ];
        for (at, block) in set {
            assert!(ctx.set_block(at, &block), "{block:?}");
        }
    });
    let early = names(&mut h, 2, &[[3, 64, 4], [3, 66, -6], [3, 66, -4]]);
    assert_eq!(early, ["farmland", "air", "air"]);
    let grown = names(&mut h, 900, &[[3, 64, 4], [3, 64, -2], [3, 67, -6], [3, 68, -6], [3, 67, -4], [3, 68, -4], [3, 70, 1]]);
    assert_eq!(grown, ["dirt", "farmland", "reeds", "air", "cactus", "air", "bamboo"], "bare farmland dries to dirt, sown farmland stays, cane and cactus stop at three");
    let fruit = names(&mut h, 0, &[[4, 65, -2], [2, 65, -2], [3, 65, -3], [3, 65, -1]]);
    assert_eq!(fruit.iter().filter(|name| *name == "melon_block").count(), 1, "one melon beside the stem: {fruit:?}");
}

#[test]
fn grass_spreads_to_open_dirt_beside_it_and_not_under_a_block() {
    // BDS's dirt beside grass was grass in 2 seconds, the dirt beyond it in 8.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_game_rule("randomTickSpeed", sapling_api::RuleValue::Int(200)));
        let set = [([3, 64, 0], "grass_block"), ([4, 64, 0], "dirt"), ([5, 64, 0], "dirt"), ([3, 64, 1], "dirt"), ([3, 65, 1], "stone")];
        for (at, name) in set {
            assert!(ctx.set_block(at, &block(name, "")));
        }
    });
    let spread = names(&mut h, 1200, &[[4, 64, 0], [5, 64, 0], [3, 64, 1]]);
    assert_eq!(spread, ["grass_block", "grass_block", "dirt"]);
}
