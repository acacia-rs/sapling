//! Bubble columns over soul sand and magma (blocks/bubbles.rs; research/captures/bds-bubbles.txt).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::UpdateSubchunkBlocks;
use common::{Harness, all, block_changes, layered_changes};
use sapling_api::{Block, ShardCtx};

fn column(drags_down: bool) -> u32 {
    let registry = sapling_world::BlockRegistry::vanilla();
    registry.network_hash(registry.find("minecraft:bubble_column", if drags_down { "drag_down=1" } else { "drag_down=0" }).unwrap())
}

fn source() -> u32 {
    let registry = sapling_world::BlockRegistry::vanilla();
    registry.network_hash(registry.find("minecraft:water", "liquid_depth=0").unwrap())
}

fn set(ctx: &mut ShardCtx<'_>, pos: [i32; 3], block: &str) {
    assert!(ctx.set_block(pos, &Block::new(block)), "{block} at {pos:?}");
}

/// Each tick's layer-0 changes in the water at x 4, z 0, for `ticks` ticks.
fn ticks_of(h: &mut Harness, ticks: usize) -> Vec<Vec<([i32; 3], u32)>> {
    (0..ticks).map(|_| block_changes(&h.tick(1)).into_iter().filter(|(at, _)| at[0] == 4 && at[2] == 0 && at[1] > 64).collect()).collect()
}

#[test]
fn a_column_climbs_the_water_a_cell_a_tick_and_falls_back_six_ticks_a_cell() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| (65..=67).for_each(|y| set(ctx, [4, y, 0], "water")));
    h.tick(1);
    h.remote.run(|ctx| set(ctx, [4, 64, 0], "soul_sand"));
    h.tick(1);
    let rising = ticks_of(&mut h, 4);
    assert_eq!(rising, [vec![([4, 65, 0], column(false))], vec![([4, 66, 0], column(false))], vec![([4, 67, 0], column(false))], vec![]]);

    h.remote.run(|ctx| set(ctx, [4, 64, 0], "magma"));
    h.tick(1);
    let turned = ticks_of(&mut h, 3);
    assert_eq!(turned, [vec![([4, 65, 0], column(true))], vec![([4, 66, 0], column(true))], vec![([4, 67, 0], column(true))]]);

    h.remote.run(|ctx| set(ctx, [4, 64, 0], "stone"));
    h.tick(1);
    let gone: Vec<(usize, [i32; 3], u32)> = ticks_of(&mut h, 20).into_iter().enumerate().flat_map(|(tick, changes)| changes.into_iter().map(move |(at, id)| (tick, at, id))).collect();
    assert_eq!(gone, [(5, [4, 65, 0], source()), (11, [4, 66, 0], source()), (17, [4, 67, 0], source())]);
}

#[test]
fn a_cell_becoming_a_column_is_shown_with_its_water_kept_and_flowing() {
    // research/captures/bds-p2more.txt: one UpdateSubchunkBlocks, the column, and in the extra layer water then flowing_water.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| set(ctx, [4, 65, 0], "water"));
    h.tick(1);
    h.remote.run(|ctx| set(ctx, [4, 64, 0], "magma"));
    let sent: Vec<RawPacket> = (0..3).flat_map(|_| h.tick(1)).collect();
    let registry = sapling_world::BlockRegistry::vanilla();
    let flowing = registry.network_hash(registry.find("minecraft:flowing_water", "liquid_depth=0").unwrap());
    let shown: Vec<(u32, u32)> = layered_changes(&sent).into_iter().filter(|(_, at, _)| *at == [4, 65, 0]).map(|(layer, _, id)| (layer, id)).collect();
    assert_eq!(shown, [(0, column(true)), (1, source()), (1, flowing)]);
    assert_eq!(all::<UpdateSubchunkBlocks>(&sent).len(), 1, "all in one packet");
}
