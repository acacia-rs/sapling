//! Which mobs a player is shown and which are ticked, and where random ticks run, as BDS's
//! `tick-distance` drills found (research/captures/bds-farmobs.txt, bds-tickcircle*.txt,
//! bds-tickrandom.txt; harness: common/mod.rs).

use crate::common;

use acacia_proto::packets::{AddEntity, RemoveEntity, RequestChunkRadius};
use common::asking::ask;
use common::{Harness, all, config, platform};
use sapling_api::{Block, RuleValue};
use sapling_shard::ShardConfig;

#[test]
fn random_ticks_run_only_in_the_tick_region_and_kept_columns_as_on_bds() {
    let mut h = Harness::with(ShardConfig { tick_distance: 4, max_view_radius: 8, ..config() }, platform());
    h.enter(1, "Alex");
    h.receive(1, &RequestChunkRadius { chunk_radius: 8, max_radius: 8 });
    (0..20).for_each(|_| drop(h.tick(1)));
    // bds-tickrandom.txt: bare farmland 4,1 chunks off (past the circle, in the diamond) dried, 4,2
    // and 5,0 off did not, 6,0 off in a ticking area did.
    let chunks = [([4, 1], true), ([4, 2], false), ([5, 0], false), ([6, 0], true)];
    let cell = |[x, z]: [i32; 2]| [x * 16 + 8, 64, z * 16 + 8];
    assert!(ask(&mut h, move |ctx| {
        ctx.force_chunk([6, 0], true);
        ctx.set_game_rule("randomTickSpeed", RuleValue::Int(200)) && chunks.iter().all(|(chunk, _)| ctx.set_block(cell(*chunk), &Block::new("minecraft:farmland")))
    }));
    (0..600).for_each(|_| drop(h.tick(1)));
    let dried: Vec<bool> = ask(&mut h, move |ctx| chunks.iter().map(|(chunk, _)| ctx.block(cell(*chunk)).is_some_and(|block| block.name == "minecraft:dirt")).collect());
    assert_eq!(dried, chunks.map(|(_, dries)| dries));
}

#[test]
fn a_mob_is_shown_only_within_four_chunks_as_on_bds() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let cow = ask(&mut h, |ctx| ctx.spawn_entity("minecraft:cow", [0.5, 65.0, 0.5])).expect("a cow");
    (0..3).for_each(|_| drop(h.tick(1)));
    // bds-farmobs.txt: chunks 3,2 and 4,0 off shown, 3,3 and 5,0 off not.
    for (chunk, shown) in [([3, 2], true), ([3, 3], false), ([4, 0], true), ([5, 0], false)] {
        let at = [chunk[0] as f32 * 16.0 + 8.5, 65.0, chunk[1] as f32 * 16.0 + 8.5];
        assert!(ask(&mut h, move |ctx| ctx.move_entity(cow, at)));
        let sent: Vec<_> = (0..2).flat_map(|_| h.tick(1)).collect();
        let (added, removed) = (!all::<AddEntity>(&sent).is_empty(), !all::<RemoveEntity>(&sent).is_empty());
        assert_eq!((added, removed), (shown && chunk == [4, 0], !shown), "{chunk:?}");
    }
}

#[test]
fn a_hidden_mob_ticks_within_the_tick_distance_and_waits_past_it_as_on_bds() {
    let mut h = Harness::with(ShardConfig { tick_distance: 6, max_view_radius: 8, ..config() }, platform());
    h.enter(1, "Alex");
    h.receive(1, &RequestChunkRadius { chunk_radius: 8, max_radius: 8 });
    (0..20).for_each(|_| drop(h.tick(1)));
    let cow = ask(&mut h, |ctx| ctx.spawn_entity("minecraft:cow", [0.5, 65.0, 0.5])).expect("a cow");
    (0..3).for_each(|_| drop(h.tick(1)));
    // bds-tickcircle6.txt: 3,3, 5,0 and 6,0 chunks off hidden but ticked; 5,3 (inside a circle of 6,
    // outside BDS's tick region) and 7,0 off neither shown nor ticked.
    for (chunk, ticked) in [([5, 0], true), ([3, 3], true), ([6, 0], true), ([5, 3], false), ([7, 0], false)] {
        let at = [chunk[0] as f32 * 16.0 + 8.5, 90.0, chunk[1] as f32 * 16.0 + 8.5];
        assert!(ask(&mut h, move |ctx| ctx.move_entity(cow, at)));
        let sent: Vec<_> = (0..5).flat_map(|_| h.tick(1)).collect();
        let feet = ask(&mut h, move |ctx| ctx.entity(cow)).expect("still there").feet;
        assert_eq!((feet[1] < 89.0, all::<AddEntity>(&sent).is_empty()), (ticked, true), "{chunk:?} {feet:?}");
    }
}
