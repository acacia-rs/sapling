//! Growth as BDS showed it (research/captures/bds-croprates.txt, bds-lightgrow.txt;
//! docs/research/growth.md "Light").

use crate::common;

use common::Harness;
use sapling_api::Block;

fn names(h: &mut Harness, cells: Vec<[i32; 3]>) -> Vec<String> {
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(cells.iter().map(|at| ctx.block(*at).map_or(String::new(), |block| block.name.replace("minecraft:", ""))).collect()).unwrap());
    h.tick_all();
    got.recv().unwrap()
}

#[test]
fn wheat_filled_over_wet_farmland_stays_and_only_the_one_over_water_pops() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        for x in 0..5 {
            assert!(ctx.set_block([x, 64, 0], &Block::new("farmland").with("moisturized_amount=7")));
        }
        assert!(ctx.set_block([2, 64, 0], &Block::new("water")));
    });
    h.tick_all();
    h.remote.run(|ctx| {
        ctx.batch_blocks(|ctx| (0..5).for_each(|x| _ = ctx.update_block([x, 65, 0], &Block::new("wheat"))));
    });
    h.tick_all();
    assert_eq!(names(&mut h, (0..5).map(|x| [x, 65, 0]).collect()), ["wheat", "wheat", "air", "wheat", "wheat"]);
}

#[test]
fn a_mushroom_in_daylight_where_a_filled_block_was_cleared_breaks_when_poked() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let cells = |y0: i32, y1: i32| (-7..=15).flat_map(move |x| (y0..=y1).flat_map(move |y| (-7..=7).map(move |z| [x, y, z]))).collect::<Vec<_>>();
    h.remote.run(move |ctx| ctx.batch_blocks(|ctx| cells(64, 69).into_iter().for_each(|at| _ = ctx.update_block(at, &Block::new("stone")))));
    h.tick_all();
    h.remote.run(move |ctx| ctx.batch_blocks(|ctx| cells(65, 69).into_iter().for_each(|at| _ = ctx.update_block(at, &Block::air()))));
    h.tick_all();
    h.remote.run(|ctx| _ = ctx.update_block([6, 65, 0], &Block::new("brown_mushroom")));
    h.tick_all();
    h.remote.run(|ctx| _ = ctx.update_block([7, 65, 0], &Block::new("stone")));
    h.tick_all();
    assert_eq!(names(&mut h, vec![[6, 65, 0]]), ["air"], "BDS's broke (research/captures/bds-lightpoke.txt)");
}

#[test]
fn wheat_in_a_dark_room_stays_when_a_light_block_is_set_beside_it() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        for at in [[0, 64, 0], [1, 64, 0], [-1, 65, 0], [2, 65, 0], [0, 65, -1], [0, 65, 1], [1, 65, -1], [1, 65, 1], [0, 66, 0], [1, 66, 0]] {
            assert!(ctx.set_block(at, &Block::new("stone")));
        }
        assert!(ctx.set_block([0, 64, 0], &Block::new("farmland").with("moisturized_amount=7")));
        assert!(ctx.set_block([1, 64, 0], &Block::new("water")));
    });
    h.tick_all();
    h.remote.run(|ctx| _ = ctx.update_block([0, 65, 0], &Block::new("wheat")));
    h.tick_all();
    h.remote.run(|ctx| _ = ctx.update_block([1, 65, 0], &Block::new("light_block_10")));
    h.tick_all();
    assert_eq!(names(&mut h, vec![[0, 65, 0]]), ["wheat"]);
}
