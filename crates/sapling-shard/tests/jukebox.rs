//! A jukebox and the comparator behind it, against research/captures/bds-discs.txt, and lit
//! redstone ore going out, against bds-ores.txt.

mod common;

use acacia_proto::packets::{AddItemEntity, Text, TextType};
use common::{Harness, all, click};
use sapling_api::{Block, GameMode, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);

/// The dust's signal, after the circuit settles.
fn dust(h: &mut Harness) -> String {
    for _ in 0..6 {
        h.tick_all();
    }
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(ctx.block([4, 65, 0]).map(|dust| dust.properties).unwrap_or_default()).expect("the test waits"));
    h.tick_all();
    got.recv().expect("ran")
}

#[test]
fn a_disc_goes_in_with_its_title_powers_the_comparator_and_comes_out() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(ALEX, GameMode::Creative);
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:music_disc_5", 1))));
        assert!(ctx.update_block([2, 65, 0], &Block::new("minecraft:jukebox")));
        assert!(ctx.update_block([3, 65, 0], &Block { name: "minecraft:unpowered_comparator".into(), properties: "minecraft:cardinal_direction=west".into() }));
        assert!(ctx.update_block([4, 65, 0], &Block::new("minecraft:redstone_wire")));
    });
    assert_eq!(dust(&mut h), "redstone_signal=0");
    h.receive(1, &click([2, 65, 0], 1, None));
    let told = all::<Text>(h.tick_all().to(1));
    assert!(told.iter().any(|text| text.r#type == TextType::JukeboxPopup), "{told:?}");
    assert_eq!(dust(&mut h), "redstone_signal=15", "BDS's read 15 of the disc 5");
    h.receive(1, &click([2, 65, 0], 1, None));
    let out = all::<AddItemEntity>(h.tick_all().to(1));
    assert_eq!(out.len(), 1, "the disc back");
    assert_eq!(dust(&mut h), "redstone_signal=0");
}

#[test]
fn lit_redstone_ore_goes_out_by_itself() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        for x in 2..=9 {
            assert!(ctx.set_block([x, 65, 3], &Block::new("minecraft:lit_redstone_ore")));
        }
    });
    let lit = |h: &mut Harness, ticks: usize| {
        for _ in 0..ticks {
            h.tick_all();
        }
        let (sent, got) = std::sync::mpsc::channel();
        h.remote.run(move |ctx| sent.send((2..=9).filter(|x| ctx.block([*x, 65, 3]).is_some_and(|ore| ore.name == "minecraft:lit_redstone_ore")).count()).expect("the test waits"));
        h.tick_all();
        got.recv().expect("ran")
    };
    // BDS's lasted 88 seconds on average: some of eight are out after a minute, all after eight.
    let (soon, late) = (lit(&mut h, 1200), lit(&mut h, 8000));
    assert!((1..8).contains(&soon) && late == 0, "{soon} lit after a minute, {late} after eight");
    h.remote.run(|ctx| assert_eq!(ctx.block([2, 65, 3]).map(|ore| ore.name).as_deref(), Some("minecraft:redstone_ore")));
    h.tick_all();
}
