//! Who may use a container: not one under a block that holds its lid down; a screen open until its
//! block goes.

use crate::common;

use acacia_proto::packets::{ContainerClose, ContainerOpen, LevelSoundEvent};
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
fn a_chest_is_heard_a_tick_after_it_opens_and_five_after_it_shuts_as_on_bds() {
    let mut h = at_a_chest();
    let heard = |sent: &[acacia_proto::RawPacket]| all::<LevelSoundEvent>(sent).into_iter().map(|sound| sound.sound_id).collect::<Vec<_>>();
    h.receive(1, &click(CHEST, 1, None));
    let opened = h.tick(1);
    let window_id = all::<ContainerOpen>(&opened)[0].window_id;
    let opening: Vec<Vec<String>> = std::iter::once(heard(&opened)).chain((0..2).map(|_| heard(&h.tick(1)))).collect();
    assert_eq!(opening, [vec![], vec!["chest.open".to_owned()], vec![]]);
    h.receive(1, &ContainerClose { window_id, window_type: acacia_proto::types::WindowType::Container, server: false });
    let answered = h.tick(1);
    let echo = all::<ContainerClose>(&answered);
    assert_eq!(echo.iter().map(|close| (close.window_type, close.server)).collect::<Vec<_>>(), [(acacia_proto::types::WindowType::None, false)], "BDS names no type");
    let shutting: Vec<Vec<String>> = std::iter::once(heard(&answered)).chain((0..6).map(|_| heard(&h.tick(1)))).collect();
    assert_eq!(shutting.iter().position(|sounds| sounds == &["chest.closed"]), Some(5), "{shutting:?}");
}

/// research/captures/bds-reach-close.txt: BDS kept the screen open 64 blocks off.
#[test]
fn a_screen_stays_open_however_far_its_player_goes_and_closes_when_its_chest_goes() {
    let mut h = at_a_chest();
    assert!(opens(&mut h));
    h.receive(1, &input_at([0.5, 65.0, 4.5]));
    h.remote.run(|ctx| ctx.teleport(sapling_api::PlayerId(1), [64.5, 65.0, 0.5]));
    assert!((0..3).all(|_| all::<ContainerClose>(&h.tick(1)).is_empty()), "open while far off");
    h.remote.run(|ctx| assert!(ctx.set_block(CHEST, &Block::air())));
    let closed: Vec<ContainerClose> = (0..2).flat_map(|_| all::<ContainerClose>(&h.tick(1))).collect();
    assert_eq!(closed.iter().map(|close| close.server).collect::<Vec<_>>(), [true]);
}
