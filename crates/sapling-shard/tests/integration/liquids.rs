//! Lava, buckets and waterlogged blocks (harness: common/mod.rs).

use crate::common;

use std::sync::{Arc, Mutex};

use acacia_proto::RawPacket;
use acacia_proto::packets::LevelSoundEvent;
use acacia_proto::types::Action;
use common::{Harness, all, block_actions, block_changes, click, hash, layered_changes};
use sapling_api::{Block, GameMode, Item, PlayerId, ShardCtx, Slot};

const ALEX: PlayerId = PlayerId(1);

fn liquid(name: &str, depth: u8) -> u32 {
    let registry = sapling_world::BlockRegistry::vanilla();
    registry.network_hash(registry.find(name, &format!("liquid_depth={depth}")).expect("a liquid of that depth"))
}

fn set(ctx: &mut ShardCtx<'_>, pos: [i32; 3], block: &str) {
    assert!(ctx.set_block(pos, &Block::new(block)), "{block} at {pos:?}");
}

/// The layer-0 changes `conn` is told of over `ticks` ticks, as (position, block).
fn changes_over(h: &mut Harness, conn: u64, ticks: usize) -> Vec<([i32; 3], u32)> {
    let sent: Vec<RawPacket> = (0..ticks).flat_map(|_| h.tick_all().to(conn).to_vec()).collect();
    block_changes(&sent)
}

/// Alex in creative, Steve watching, and a pool of `name` at x 4 dammed at x 3.
fn pool_of(name: &'static str) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(move |ctx| {
        [[3, 65, 0], [5, 65, 0], [4, 65, 1], [4, 65, -1]].into_iter().for_each(|wall| set(ctx, wall, "stone"));
        set(ctx, [4, 65, 0], name);
        ctx.set_game_mode(ALEX, GameMode::Creative);
    });
    h.tick_all();
    h
}

#[test]
fn lava_runs_six_times_slower_and_two_cells_shallower_than_water() {
    let mut h = pool_of("lava");
    h.receive(1, &block_actions(&[(Action::PredictBreak, [3, 65, 0])]));
    let early = changes_over(&mut h, 2, 29);
    assert_eq!(early, [([3, 65, 0], hash("minecraft:air"))], "nothing moves for 30 ticks");
    let ran = changes_over(&mut h, 2, 3);
    assert_eq!(ran, [([3, 65, 0], liquid("minecraft:lava", 2))]);
    let further = changes_over(&mut h, 2, 31);
    assert!(further.contains(&([2, 65, 0], liquid("minecraft:lava", 4))), "{further:?}");
}

fn held(h: &mut Harness, slot: u8) -> Option<(String, u8)> {
    let seen = Arc::new(Mutex::new(None));
    let out = seen.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = ctx.item(ALEX, Slot::Main(slot)).map(|item| (item.name, item.count)));
    h.tick_all();
    seen.lock().unwrap().take()
}

fn sounds(packets: &[RawPacket]) -> Vec<String> {
    all::<LevelSoundEvent>(packets).into_iter().map(|sound| sound.sound_id).collect()
}

#[test]
fn a_bucket_takes_a_source_and_pours_it_out_again() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let source = [2, 65, 0];
    h.remote.run(move |ctx| {
        set(ctx, source, "water");
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:bucket", 2))));
    });
    h.tick_all();

    // One of the two buckets is filled: it goes into the next free slot.
    h.receive(1, &click(source, 1, None));
    let sent = h.tick_all();
    assert_eq!(layered_changes(sent.to(2)).into_iter().map(|(_, at, block)| (at, block)).collect::<Vec<_>>(), [(source, hash("minecraft:air"))]);
    assert_eq!(sounds(sent.to(2)), ["bucket.fill.water"]);
    assert_eq!([held(&mut h, 0), held(&mut h, 1)], [Some(("minecraft:bucket".into(), 1)), Some(("minecraft:water_bucket".into(), 1))]);

    // Poured onto the platform from a full bucket in the hand, which comes back empty.
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:water_bucket", 1)))));
    h.tick_all();
    h.receive(1, &click([1, 64, 0], 1, None));
    let sent = h.tick_all();
    let poured: Vec<_> = layered_changes(sent.to(2)).into_iter().map(|(_, at, block)| (at, block)).collect();
    assert_eq!((poured, sounds(sent.to(2))), (vec![([1, 65, 0], liquid("minecraft:water", 0))], vec!["bucket.empty.water".to_string()]));
    assert_eq!(held(&mut h, 0), Some(("minecraft:bucket".into(), 1)));
    let ran = changes_over(&mut h, 2, 6);
    assert!(ran.contains(&([2, 65, 0], liquid("minecraft:water", 1))), "and it runs: {ran:?}");

    // An adventure player's bucket does nothing.
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Adventure);
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:lava_bucket", 1))));
    });
    h.tick_all();
    h.receive(1, &click([0, 64, 2], 1, None));
    assert!(changes_over(&mut h, 2, 1).is_empty());
    assert_eq!(held(&mut h, 0), Some(("minecraft:lava_bucket".into(), 1)));
}

/// Both layers `conn` is told of at `pos` over one tick, as (layer, block).
fn layers_at(h: &mut Harness, conn: u64, pos: [i32; 3]) -> Vec<(u32, u32)> {
    layered_changes(h.tick_all().to(conn)).into_iter().filter(|(_, at, _)| *at == pos).map(|(layer, _, block)| (layer, block)).collect()
}

#[test]
fn stairs_placed_into_a_source_keep_it_beside_them_and_give_it_back_when_broken() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let (wet, dry) = ([2, 65, 0], [3, 65, 0]);
    h.remote.run(move |ctx| {
        set(ctx, wet, "water");
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:oak_stairs", 2))));
    });
    h.tick_all();
    let stairs = |at: &[(u32, u32)]| at.iter().any(|(layer, block)| *layer == 0 && *block != hash("minecraft:air") && *block != liquid("minecraft:water", 0));

    // Clients keep the source beside the stairs by the same rule: BDS sends layer 0 only.
    h.receive(1, &click([2, 64, 0], 1, None));
    let placed = layers_at(&mut h, 2, wet);
    assert!(stairs(&placed) && placed.len() == 1, "{placed:?}");

    // A water bucket poured on dry stairs fills them.
    h.receive(1, &click([3, 64, 0], 1, None));
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:water_bucket", 1)))));
    h.tick_all();
    h.receive(1, &click(dry, 2, None));
    assert_eq!(layers_at(&mut h, 2, dry), [(1, liquid("minecraft:water", 0))]);

    h.remote.run(|ctx| ctx.set_game_mode(ALEX, GameMode::Creative));
    h.tick_all();
    h.receive(1, &block_actions(&[(Action::PredictBreak, wet)]));
    let broken = layers_at(&mut h, 2, wet);
    assert_eq!(broken, [(0, liquid("minecraft:water", 0))], "the source kept beside the stairs takes the cell");
}

#[test]
fn water_leaves_stairs_through_their_open_faces_only() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    // Stairs whose back is east (`weirdo_direction` 0), on the platform; a slab over a hole.
    let (stairs, slab) = ([2, 65, 0], [-2, 64, 2]);
    h.remote.run(move |ctx| {
        set(ctx, [-2, 63, 2], "air");
        assert!(ctx.set_block(stairs, &Block::new("oak_stairs").with("minecraft:corner=none,upside_down_bit=0,weirdo_direction=0")));
        assert!(ctx.set_block(slab, &Block::new("oak_slab").with("minecraft:vertical_half=bottom")));
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:water_bucket", 1))));
    });
    h.tick_all();
    h.receive(1, &click(stairs, 1, None));
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:water_bucket", 1)))));
    h.tick_all();
    h.receive(1, &click(slab, 1, None));
    let ran = changes_over(&mut h, 2, 12);
    let wet = |pos: [i32; 3]| ran.iter().any(|(at, block)| *at == pos && *block == liquid("minecraft:water", 1));
    assert!(wet([1, 65, 0]) && wet([2, 65, 1]) && wet([2, 65, -1]), "out of the front and the sides: {ran:?}");
    assert!(!ran.iter().any(|(at, _)| *at == [3, 65, 0]), "never out of the back");
    assert!(!ran.iter().any(|(at, _)| *at == [-2, 63, 2]), "a bottom slab keeps its water over a hole");
}

#[test]
fn water_that_reaches_a_lava_source_turns_it_to_obsidian() {
    let mut h = pool_of("lava");
    h.remote.run(|ctx| {
        [[1, 65, 0], [2, 65, 1], [2, 65, -1]].into_iter().for_each(|wall| set(ctx, wall, "stone"));
        set(ctx, [2, 65, 0], "water");
    });
    h.tick_all();
    h.receive(1, &block_actions(&[(Action::PredictBreak, [3, 65, 0])]));
    let changed = changes_over(&mut h, 2, 8);
    assert!(changed.contains(&([3, 65, 0], liquid("minecraft:water", 1))), "{changed:?}");
    assert!(changed.contains(&([4, 65, 0], hash("minecraft:obsidian"))), "{changed:?}");
}
