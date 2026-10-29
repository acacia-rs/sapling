//! Note blocks, copper bulbs, targets, tripwire and sculk sensors in circuits
//! (docs/research/redstone-blocks.md, docs/research/sculk.md).

use crate::common;

use std::sync::mpsc;

use acacia_proto::RawPacket;
use acacia_proto::packets::{BlockEvent, LevelSoundEvent};
use common::{Harness, all, click};
use sapling_api::{Block, Item, PlayerId};

const ALEX: PlayerId = PlayerId(1);

fn ready(blocks: Vec<([i32; 3], Block)>) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        for (pos, block) in &blocks {
            assert!(ctx.update_block(*pos, block), "{block:?}");
        }
    });
    ticks(&mut h, 4);
    h
}

fn ticks(h: &mut Harness, n: usize) -> Vec<RawPacket> {
    (0..n).flat_map(|_| h.tick_all().to(1).to_vec()).collect()
}

fn put(h: &mut Harness, pos: [i32; 3], name: &str) {
    let block = Block::new(name);
    h.remote.run(move |ctx| assert!(ctx.update_block(pos, &block)));
}

fn properties(h: &mut Harness, pos: [i32; 3]) -> String {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.block(pos).map(|block| block.properties).unwrap_or_default()).unwrap());
    h.tick_all();
    rx.recv().unwrap()
}

/// The signal of the dust at `pos`.
fn dust(h: &mut Harness, pos: [i32; 3]) -> u8 {
    properties(h, pos).strip_prefix("redstone_signal=").and_then(|signal| signal.parse().ok()).expect("dust")
}

#[test]
fn a_note_block_keeps_its_note_and_plays_it_on_a_rising_signal() {
    let mut h = ready(vec![([3, 65, 6], Block::new("noteblock"))]);
    for _ in 0..3 {
        h.receive(1, &click([3, 65, 6], 1, None));
        ticks(&mut h, 1);
    }
    put(&mut h, [3, 65, 7], "redstone_block");
    let played = ticks(&mut h, 6);
    let notes: Vec<i32> = all::<BlockEvent>(&played).iter().map(|event| event.data).collect();
    assert_eq!(notes, [3], "the third note, once");
    assert!(all::<LevelSoundEvent>(&played).iter().any(|sound| sound.sound_id == "note" && sound.extra_data == 1 << 8 | 3), "over stone: the bass drum");
    assert!(ticks(&mut h, 6).iter().all(|packet| all::<BlockEvent>(std::slice::from_ref(packet)).is_empty()), "a held signal plays no more");
}

#[test]
fn a_copper_bulb_turns_on_and_off_with_each_rise() {
    let mut h = ready(vec![([3, 65, 6], Block::new("copper_bulb"))]);
    put(&mut h, [3, 65, 7], "redstone_block");
    let heard: Vec<(String, [f32; 3])> = ticks(&mut h, 6).iter().flat_map(|packet| all::<acacia_proto::packets::LevelSoundEvent>(std::slice::from_ref(packet))).map(|sound| (sound.sound_id, [sound.position.x, sound.position.y, sound.position.z])).collect();
    assert_eq!(heard, [("block.copper_bulb.turn_on".to_owned(), [3.0, 65.0, 6.0])], "at the corner (research/captures/bds-rsdevices.txt)");
    assert_eq!(properties(&mut h, [3, 65, 6]), "lit=1,powered_bit=1");
    put(&mut h, [3, 65, 7], "air");
    ticks(&mut h, 6);
    assert_eq!(properties(&mut h, [3, 65, 6]), "lit=1,powered_bit=0", "it stays lit");
    put(&mut h, [3, 65, 7], "redstone_block");
    ticks(&mut h, 6);
    assert_eq!(properties(&mut h, [3, 65, 6]), "lit=0,powered_bit=1");
    let comparator = Block::new("unpowered_comparator").with("minecraft:cardinal_direction=west");
    let mut h = ready(vec![([3, 65, 6], Block::new("copper_bulb").with("lit=1")), ([4, 65, 6], comparator), ([5, 65, 6], Block::new("redstone_wire"))]);
    ticks(&mut h, 6);
    assert_eq!(dust(&mut h, [5, 65, 6]), 15, "a comparator reads 15 off a lit bulb");
}

#[test]
fn a_target_hit_by_an_arrow_gives_a_signal_for_a_while() {
    let mut h = ready(vec![([3, 65, 6], Block::new("dispenser").with("facing_direction=5")), ([6, 65, 6], Block::new("target")), ([6, 65, 5], Block::new("redstone_wire"))]);
    h.remote.run(|ctx| assert!(ctx.set_container_item([3, 65, 6], 0, Some(&Item::new("minecraft:arrow", 1)))));
    put(&mut h, [3, 65, 7], "redstone_block");
    let mut strongest = 0;
    for _ in 0..16 {
        strongest = strongest.max(dust(&mut h, [6, 65, 5]));
    }
    assert!(strongest >= 8, "near the middle: {strongest}");
    ticks(&mut h, 30);
    assert_eq!(dust(&mut h, [6, 65, 5]), 0, "an arrow's signal lasts 20 ticks");
}

#[test]
fn a_player_on_a_tripwire_powers_both_hooks() {
    let hook = |direction: u8| Block::new("tripwire_hook").with(&format!("attached_bit=0,direction={direction},powered_bit=0"));
    let mut blocks = vec![([5, 65, 1], Block::new("stone")), ([5, 65, 2], hook(0)), ([5, 65, 6], hook(2)), ([5, 65, 7], Block::new("stone"))];
    blocks.extend((3..=5).map(|z| ([5, 65, z], Block::new("trip_wire"))));
    let mut h = ready(blocks);
    assert!(properties(&mut h, [5, 65, 2]).contains("attached_bit=1"), "the hooks hold the line");
    h.remote.run(|ctx| ctx.teleport(ALEX, [5.5, 65.0, 4.5]));
    ticks(&mut h, 4);
    for hook in [[5, 65, 2], [5, 65, 6]] {
        assert!(properties(&mut h, hook).contains("powered_bit=1"), "{hook:?}: {}", properties(&mut h, hook));
    }
    h.remote.run(|ctx| ctx.teleport(ALEX, [0.5, 65.0, 0.5]));
    ticks(&mut h, 24);
    assert!(properties(&mut h, [5, 65, 2]).contains("powered_bit=0"), "let go once nothing is on it");
}

#[test]
fn a_sculk_sensor_hears_a_trapdoor_and_powers_by_its_distance_then_cools_down() {
    let mut h = ready(vec![([2, 65, -2], Block::new("trapdoor")), ([6, 65, -2], Block::new("sculk_sensor")), ([7, 65, -2], Block::new("redstone_wire"))]);
    h.receive(1, &click([2, 65, -2], 1, None));
    let heard = ticks(&mut h, 8);
    assert!(all::<LevelSoundEvent>(&heard).iter().any(|sound| sound.sound_id == "power.on.sculk_sensor"));
    assert!(properties(&mut h, [6, 65, -2]).contains("sculk_sensor_phase=1"));
    assert_eq!(dust(&mut h, [7, 65, -2]), 15 - 15 * 4 / 8, "four blocks off of eight");
    ticks(&mut h, 45);
    assert!(properties(&mut h, [6, 65, -2]).contains("sculk_sensor_phase=0"), "active, cooled down and idle again");
}
