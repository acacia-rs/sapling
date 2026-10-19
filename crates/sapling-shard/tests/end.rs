//! A world's End: travel there by command as BDS makes it (research/captures/bds-end-travel.txt).

mod common;

use acacia_proto::Packet;
use acacia_proto::packets::{ChangeDimension, LevelChunk, PlayerAction};
use common::{Harness, all, ids, input_at};
use sapling_api::PlayerId;
use sapling_world::{END, World};

const ALEX: PlayerId = PlayerId(1);

/// A void End with an end stone floor at y 48 round (100, 0).
pub fn end() -> World {
    let mut end = World::void_in(END);
    let end_stone = end.block_id("minecraft:end_stone").unwrap();
    end.fill([84, 40, -16], [116, 48, 16], end_stone);
    end
}

#[test]
fn a_move_into_the_end_by_command_goes_as_into_the_nether() {
    let mut h = Harness::new();
    assert!(h.shard.add_dimension("default", end()));
    assert!(!h.shard.add_dimension("default", end()), "a world has one End");
    h.enter(1, "Alex");
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.travel(ALEX, END, [100.5, 49.0, 0.5])));
    let sent = h.tick_all();
    let change = all::<ChangeDimension>(sent.to(1)).remove(0);
    assert_eq!((change.dimension, [change.position.x, change.position.y, change.position.z], change.respawn, change.loading_screen_id), (2, [100.5, 49.0, 0.5], false, Some(0)));
    h.receive(1, &input_at([100.5, 49.0, 0.5]));
    let mut seen = Vec::new();
    for _ in 0..10 {
        seen.extend(h.tick_all().to(1).iter().cloned());
        if seen.iter().any(|p| p.id == PlayerAction::ID) {
            break;
        }
    }
    let columns = all::<LevelChunk>(&seen);
    assert!(!columns.is_empty() && columns.iter().all(|column| column.dimension == END), "{:?}", ids(&seen));
    assert!(columns.iter().any(|column| (column.x, column.z) == (6, 0) && column.highest_subchunk_count == Some(4)), "sections up to y 63 are asked for, as BDS's (6, 0)");
    h.remote.run(|ctx| {
        assert_eq!((ctx.world_of(ALEX).as_deref(), ctx.dimension_of(ALEX)), (Some("default:the_end"), Some(2)));
        assert!(ctx.travel(ALEX, 0, [0.5, 65.0, 0.5]));
    });
    let sent = h.tick_all();
    assert_eq!(all::<ChangeDimension>(sent.to(1)).remove(0).loading_screen_id, Some(1));
}
