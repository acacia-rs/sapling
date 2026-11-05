//! Hoppers in BDS's tick order, a jukebox fed and emptied by hoppers through a song, and a
//! comparator set again by command (research/captures/bds-e8hopchain.txt, bds-e8juke.txt,
//! bds-e8cmpset.txt).

use crate::common;

use common::Harness;
use sapling_api::{Block, Item};

fn comparator(rear: &str, subtract: bool) -> Block {
    Block::new("unpowered_comparator").with(format!("minecraft:cardinal_direction={rear},output_subtract_bit={}", u8::from(subtract)))
}

fn block_at(h: &mut Harness, pos: [i32; 3]) -> Block {
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(ctx.block(pos).expect("loaded")).expect("the test waits"));
    h.tick_all();
    got.recv().expect("ran")
}

fn lit(h: &mut Harness, dust: [i32; 3]) -> bool {
    block_at(h, dust).properties != "redstone_signal=0"
}

/// Four hoppers stepping down from y 69 under the chests they fill, one item in the first, a
/// comparator and dust reading the last chest at y 65; the dust's cell.
fn chain(start: i32, step: i32, z: i32) -> impl FnOnce(&mut sapling_api::ShardCtx<'_>) -> [i32; 3] {
    move |ctx| {
        let (facing, rear) = if step > 0 { (5, "west") } else { (4, "east") };
        let hopper = Block::new("hopper").with(format!("facing_direction={facing},toggle_bit=0"));
        for stage in 1..=4 {
            let (x, y) = (start + step * stage, 69 - stage);
            assert!(ctx.set_block([x, y + 1, z], &Block::new("chest")));
            assert!(ctx.set_block([x, y, z], &hopper));
        }
        let end = start + 5 * step;
        assert!(ctx.set_block([end, 65, z], &Block::new("chest")));
        assert!(ctx.set_block([end + step, 65, z], &comparator(rear, false)));
        assert!(ctx.set_block([end + 2 * step, 65, z], &Block::new("redstone_wire")));
        assert!(ctx.set_block([start, 69, z], &hopper));
        assert!(ctx.set_container_item([start, 69, z], 0, Some(&Item::new("minecraft:stone", 1))));
        [end + 2 * step, 65, z]
    }
}

#[test]
fn an_item_takes_as_long_down_a_chain_whichever_way_it_runs() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send([chain(2, 1, -4)(ctx), chain(14, -1, 4)(ctx)]).expect("the test waits"));
    h.tick_all();
    let dusts = got.recv().expect("ran");
    let mut arrived = [None; 2];
    for tick in 0..80 {
        let (sent, got) = std::sync::mpsc::channel();
        h.remote.run(move |ctx| sent.send(dusts.map(|dust| ctx.block(dust).expect("loaded").properties != "redstone_signal=0")).expect("the test waits"));
        h.tick_all();
        for (rig, lit) in got.recv().expect("ran").into_iter().enumerate() {
            if lit && arrived[rig].is_none() {
                arrived[rig] = Some(tick);
            }
        }
    }
    assert!(arrived[0].is_some(), "the item came through: {arrived:?}");
    assert_eq!(arrived[0], arrived[1], "BDS lit both comparators in one evaluation: the hopper under a chest moves before the one filling it");
}

#[test]
fn a_jukebox_takes_a_disc_from_a_hopper_and_gives_it_down_when_the_song_ends() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_block([4, 65, -4], &Block::new("hopper").with("facing_direction=5,toggle_bit=0")));
        assert!(ctx.set_block([5, 65, -4], &Block::new("chest")));
        assert!(ctx.set_block([6, 65, -4], &comparator("west", false)));
        assert!(ctx.set_block([7, 65, -4], &Block::new("redstone_wire")));
        assert!(ctx.set_block([4, 66, -4], &Block::new("jukebox")));
        assert!(ctx.set_block([3, 66, -4], &Block::new("redstone_lamp")));
        assert!(ctx.set_block([4, 67, -4], &Block::new("hopper").with("facing_direction=0,toggle_bit=0")));
        assert!(ctx.set_container_item([4, 67, -4], 0, Some(&Item::new("minecraft:music_disc_11", 1))));
    });
    for _ in 0..10 {
        h.tick_all();
    }
    assert_eq!(block_at(&mut h, [3, 66, -4]).name, "minecraft:lit_redstone_lamp", "the disc went in and plays");
    assert!(block_at(&mut h, [4, 65, -4]).properties.contains("toggle_bit=1"), "the jukebox powers the hopper under it");
    // Disc 11 is 71 s: BDS's power went 72 s after the disc went in.
    for _ in 0..1410 {
        h.tick_all();
    }
    assert_eq!(block_at(&mut h, [3, 66, -4]).name, "minecraft:lit_redstone_lamp", "still playing at 71 s");
    assert!(!lit(&mut h, [7, 65, -4]));
    for _ in 0..40 {
        h.tick_all();
    }
    assert_eq!(block_at(&mut h, [3, 66, -4]).name, "minecraft:redstone_lamp", "over by 73 s");
    for _ in 0..20 {
        h.tick_all();
    }
    assert!(lit(&mut h, [7, 65, -4]), "the hopper under it took the disc on to the chest");
}

#[test]
fn a_comparator_set_again_in_its_own_type_keeps_its_output() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    // 15 behind (west) and 15 at the south side; dust east of each comparator.
    let rig = |z: i32, subtract: bool| {
        move |ctx: &mut sapling_api::ShardCtx<'_>| {
            assert!(ctx.update_block([2, 65, z], &Block::new("redstone_block")));
            assert!(ctx.update_block([3, 65, z], &Block::new("redstone_wire")));
            assert!(ctx.update_block([4, 65, z + 2], &Block::new("redstone_block")));
            assert!(ctx.update_block([4, 65, z + 1], &Block::new("redstone_wire")));
            assert!(ctx.update_block([4, 65, z], &comparator("west", subtract)));
            assert!(ctx.update_block([5, 65, z], &Block::new("redstone_wire")));
        }
    };
    h.remote.run(rig(-6, false));
    h.remote.run(rig(-2, false));
    h.remote.run(rig(2, true));
    for _ in 0..10 {
        h.tick_all();
    }
    assert_eq!([-6, -2, 2].map(|z| lit(&mut h, [5, 65, z])), [true, true, false]);
    h.remote.run(|ctx| {
        let powered = Block::new("powered_comparator").with("minecraft:cardinal_direction=west,output_lit_bit=1,output_subtract_bit=1");
        assert!(ctx.update_block([4, 65, -6], &powered));
        assert!(ctx.update_block([4, 65, -2], &comparator("west", true)));
        assert!(ctx.update_block([4, 65, 2], &comparator("west", false)));
    });
    for _ in 0..10 {
        h.tick_all();
    }
    assert_eq!([-6, -2, 2].map(|z| lit(&mut h, [5, 65, z])), [true, false, false], "own type: left as it was; the other type: evaluated");
    h.remote.run(|ctx| assert!(ctx.update_block([4, 65, 3], &Block::new("air"))));
    for _ in 0..10 {
        h.tick_all();
    }
    assert!(lit(&mut h, [5, 65, 2]), "its side input gone, it reads its new mode");
}

/// research/captures/bds-p2hopper.txt: BDS's hopper took loot lying on its cell's edge, drifting off it,
/// within 2 ticks: its box, not its middle, reaches over the hopper.
#[test]
fn a_hopper_takes_an_item_whose_middle_is_off_it_but_whose_box_is_over_it() {
    const HOPPER: [i32; 3] = [6, 65, -3];
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_block(HOPPER, &Block::new("hopper").with("facing_direction=0,toggle_bit=0")));
        assert!(ctx.drop_item(&Item::new("minecraft:feather", 1), [5.95, 66.2, -2.5]));
    });
    let taken = (0..2).any(|_| {
        let (sent, got) = std::sync::mpsc::channel();
        h.remote.run(move |ctx| sent.send(ctx.container_item(HOPPER, 0).is_some()).expect("the test waits"));
        h.tick_all();
        got.recv().expect("ran")
    });
    assert!(taken, "the hopper took the feather");
}
