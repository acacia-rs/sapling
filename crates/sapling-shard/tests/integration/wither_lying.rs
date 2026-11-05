//! Withers called up from a T of soul sand lying on the floor (docs/research/wither-lightning-fireworks.md).

use crate::common;

use acacia_proto::packets::AddEntity;
use common::{Harness, all, click};
use sapling_api::{Block, Difficulty, GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);
const SKULL: &str = "minecraft:wither_skeleton_skull";

/// Alex, in creative, lays `sand` and `skulls` and puts the last skull on the top of `on`; the
/// wither's spot, if one came.
fn wither_from(sand: [[i32; 3]; 4], skulls: [[i32; 3]; 2], on: [i32; 3]) -> Option<[f32; 3]> {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_difficulty(Difficulty::Normal);
        ctx.set_game_mode(ALEX, GameMode::Creative);
        for pos in sand {
            ctx.set_block(pos, &Block::new("minecraft:soul_sand"));
        }
        for pos in skulls {
            ctx.set_block(pos, &Block::new(SKULL));
        }
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new(SKULL, 1))));
    });
    h.tick_all();
    h.receive(1, &click(on, 1, Some(SKULL)));
    let sent: Vec<_> = (0..3).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    all::<AddEntity>(&sent).into_iter().find(|added| added.entity_type == "minecraft:wither").map(|added| [added.position.x, added.position.y, added.position.z])
}

fn assert_at(at: Option<[f32; 3]>, x: f32, y: f32, z: f32) {
    let at = at.expect("a wither");
    assert!((at[0], at[2]) == (x, z) && (at[1] - y).abs() < 1e-3, "{at:?}");
}

/// BDS called a wither up from a T lying on the floor (research/captures/bds-n2wither.txt).
#[test]
fn a_t_lying_on_the_floor_calls_one_too() {
    assert!(wither_from([[0, 65, 5], [-1, 65, 4], [0, 65, 4], [1, 65, 4]], [[-1, 65, 3], [1, 65, 3]], [0, 64, 3]).is_some());
}

/// research/captures/bds-g8wither.txt: skulls along z at x 4, the stem west at x 2, gave the
/// wither at (6.5, -27.95, 0.5); here the same T a block east and 91 up.
#[test]
fn a_t_lying_with_its_skulls_along_z_calls_it_two_and_a_half_east_of_them() {
    assert_at(wither_from([[3, 65, 0], [4, 65, -1], [4, 65, 0], [4, 65, 1]], [[5, 65, -1], [5, 65, 0]], [5, 64, 1]), 7.5, 63.05, 0.5);
}

/// research/captures/bds-s3wither.txt: skulls along x at z 0, the stem north, the wither a block east of the corner rule.
#[test]
fn a_t_lying_with_its_skulls_along_x_calls_it_a_block_east_of_the_corner_rule() {
    assert_at(wither_from([[4, 65, -2], [3, 65, -1], [4, 65, -1], [5, 65, -1]], [[5, 65, 0], [4, 65, 0]], [3, 64, 0]), 5.5, 63.05, 1.5);
}
