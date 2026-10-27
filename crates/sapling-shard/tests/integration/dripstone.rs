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
