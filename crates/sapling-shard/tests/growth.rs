//! What random ticks grow and dry, against research/captures/bds-grows.txt.

mod common;

use acacia_proto::types::InputData;
use common::{Harness, input_at};
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

#[test]
fn leaves_decay_when_their_log_is_taken_and_a_players_do_not() {
    // research/captures/bds-leaves.txt: all 26 around the log were gone two seconds after it.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_game_rule("randomTickSpeed", sapling_api::RuleValue::Int(200)));
        for (z, kept) in [(-4, "persistent_bit=0"), (4, "persistent_bit=1")] {
            for at in (3..=5).flat_map(|x| (66..=68).flat_map(move |y| (z - 1..=z + 1).map(move |z| [x, y, z]))) {
                assert!(ctx.set_block(at, &block("oak_leaves", kept)));
            }
            assert!(ctx.set_block([4, 67, z], &block("oak_log", "")));
        }
    });
    let held = names(&mut h, 200, &[[3, 66, -5], [5, 68, -3], [3, 66, 3]]);
    assert_eq!(held, ["oak_leaves", "oak_leaves", "oak_leaves"], "nothing decays while the log stands");
    h.remote.run(|ctx| assert!(ctx.update_block([4, 67, -4], &Block::air()) && ctx.update_block([4, 67, 4], &Block::air())));
    let after = names(&mut h, 400, &[[3, 66, -5], [5, 68, -3], [4, 68, -4], [3, 66, 3], [5, 68, 5]]);
    assert_eq!(after, ["air", "air", "air", "oak_leaves", "oak_leaves"]);
}

#[test]
fn a_player_who_drops_onto_farmland_treads_it_to_dirt_and_one_who_steps_on_does_not() {
    // research/captures/bds-trample.txt: a drop of 0.8 left it, drops of 1.5 and 3 did not.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_block([0, 64, 0], &block("farmland", "")) && ctx.set_block([0, 65, 0], &block("wheat", ""))));
    let land = |h: &mut Harness, from: &[f32]| {
        for height in from {
            h.receive(1, &input_at([0.5, *height, 0.5]));
            h.tick_all();
        }
        let mut landing = input_at([0.5, 64.9375, 0.5]);
        (landing.input_data, landing.delta.y) = (vec![InputData::VerticalCollision], -1.0);
        h.receive(1, &landing);
    };
    land(&mut h, &[65.3]);
    assert_eq!(names(&mut h, 3, &[[0, 64, 0], [0, 65, 0]]), ["farmland", "wheat"]);
    land(&mut h, &[72.0, 70.0]);
    assert_eq!(names(&mut h, 3, &[[0, 64, 0], [0, 65, 0]]), ["dirt", "air"], "and the wheat on it goes");
}

#[test]
fn scaffolding_holds_six_out_from_its_column_and_comes_down_with_its_foot() {
    // research/captures/bds-scaffold.txt: put one at a time, as the drill put them.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    for at in [[-4, 65, 3], [-4, 66, 3]].into_iter().chain((-3..=3).map(|x| [x, 66, 3])) {
        h.remote.run(move |ctx| assert!(ctx.set_block(at, &block("scaffolding", ""))));
        for _ in 0..6 {
            h.tick_all();
        }
    }
    for _ in 0..40 {
        h.tick_all();
    }
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| {
        let far: Vec<String> = (-4..=3).map(|x| ctx.block([x, 66, 3]).map(|block| format!("{} {}", block.name.replace("minecraft:", ""), block.properties)).unwrap_or_default()).collect();
        sent.send((far, ctx.block([3, 65, 3]).map(|fallen| fallen.name))).expect("the test waits");
    });
    h.tick_all();
    let (far, fallen) = got.recv().expect("ran");
    let held: Vec<String> = (0..=6).map(|far| format!("scaffolding stability={far},stability_check=1")).chain(["air ".to_owned()]).collect();
    assert_eq!((far, fallen.as_deref()), (held, Some("minecraft:scaffolding")), "the seventh out fell to the floor");
    h.remote.run(|ctx| assert!(ctx.update_block([-4, 65, 3], &Block::air())));
    assert_eq!(names(&mut h, 40, &[[-4, 66, 3], [-1, 66, 3], [2, 66, 3], [3, 65, 3]]), ["air", "air", "air", "scaffolding"]);
}
