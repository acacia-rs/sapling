//! Paintings players hang, against research/captures/bds-paintings.txt.

use crate::common;

use acacia_proto::packets::AddPainting;
use common::{Harness, all, click};
use sapling_api::{Item, PlayerId, Slot};
use sapling_world::World;

const ALEX: PlayerId = PlayerId(1);
const WEST: u8 = 4;

/// Alex with paintings before a stone wall at x 3 (y 65 to 70, z -6 to 6) on the floor, `blocks`
/// in front of it.
fn before_a_wall(blocks: &[[i32; 3]]) -> Harness {
    let mut level = World::void();
    let stone = level.block_id("minecraft:stone").unwrap();
    level.fill([3, 65, -6], [3, 70, 6], stone);
    blocks.iter().for_each(|&cell| level.fill(cell, cell, stone));
    let mut h = Harness::with(common::config(), level);
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:painting", 3)))));
    h.tick_all();
    h
}

fn hang(h: &mut Harness, wall: [i32; 3]) -> Vec<(String, [f32; 3], i32)> {
    h.receive(1, &click(wall, WEST, None));
    all::<AddPainting>(h.tick_all().to(1)).into_iter().map(|painting| (painting.title, [painting.coordinates.x, painting.coordinates.y, painting.coordinates.z], painting.direction)).collect()
}

#[test]
fn a_painting_on_a_big_wall_is_one_of_the_largest() {
    let mut h = before_a_wall(&[]);
    let hung = hang(&mut h, [3, 66, -1]);
    let [(title, at, direction)] = &hung[..] else { panic!("{hung:?}") };
    assert!(["Pointer", "Pigscene", "BurningSkull", "unpacked", "orb"].contains(&title.as_str()), "{title}");
    assert_eq!((*at, *direction), ([2.96875, 67.0, 0.0], 1));
    let left = common::asking::ask(&mut h, |ctx| ctx.item(ALEX, Slot::Main(0)).map(|held| held.count));
    assert_eq!(left, Some(2), "spent in survival");
}

#[test]
fn a_gap_two_wide_takes_a_two_by_one_and_the_next_does_not_overlap_it() {
    // A frame at x 2 leaves z -2 and -1 at y 66 open, as the capture's second wall.
    let frame = [[2, 65, -3], [2, 65, -2], [2, 65, -1], [2, 65, 0], [2, 66, -3], [2, 66, 0], [2, 67, -3], [2, 67, -2], [2, 67, -1], [2, 67, 0]];
    let mut h = before_a_wall(&frame);
    let hung = hang(&mut h, [3, 66, -2]);
    let [(title, at, _)] = &hung[..] else { panic!("{hung:?}") };
    assert!(["Pool", "Courbet", "Sea", "Sunset", "Creebet"].contains(&title.as_str()), "{title}");
    assert_eq!(*at, [2.96875, 66.5, -1.0]);
    assert_eq!(hang(&mut h, [3, 66, -1]), [], "the cell is taken");
}
