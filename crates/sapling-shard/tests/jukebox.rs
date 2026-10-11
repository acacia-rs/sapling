//! A jukebox and the comparator behind it, against research/captures/bds-discs.txt, and lit
//! redstone ore going out, against bds-ores.txt.

mod common;

use acacia_proto::packets::{AddItemEntity, LevelEvent, LevelSoundEvent, Text, TextType};
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

#[test]
fn an_item_frame_takes_an_item_and_turns_it_and_a_decorated_pot_takes_a_stack() {
    // research/captures/bds-clicks2.txt.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&Item::new("minecraft:stone", 5))));
        assert!(ctx.set_block([2, 65, 0], &Block { name: "minecraft:frame".into(), properties: "facing_direction=1".into() }));
        assert!(ctx.set_block([2, 65, 2], &Block::new("minecraft:decorated_pot")));
    });
    h.tick_all();
    let mut heard = Vec::new();
    for at in [[2, 65, 0], [2, 65, 0], [2, 65, 2], [2, 65, 2]] {
        h.receive(1, &click(at, 1, None));
        let sent = h.tick_all().to(1).to_vec();
        heard.extend(all::<LevelEvent>(&sent).into_iter().filter(|event| event.position.x == 2.0).map(|event| format!("{:?}", event.event)));
        heard.extend(all::<LevelSoundEvent>(&sent).into_iter().filter(|sound| sound.sound_id.contains("pot")).map(|sound| format!("{} {}", sound.sound_id, sound.extra_data)));
    }
    assert_eq!(heard, ["SoundItemframeAddItem", "SoundItemframeRotateItem", "block.decorated_pot.insert 707", "block.decorated_pot.insert 715"]);
    h.remote.run(|ctx| {
        assert_eq!(ctx.item(ALEX, Slot::Main(0)).map(|held| held.count), Some(2), "one in the frame, two in the pot");
        assert!(ctx.block([2, 66, 0]).is_some_and(|over| over.name == "minecraft:air"), "no stone was put against the frame");
    });
    h.tick_all();
}
