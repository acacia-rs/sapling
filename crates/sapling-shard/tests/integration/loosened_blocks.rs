//! What loses its hold when the block it hangs on goes (research/captures/bds-holds.txt): shown
//! breaking in that tick and gone, with its item, a tick later; a vine that hung from a vine is shown
//! breaking as that one goes and goes a tick after.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use common::asking::ask;
use common::{Harness, all};
use sapling_api::Block;

fn name_at(h: &mut Harness, pos: [i32; 3]) -> String {
    ask(h, move |ctx| ctx.block(pos).map(|block| block.name)).unwrap_or_default()
}

fn breaks_shown(sent: &[RawPacket]) -> usize {
    all::<LevelEvent>(sent).into_iter().filter(|event| event.event == LevelEventEvent::ParticleDestroy).count()
}

fn put(h: &mut Harness, blocks: &'static [([i32; 3], &'static str, &'static str)]) {
    h.remote.run(move |ctx| {
        for (pos, name, properties) in blocks {
            assert!(ctx.update_block(*pos, &Block::new(*name).with(*properties)));
        }
    });
    h.tick_all();
}

#[test]
fn a_lever_whose_block_goes_is_shown_breaking_and_goes_a_tick_later() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    put(&mut h, &[([2, 66, 0], "minecraft:stone", ""), ([3, 66, 0], "minecraft:lever", "lever_direction=east,open_bit=0")]);
    h.remote.run(|ctx| assert!(ctx.update_block([2, 66, 0], &Block::new("minecraft:air"))));
    let now = h.tick_all().to(1).to_vec();
    assert_eq!(breaks_shown(&now), 1, "shown breaking with its block's going");
    assert_eq!(name_at(&mut h, [3, 66, 0]), "minecraft:air", "gone a tick later");
    let items = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.item.is_some()).count());
    assert_eq!(items, 1, "with its item");
}

#[test]
fn a_vine_goes_a_tick_after_its_block_and_the_vine_under_it_a_tick_after_that() {
    const UPPER: [i32; 3] = [3, 67, 0];
    const LOWER: [i32; 3] = [3, 66, 0];
    let mut h = Harness::new();
    h.enter(1, "Alex");
    put(&mut h, &[([2, 67, 0], "minecraft:stone", ""), (UPPER, "minecraft:vine", "vine_direction_bits=2"), (LOWER, "minecraft:vine", "vine_direction_bits=2")]);
    h.remote.run(|ctx| assert!(ctx.update_block([2, 67, 0], &Block::new("minecraft:air"))));
    let cut = h.tick_all().to(1).to_vec();
    assert_eq!(breaks_shown(&cut), 1, "the upper vine shown breaking");
    let next = h.tick_all().to(1).to_vec();
    assert_eq!(breaks_shown(&next), 1, "the lower vine shown breaking as the upper goes");
    let after = h.tick_all().to(1).to_vec();
    assert_eq!(breaks_shown(&after), 0, "nothing more shown");
    assert_eq!([name_at(&mut h, UPPER), name_at(&mut h, LOWER)], ["minecraft:air", "minecraft:air"]);
}
