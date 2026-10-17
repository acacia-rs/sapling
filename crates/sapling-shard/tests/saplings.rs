//! Saplings growing into worldgen's trees by bone meal and by random tick
//! (docs/research/saplings.md).

mod common;

use acacia_proto::packets::UpdateSubchunkBlocks;
use common::{Harness, all, click};
use sapling_api::{Block, GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

/// Alex in creative with bone meal in hand, `saplings` set on dirt.
fn planted(saplings: &'static [([i32; 3], &'static str)]) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:bone_meal", 8))));
        for x in -4..=8 {
            for z in -6..=6 {
                assert!(ctx.set_block([x, 64, z], &Block::new("minecraft:dirt")));
            }
        }
        for (at, name) in saplings {
            assert!(ctx.set_block(*at, &Block::new(*name)));
        }
    });
    h.tick_all();
    h
}

fn names(h: &mut Harness, cells: Vec<[i32; 3]>) -> Vec<String> {
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(cells.iter().map(|at| ctx.block(*at).map(|block| block.name).unwrap_or_default()).collect()).expect("the test waits"));
    h.tick_all();
    got.recv().expect("ran")
}

#[test]
fn bone_meal_in_creative_grows_an_oak_at_once_shown_a_sub_chunk_at_a_time() {
    let mut h = planted(&[([2, 65, 0], "minecraft:oak_sapling")]);
    h.receive(1, &click([2, 65, 0], 1, None));
    let shown = all::<UpdateSubchunkBlocks>(h.tick_all().to(1));
    assert!(!shown.is_empty() && shown.iter().all(|packet| packet.blocks.iter().all(|cell| cell.position.x.div_euclid(16) * 16 == packet.x && cell.position.y.div_euclid(16) * 16 == packet.y)));
    assert_eq!(names(&mut h, vec![[2, 65, 0], [2, 66, 0]]), ["minecraft:oak_log", "minecraft:oak_log"]);
}

#[test]
fn a_dark_oak_needs_four_saplings_and_grows_from_their_corner() {
    let mut h = planted(&[([2, 65, 0], "minecraft:dark_oak_sapling")]);
    h.receive(1, &click([2, 65, 0], 1, None));
    h.tick_all();
    assert_eq!(names(&mut h, vec![[2, 65, 0]]), ["minecraft:dark_oak_sapling"], "one alone takes no bone meal");

    let square: &[([i32; 3], &str)] = &[([2, 65, 0], "minecraft:dark_oak_sapling"), ([3, 65, 0], "minecraft:dark_oak_sapling"), ([2, 65, 1], "minecraft:dark_oak_sapling"), ([3, 65, 1], "minecraft:dark_oak_sapling")];
    let mut h = planted(square);
    h.receive(1, &click([3, 65, 1], 1, None));
    h.tick_all();
    assert_eq!(names(&mut h, vec![[2, 65, 0], [3, 65, 0], [2, 65, 1], [3, 65, 1]]), ["minecraft:dark_oak_log"; 4]);
}

#[test]
fn an_oak_sapling_ages_and_then_grows_by_random_ticks() {
    let mut h = planted(&[([2, 65, 0], "minecraft:oak_sapling")]);
    h.remote.run(|ctx| assert!(ctx.set_game_rule("randomTickSpeed", sapling_api::RuleValue::Int(1000))));
    let grown = (0..60).any(|_| {
        (0..50).for_each(|_| drop(h.tick_all()));
        names(&mut h, vec![[2, 65, 0]])[0] == "minecraft:oak_log"
    });
    assert!(grown, "a tree within 3000 ticks at a thousand times the speed");
}
