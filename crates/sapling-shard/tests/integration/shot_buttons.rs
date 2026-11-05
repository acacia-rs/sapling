//! Wooden buttons an arrow rests in (research/captures/bds-rsparts.txt): pressed once it lands, held
//! in while it stays, let out at the next 30-tick check after it went; a stone one stays out.

use crate::common;

use common::Harness;
use common::asking::ask;
use sapling_api::Block;

const WOODEN: [i32; 3] = [2, 65, 0];
const STONE: [i32; 3] = [5, 65, 0];

fn pressed(h: &mut Harness, pos: [i32; 3]) -> bool {
    ask(h, move |ctx| ctx.block(pos).is_some_and(|block| block.properties.contains("button_pressed_bit=1")))
}

fn ticks(h: &mut Harness, count: usize) {
    (0..count).for_each(|_| drop(h.tick_all()));
}

#[test]
fn an_arrow_resting_on_a_wooden_button_holds_it_in_and_a_stone_one_stays_out() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        for (pos, name) in [(WOODEN, "minecraft:wooden_button"), (STONE, "minecraft:stone_button")] {
            assert!(ctx.update_block(pos, &Block::new(name).with("button_pressed_bit=0,facing_direction=1")));
        }
        for x in [2.5, 5.5] {
            assert!(ctx.summon_as(None, "minecraft:arrow", [x, 68.0, 0.5], 0.0, 0.0, None).is_some());
        }
    });
    ticks(&mut h, 40);
    assert!(pressed(&mut h, WOODEN) && !pressed(&mut h, STONE), "the wooden one goes in, the stone one not");
    ticks(&mut h, 60);
    assert!(pressed(&mut h, WOODEN), "held while the arrow stays");
    let arrows: Vec<_> = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:arrow").map(|entity| entity.id).collect());
    h.remote.run(move |ctx| arrows.iter().for_each(|id| assert!(ctx.remove_entity(*id))));
    ticks(&mut h, 32);
    assert!(!pressed(&mut h, WOODEN), "out once the arrow is gone");
}
