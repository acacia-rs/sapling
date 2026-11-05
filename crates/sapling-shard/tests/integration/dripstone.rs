//! Pointed dripstone falling (blocks/falling/dripstone.rs; research/captures/bds-dripstone.txt).

use crate::common;

use acacia_proto::packets::{AddItemEntity, EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent};
use acacia_proto::types::Action;
use common::{Harness, all, block_actions};
use sapling_api::{Block, GameMode, PlayerId, ShardCtx};

fn set(ctx: &mut ShardCtx<'_>, pos: [i32; 3], block: &Block) {
    assert!(ctx.set_block(pos, block), "{block:?} at {pos:?}");
}

fn piece(thickness: &str) -> Block {
    Block::new("pointed_dripstone").with(format!("dripstone_thickness={thickness},hanging=1"))
}

fn standing(thickness: &str) -> Block {
    Block::new("pointed_dripstone").with(format!("dripstone_thickness={thickness},hanging=0"))
}

/// Each of `cells` as `name[properties]`, a tick on.
fn states(h: &mut Harness, cells: Vec<[i32; 3]>) -> Vec<String> {
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(cells.iter().map(|at| ctx.block(*at).map_or(String::new(), |block| format!("{}[{}]", block.name, block.properties))).collect::<Vec<_>>()).unwrap());
    h.tick(1);
    got.recv().unwrap()
}

/// research/captures/bds-i2drip.txt: the stone under a stalagmite set to air, its frustum was shown
/// breaking at once and went with its item a tick on, its tip the tick after.
#[test]
fn a_stalagmite_whose_ground_goes_breaks_piece_by_piece_a_tick_apart() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        set(ctx, [4, 65, 0], &Block::new("stone"));
        set(ctx, [4, 66, 0], &standing("frustum"));
        set(ctx, [4, 67, 0], &standing("tip"));
    });
    h.tick(1);
    h.remote.run(|ctx| assert!(ctx.update_block([4, 65, 0], &Block::air())));
    let items: Vec<usize> = (0..4).map(|_| all::<AddItemEntity>(&h.tick(1)).len()).collect();
    assert_eq!(items, [0, 1, 1, 0], "the frustum a tick on, the tip the next");
    assert!(states(&mut h, vec![[4, 66, 0], [4, 67, 0]]).iter().all(|state| state.starts_with("minecraft:air")));
}

/// A stalactite from a dripstone block under water grows, by `PointedDripstoneBlock` 14660e6b0: its tip
/// a piece down, the old tip a frustum, or a stalagmite on the ground under it.
#[test]
fn a_stalactite_under_a_dripstone_block_under_water_grows() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        set(ctx, [4, 72, 0], &Block::new("water"));
        set(ctx, [4, 71, 0], &Block::new("dripstone_block"));
        set(ctx, [4, 70, 0], &piece("tip"));
        assert!(ctx.set_game_rule("randomTickSpeed", sapling_api::RuleValue::Int(4000)));
    });
    h.tick(1);
    (0..600).for_each(|_| _ = h.tick(1));
    let cells = states(&mut h, (64..=70).map(|y| [4, y, 0]).collect());
    let pieces: Vec<&String> = cells.iter().filter(|state| state.starts_with("minecraft:pointed_dripstone")).collect();
    assert!(pieces.len() > 1, "it grew: {cells:?}");
    assert!(cells[6].contains("hanging=1") && !cells[6].contains("thickness=tip"), "the root no longer the tip: {cells:?}");
    assert!(cells.iter().filter(|state| state.contains("thickness=tip")).count() <= 2, "{cells:?}");
}

#[test]
fn a_stalactite_whose_stone_goes_falls_whole_hurts_by_its_length_and_breaks_into_items() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        for y in 65..=66 {
            [[3, y, 0], [5, y, 0], [4, y, 1], [4, y, -1]].into_iter().for_each(|wall| set(ctx, wall, &Block::new("stone")));
        }
        set(ctx, [4, 71, 0], &Block::new("stone"));
        set(ctx, [4, 70, 0], &piece("base"));
        set(ctx, [4, 69, 0], &piece("frustum"));
        set(ctx, [4, 68, 0], &piece("tip"));
        assert!(ctx.spawn_entity("minecraft:cow", [4.5, 65.0, 0.5]).is_some());
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
    });
    h.tick(1);
    h.receive(1, &block_actions(&[(Action::PredictBreak, [4, 71, 0])]));
    let sent: Vec<_> = (0..40).flat_map(|_| h.tick(1)).collect();
    let hurts: Vec<i32> = all::<EntityEvent>(&sent).iter().filter(|event| event.event_id == EntityEventEventId::HurtAnimation).map(|event| event.data).collect();
    assert_eq!(hurts, [28], "the tip alone hurt the cow, as a stalactite");
    let landings = all::<LevelEvent>(&sent).iter().filter(|event| event.event == LevelEventEvent::PointedDripstoneLand).count();
    assert_eq!(landings, 3);
    assert_eq!(all::<AddItemEntity>(&sent).len(), 3, "each piece leaves its item");
    let (sent_back, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent_back.send([70, 69, 68, 65].map(|y| ctx.block([4, y, 0]).map(|block| block.name))).unwrap());
    h.tick(1);
    assert!(got.recv().unwrap().iter().all(|name| name.as_deref() == Some("minecraft:air")), "nothing is left standing");
}
