//! Who may use a container: not one under a block that holds its lid down, and not from out of
//! reach.

mod common;

use acacia_proto::packets::{ContainerClose, ContainerOpen};
use common::{Harness, all, click, input_at};
use sapling_api::Block;

const CHEST: [i32; 3] = [1, 65, 0];

/// Alex beside a chest, in survival with an empty hand.
fn at_a_chest() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_block(CHEST, &Block::new("chest"))));
    h.tick_all();
    h
}

fn opens(h: &mut Harness) -> bool {
    h.receive(1, &click(CHEST, 1, None));
    !all::<ContainerOpen>(&h.tick(1)).is_empty()
}

#[test]
fn a_chest_under_a_block_does_not_open_and_one_under_a_torch_does() {
    let mut h = at_a_chest();
    let above = [CHEST[0], CHEST[1] + 1, CHEST[2]];
    for (block, opens_under) in [("stone", false), ("oak_slab", false), ("torch", true), ("air", true)] {
        h.remote.run(move |ctx| assert!(ctx.set_block(above, &Block::new(block))));
        h.tick(1);
        assert_eq!(opens(&mut h), opens_under, "under {block}");
        h.receive(1, &ContainerClose { window_id: acacia_proto::types::WindowID::from_raw(0), window_type: acacia_proto::types::WindowType::Container, server: false });
        h.tick(1);
    }
}

#[test]
fn a_screen_closes_on_a_player_who_walks_out_of_reach() {
    let mut h = at_a_chest();
    assert!(opens(&mut h));
    h.receive(1, &input_at([0.5, 65.0, 4.5]));
    assert!(all::<ContainerClose>(&h.tick(1)).is_empty(), "still within reach");
    h.receive(1, &input_at([0.5, 65.0, 9.5]));
    let closed = all::<ContainerClose>(&h.tick(1));
    assert_eq!(closed.iter().map(|close| close.server).collect::<Vec<_>>(), [true]);
}
