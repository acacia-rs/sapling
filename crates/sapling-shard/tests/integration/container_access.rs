//! Who may use a container: not one under a block that holds its lid down; a screen open until its
//! block goes.

use crate::common;

use acacia_proto::packets::{BlockEntityData, BlockEvent, ContainerClose, ContainerOpen, LevelSoundEvent, UpdateBlock};
use acacia_proto::packets::EntityEventEventId as Event;
use acacia_proto::types::MetadataFlags1 as Flags;
use common::raising::{click_on, has_flag, showed};
use common::shooting::spawn;
use common::{Harness, all, click, input_at};
use sapling_api::{Block, EntityId, Item, PlayerId, Slot};

const CHEST: [i32; 3] = [1, 65, 0];

/// Alex beside a chest, in survival with an empty hand.
fn at_a_chest() -> Harness {
    beside("chest")
}

/// Alex beside the container `block` at [`CHEST`].
fn beside(block: &'static str) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| assert!(ctx.set_block(CHEST, &Block::new(block))));
    h.tick_all();
    h
}

fn opens(h: &mut Harness) -> bool {
    h.receive(1, &click(CHEST, 1, None));
    !all::<ContainerOpen>(&h.tick(1)).is_empty()
}

#[test]
fn a_chest_under_a_block_does_not_open_and_one_under_a_torch_or_a_hopper_does() {
    // A hopper's `chest_obstruction` is `never` (research/captures/bds-p2hopper.txt).
    opens_under(at_a_chest(), &[("stone", false), ("oak_slab", false), ("torch", true), ("hopper", true), ("air", true)]);
}

/// research/captures/bds-e2lids.txt: a box set facing up opened under nothing but air.
#[test]
fn a_shulker_box_under_a_block_a_slab_or_a_carpet_does_not_open() {
    opens_under(beside("undyed_shulker_box"), &[("stone", false), ("oak_slab", false), ("white_carpet", false), ("air", true)]);
}

/// BDS `ChestBlockActor::_detectEntityObstruction` (beds-and-containers.md section 6): a sitting
/// cat in the lid's space holds the chest shut; standing, it does not.
#[test]
fn a_cat_sitting_on_a_chest_holds_it_shut() {
    let mut h = at_a_chest();
    let hold = |h: &mut Harness, item: Option<Item>| {
        h.remote.run(move |ctx| assert!(ctx.set_item(PlayerId(1), Slot::Main(0), item.as_ref())));
        h.tick(1);
    };
    hold(&mut h, Some(Item::new("minecraft:cod", 64)));
    let cat = spawn(&mut h, "minecraft:cat", [1.5, 66.0, 0.5]);
    let tamed = (0..40).map(|_| click_on(&mut h, cat)).find(|sent| showed(sent, cat, Event::TameSuccess)).expect("a third of fish tame");
    assert_eq!(has_flag(&tamed, cat, Flags::SITTING), Some(true), "a cat sits as it is tamed (bds-ar2cat.txt)");
    hold(&mut h, None);
    h.remote.run(move |ctx| assert!(ctx.move_entity(EntityId(cat), [1.5, 65.875, 0.5])));
    h.tick(1);
    h.tick(1);
    assert!(!opens(&mut h), "a sitting cat on it");
    h.receive(1, &ContainerClose { window_id: acacia_proto::types::WindowID::from_raw(0), window_type: acacia_proto::types::WindowType::Container, server: false });
    assert_eq!(has_flag(&click_on(&mut h, cat), cat, Flags::SITTING), Some(false));
    assert!(opens(&mut h), "a standing cat on it");
}

fn opens_under(mut h: Harness, cases: &[(&'static str, bool)]) {
    let above = [CHEST[0], CHEST[1] + 1, CHEST[2]];
    for &(block, opens_under) in cases {
        h.remote.run(move |ctx| assert!(ctx.set_block(above, &Block::new(block))));
        h.tick(1);
        assert_eq!(opens(&mut h), opens_under, "under {block}");
        h.receive(1, &ContainerClose { window_id: acacia_proto::types::WindowID::from_raw(0), window_type: acacia_proto::types::WindowType::Container, server: false });
        h.tick(1);
    }
}

fn heard(sent: &[acacia_proto::RawPacket]) -> Vec<String> {
    all::<LevelSoundEvent>(sent).into_iter().map(|sound| sound.sound_id).collect()
}

/// research/captures/bds-shut.txt: a chest sounds a tick after it opens and five after it shuts,
/// an ender chest and a barrel two and six, a shulker box two and eleven.
#[test]
fn containers_are_heard_as_late_as_on_bds() {
    let containers = [
        ("chest", [1, 5], ["chest.open", "chest.closed"]),
        ("trapped_chest", [1, 5], ["chest.open", "chest.closed"]),
        ("ender_chest", [2, 6], ["enderchest.open", "enderchest.closed"]),
        ("barrel", [2, 6], ["block.barrel.open", "block.barrel.close"]),
        ("undyed_shulker_box", [2, 11], ["shulkerbox.open", "shulkerbox.closed"]),
    ];
    for (block, [open, shut], [opening, shutting]) in containers {
        let mut h = beside(block);
        h.receive(1, &click(CHEST, 1, None));
        let opened = h.tick(1);
        let window_id = all::<ContainerOpen>(&opened)[0].window_id;
        let heard_opening: Vec<Vec<String>> = std::iter::once(heard(&opened)).chain((0..12).map(|_| heard(&h.tick(1)))).collect();
        assert_eq!(heard_opening.iter().position(|sounds| sounds == &[opening]), Some(open), "{block}: {heard_opening:?}");
        h.receive(1, &ContainerClose { window_id, window_type: acacia_proto::types::WindowType::Container, server: false });
        let answered = h.tick(1);
        let echo = all::<ContainerClose>(&answered);
        assert_eq!(echo.iter().map(|close| (close.window_type, close.server)).collect::<Vec<_>>(), [(acacia_proto::types::WindowType::None, false)], "BDS names no type");
        let heard_shutting: Vec<Vec<String>> = std::iter::once(heard(&answered)).chain((0..12).map(|_| heard(&h.tick(1)))).collect();
        assert_eq!(heard_shutting.iter().position(|sounds| sounds == &[shutting]), Some(shut), "{block}: {heard_shutting:?}");
    }
}

/// research/captures/bds-shut.txt: a barrel's top turns a tick after its lid, shown with what it
/// holds, and an open lid is shown again 80 ticks after it opened.
#[test]
fn a_barrel_turns_a_tick_after_it_opens_and_its_lid_is_shown_again_80_ticks_on() {
    let mut h = beside("barrel");
    h.tick(7);
    h.receive(1, &click(CHEST, 1, None));
    let ticks: Vec<(usize, usize, usize)> = (0..=80).map(|_| h.tick(1)).map(|sent| (all::<BlockEvent>(&sent).len(), all::<UpdateBlock>(&sent).len(), all::<BlockEntityData>(&sent).len())).collect();
    assert_eq!(ticks[..2], [(1, 0, 0), (0, 1, 1)]);
    let lids: Vec<usize> = ticks.iter().enumerate().filter(|(_, (lids, ..))| *lids > 0).map(|(at, _)| at).collect();
    assert_eq!(lids, [0, 80]);
}

/// Opens the chest, keeps it open `ticks` more ticks (a request at `request_at`), shuts it: the
/// ticks from the opening one on that showed a lid.
fn lids_open_for(h: &mut Harness, ticks: usize, request_at: Option<usize>) -> Vec<usize> {
    h.receive(1, &click(CHEST, 1, None));
    let opened = h.tick(1);
    let window_id = all::<ContainerOpen>(&opened)[0].window_id;
    let mut lids: Vec<usize> = Some(0).filter(|_| !all::<BlockEvent>(&opened).is_empty()).into_iter().collect();
    for at in 1..=ticks {
        if Some(at) == request_at {
            h.receive(1, &common::requests::request(-3, Vec::new()));
        }
        lids.extend(Some(at).filter(|_| !all::<BlockEvent>(&h.tick(1)).is_empty()));
    }
    h.receive(1, &ContainerClose { window_id, window_type: acacia_proto::types::WindowType::Container, server: false });
    h.tick(1);
    lids
}

/// BDS's `ChestBlockActor::tick` counts a lid's open ticks over its openings and shows it again at
/// each 80th; a request plays no part (bds-u3chest.txt: about 60 open, then again 20 into the next
/// opening; bds-n13win.txt: none at 79).
#[test]
fn an_open_lid_is_shown_again_at_each_80th_open_tick_counted_over_openings() {
    let mut h = at_a_chest();
    assert_eq!(lids_open_for(&mut h, 60, Some(30)), [0]);
    assert_eq!(lids_open_for(&mut h, 30, None), [0, 20]);
}

/// research/captures/bds-chest-moves.txt: an opened chest's slots are followed by the player's
/// windows, its held slot and the slots again, and those once more a tick on.
#[test]
fn an_opened_chest_is_shown_with_the_players_windows_and_again_a_tick_on() {
    use acacia_proto::Packet;
    use acacia_proto::packets::{InventoryContent, PlayerHotbar};
    let shown = [ContainerOpen::ID, InventoryContent::ID, PlayerHotbar::ID];
    let kinds = |sent: &[acacia_proto::RawPacket]| common::ids(sent).into_iter().filter(|id| shown.contains(id)).collect::<Vec<_>>();
    let mut h = at_a_chest();
    h.receive(1, &click(CHEST, 1, None));
    let opened = h.tick(1);
    let content = InventoryContent::ID;
    assert_eq!(kinds(&opened), [ContainerOpen::ID, content, content, content, content, content, PlayerHotbar::ID, content]);
    assert_eq!(kinds(&h.tick(1)), [content]);
    assert!(kinds(&h.tick(1)).is_empty());
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
