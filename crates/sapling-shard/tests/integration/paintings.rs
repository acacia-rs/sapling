//! Paintings players hang, against research/captures/bds-paintings.txt.

use crate::common;

use acacia_proto::packets::{AddItemEntity, AddPainting, LevelEvent, LevelEventEvent, RemoveEntity};
use common::{Harness, all, attack, click};
use sapling_api::{Block, GameMode, Item, PlayerId, Slot};
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

/// What falling paintings showed over `ticks`: their break events, the items they dropped (where)
/// and the entities gone.
fn falls(h: &mut Harness, ticks: usize) -> (Vec<LevelEventEvent>, Vec<[f32; 3]>, usize) {
    let sent: Vec<acacia_proto::RawPacket> = (0..ticks).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let events = all::<LevelEvent>(&sent).into_iter().map(|event| event.event).filter(|event| matches!(event, LevelEventEvent::SoundItemframeRemove | LevelEventEvent::ParticleDestroyBlockNoSound)).collect();
    let items = all::<AddItemEntity>(&sent).into_iter().map(|item| [item.position.x, item.position.y, item.position.z]).collect();
    (events, items, all::<RemoveEntity>(&sent).len())
}

#[test]
fn a_hit_painting_falls_dropping_itself_but_not_for_a_creative_player() {
    // research/captures/bds-hangers.txt: the sound and oak plank particles, the item 1.125 above its middle.
    let mut h = before_a_wall(&[]);
    h.receive(1, &click([3, 66, -1], WEST, None));
    let id = all::<AddPainting>(h.tick_all().to(1))[0].runtime_entity_id;
    h.receive(1, &attack(id));
    let (events, items, gone) = falls(&mut h, 3);
    assert_eq!(events, [LevelEventEvent::SoundItemframeRemove, LevelEventEvent::ParticleDestroyBlockNoSound]);
    assert_eq!((items.len(), gone), (1, 1));
    assert!((items[0][0] - 2.96875).abs() < 1e-4 && (items[0][1] - 68.125).abs() < 1e-4 && items[0][2].abs() < 1e-4, "{items:?}");
    h.receive(1, &click([3, 66, -1], WEST, None));
    let id = all::<AddPainting>(h.tick_all().to(1))[0].runtime_entity_id;
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    h.tick_all();
    h.receive(1, &attack(id));
    let (events, items, gone) = falls(&mut h, 3);
    assert_eq!((events.len(), items.len(), gone), (2, 0, 1));
}

#[test]
fn a_painting_whose_wall_goes_falls_within_five_seconds() {
    let mut h = before_a_wall(&[]);
    assert_eq!(hang(&mut h, [3, 66, -1]).len(), 1);
    h.remote.run(|ctx| {
        for y in 65..=70 {
            for z in -6..=6 {
                ctx.set_block([3, y, z], &Block::new("minecraft:air"));
            }
        }
    });
    let (events, items, gone) = falls(&mut h, 101);
    assert_eq!((events.len(), items.len(), gone), (2, 1, 1));
    let (events, items, _) = falls(&mut h, 101);
    assert_eq!((events, items), (vec![], vec![]), "and is gone from its column");
}
