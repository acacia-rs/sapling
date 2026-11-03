//! The still-open wave's plants (research/captures/bds-plantpop.txt, bds-plantpop2.txt): kelp and
//! nether vines losing their hold, and vines washed away by water run in beside them.

use crate::common;

use common::Harness;
use common::asking::ask;
use sapling_api::Block;

fn block_at(h: &mut Harness, pos: [i32; 3]) -> String {
    ask(h, move |ctx| ctx.block(pos).map(|block| block.name).unwrap_or_default())
}

fn set(h: &mut Harness, pos: [i32; 3], name: &'static str) {
    ask(h, move |ctx| assert!(ctx.update_block(pos, &Block::new(name))));
}

fn alex() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h
}

#[test]
fn kelp_on_glass_and_nether_vines_that_lose_their_block_break_at_a_neighbour_s_change() {
    let mut h = alex();
    for (x, under) in [(20, "minecraft:glass"), (24, "minecraft:stone")] {
        set(&mut h, [x, 65, 0], under);
        set(&mut h, [x, 66, 0], "minecraft:kelp");
        set(&mut h, [x + 1, 66, 0], "minecraft:stone");
    }
    assert_eq!((block_at(&mut h, [20, 66, 0]), block_at(&mut h, [24, 66, 0])), ("minecraft:water".to_owned(), "minecraft:kelp".to_owned()), "on glass it went, leaving its water");

    set(&mut h, [30, 65, 0], "minecraft:stone");
    set(&mut h, [30, 66, 0], "minecraft:twisting_vines");
    set(&mut h, [30, 65, 0], "minecraft:air");
    set(&mut h, [34, 70, 0], "minecraft:stone");
    set(&mut h, [34, 69, 0], "minecraft:weeping_vines");
    set(&mut h, [34, 70, 0], "minecraft:air");
    assert_eq!((block_at(&mut h, [30, 66, 0]), block_at(&mut h, [34, 69, 0])), ("minecraft:air".to_owned(), "minecraft:air".to_owned()));
}

#[test]
fn water_run_in_beside_a_vine_washes_it_away() {
    let mut h = alex();
    let vines = [(40, "minecraft:twisting_vines", [40, 65, 0]), (44, "minecraft:weeping_vines", [44, 67, 0]), (48, "minecraft:cave_vines", [48, 67, 0])];
    for (x, vine, support) in vines {
        // A glass trough two long, as bds-plantpop2.txt's, so the water can only run to the vine.
        ask(&mut h, move |ctx| {
            for (dx, y, z) in (-1..=2).flat_map(|dx| (65..=67).flat_map(move |y| (-1..=1).map(move |z| (dx, y, z)))) {
                ctx.update_block([x + dx, y, z], &Block::new("minecraft:glass"));
            }
            ctx.update_block([x, 66, 0], &Block::new("minecraft:air"));
            ctx.update_block([x + 1, 66, 0], &Block::new("minecraft:air"));
        });
        set(&mut h, support, "minecraft:stone");
        set(&mut h, [x, 66, 0], vine);
        set(&mut h, [x + 1, 66, 0], "minecraft:water");
        set(&mut h, [x + 2, 66, 0], "minecraft:stone");
    }
    (0..20).for_each(|_| drop(h.tick_all()));
    for (x, vine, _) in vines {
        assert_eq!(block_at(&mut h, [x, 66, 0]), "minecraft:water", "{vine}: research/captures/bds-plantpop2.txt");
    }
}
