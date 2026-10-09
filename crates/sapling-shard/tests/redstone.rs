//! Redstone against research/captures/bds-redstone.txt (harness: common/mod.rs).

mod common;

use std::sync::mpsc;

use acacia_proto::RawPacket;
use acacia_proto::packets::LevelSoundEvent;
use common::{Harness, all, block_changes, hash};
use sapling_api::Block;

fn state(name: &str, properties: &str) -> u32 {
    let registry = sapling_world::BlockRegistry::vanilla();
    registry.network_hash(registry.find(name, properties).unwrap_or_else(|| panic!("{name} [{properties}]")))
}

fn dust(signal: u8) -> u32 {
    state("minecraft:redstone_wire", &format!("redstone_signal={signal}"))
}

fn ready() -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h
}

/// Puts `blocks` as a handler's `update_block` does, in the next tick: the first that [`watch`] sees.
fn put(h: &mut Harness, blocks: &[([i32; 3], &str, &str)]) {
    let blocks: Vec<([i32; 3], Block)> = blocks.iter().map(|(pos, name, properties)| (*pos, Block::new(*name).with(*properties))).collect();
    h.remote.run(move |ctx| {
        for (pos, block) in &blocks {
            assert!(ctx.update_block(*pos, block), "{pos:?} {block:?}");
        }
    });
}

/// The changes Steve is shown in each of the next `ticks` ticks.
fn watch(h: &mut Harness, ticks: usize) -> Vec<Vec<RawPacket>> {
    (0..ticks).map(|_| h.tick_all().to(2).to_vec()).collect()
}

/// The first of `ticks` in which `pos` became `block`.
fn when(ticks: &[Vec<RawPacket>], pos: [i32; 3], block: u32) -> Option<usize> {
    ticks.iter().position(|sent| block_changes(sent).contains(&(pos, block)))
}

#[test]
fn dust_carries_a_block_of_redstone_15_blocks_to_a_lamp_and_the_lamp_goes_out_4_ticks_after_it() {
    let mut h = ready();
    // The platform ends at x 7: stone carries the lines on.
    let mut line: Vec<([i32; 3], &str, &str)> = (8..=10).flat_map(|x| [([x, 64, 4], "stone", ""), ([x, 64, 6], "stone", "")]).collect();
    line.extend((-6..=8).map(|x| ([x, 65, 4], "redstone_wire", "")));
    line.push(([9, 65, 4], "redstone_lamp", ""));
    // A 16th dust leaves the second lamp dark.
    line.extend((-6..=9).map(|x| ([x, 65, 6], "redstone_wire", "")));
    line.push(([10, 65, 6], "redstone_lamp", ""));
    put(&mut h, &line);
    watch(&mut h, 4);

    put(&mut h, &[([-7, 65, 4], "redstone_block", ""), ([-7, 65, 6], "redstone_block", "")]);
    let on = watch(&mut h, 6);
    let lit = when(&on, [9, 65, 4], hash("minecraft:lit_redstone_lamp")).unwrap_or_else(|| panic!("the lamp lit: {:?}", on.iter().map(|sent| block_changes(sent)).collect::<Vec<_>>()));
    assert!((2..=3).contains(&lit), "2 or 3 ticks after, by the 2-tick phase, as BDS: {lit}");
    let shown = block_changes(&on[lit]);
    for (x, signal) in [(-6, 15), (1, 8), (8, 1)] {
        assert!(shown.contains(&([x, 65, 4], dust(signal))), "dust at {x} with the lamp: {signal}");
    }
    assert_eq!(when(&on, [10, 65, 6], hash("minecraft:lit_redstone_lamp")), None, "16 dust are too many");
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send([[-6, 65, 4], [8, 65, 4], [9, 65, 4], [10, 65, 6]].map(|pos| ctx.power_at(pos))).unwrap());
    h.tick_all();
    assert_eq!(rx.recv().unwrap(), [15, 1, 1, 0]);

    put(&mut h, &[([-7, 65, 4], "air", "")]);
    let off = watch(&mut h, 10);
    let unpowered = when(&off, [-6, 65, 4], dust(0)).expect("the dust lost its signal");
    assert_eq!(when(&off, [9, 65, 4], hash("minecraft:redstone_lamp")), Some(unpowered + 4), "BDS: 27632 -> 27835");
}

#[test]
fn a_lever_powers_a_lamp_through_its_block_and_puts_out_a_torch_on_it() {
    let mut h = ready();
    put(&mut h, &[
        ([3, 65, 8], "stone", ""),
        ([4, 65, 8], "redstone_lamp", ""),
        ([7, 65, 8], "stone", ""),
        ([7, 66, 8], "redstone_torch", "torch_facing_direction=top"),
        ([7, 67, 8], "redstone_lamp", ""),
    ]);
    let placed = watch(&mut h, 4);
    let shown: Vec<_> = placed.iter().map(|sent| block_changes(sent)).collect();
    assert!(when(&placed, [7, 67, 8], hash("minecraft:lit_redstone_lamp")).is_some(), "a torch lights the lamp over it: {shown:?}");

    put(&mut h, &[([3, 66, 8], "lever", "lever_direction=up_north_south,open_bit=1"), ([6, 65, 8], "lever", "lever_direction=west,open_bit=1")]);
    let thrown = watch(&mut h, 12);
    assert!(when(&thrown, [4, 65, 8], hash("minecraft:lit_redstone_lamp")).is_some_and(|tick| (2..=3).contains(&tick)), "the lamp beside the lever's block");
    let out = when(&thrown, [7, 66, 8], state("minecraft:unlit_redstone_torch", "torch_facing_direction=top")).expect("the torch went out");
    assert_eq!(when(&thrown, [7, 67, 8], hash("minecraft:redstone_lamp")).map(|tick| tick > out), Some(true), "and then its lamp");
}

#[test]
fn a_powered_iron_door_opens_its_lower_half_with_a_sound_and_shuts_again() {
    let mut h = ready();
    let door = |upper: u8, open: u8| format!("door_hinge_bit=0,minecraft:cardinal_direction=south,open_bit={open},upper_block_bit={upper}");
    let (lower, upper) = (door(0, 0), door(1, 0));
    put(&mut h, &[([10, 65, 8], "iron_door", &lower)]);
    let placed: Vec<RawPacket> = watch(&mut h, 4).concat();
    assert!(block_changes(&placed).contains(&([10, 66, 8], state("minecraft:iron_door", &upper))), "its upper half comes with it, as /setblock on BDS");

    put(&mut h, &[([11, 65, 8], "redstone_block", "")]);
    let opened: Vec<RawPacket> = watch(&mut h, 4).concat();
    assert!(block_changes(&opened).contains(&([10, 65, 8], state("minecraft:iron_door", &door(0, 1)))), "the lower half opens");
    let sounds: Vec<String> = all::<LevelSoundEvent>(&opened).iter().map(|sound| format!("{} {:?}", sound.sound_id, sound.position)).collect();
    assert!(sounds.iter().any(|sound| sound.starts_with("door.open")), "{sounds:?}");

    put(&mut h, &[([11, 65, 8], "air", "")]);
    let shut: Vec<RawPacket> = watch(&mut h, 4).concat();
    assert!(block_changes(&shut).contains(&([10, 65, 8], state("minecraft:iron_door", &lower))));
}

#[test]
fn dust_powers_a_lamp_through_stone_soul_sand_and_a_double_slab_but_not_glass_leaves_or_a_chest() {
    let mut h = ready();
    let through = [("stone", true), ("soul_sand", true), ("smooth_stone_double_slab", true), ("glass", false), ("oak_leaves", false), ("chest", false), ("oak_fence", false)];
    let mut rigs = Vec::new();
    for (index, (block, _)) in through.iter().enumerate() {
        let x = -6 + 2 * index as i32;
        rigs.extend([([x, 65, 5], "redstone_wire", ""), ([x, 65, 6], *block, ""), ([x, 65, 7], "redstone_lamp", ""), ([x, 65, 4], "redstone_block", "")]);
    }
    put(&mut h, &rigs);
    let shown = watch(&mut h, 5);
    for (index, (block, lights)) in through.iter().enumerate() {
        let lamp = [-6 + 2 * index as i32, 65, 7];
        assert_eq!(when(&shown, lamp, hash("minecraft:lit_redstone_lamp")).is_some(), *lights, "{block}, as research/captures/bds-conductors.txt");
    }
}

#[test]
fn a_torch_powering_itself_flips_15_times_and_stays_out_then_relights_and_fizzes() {
    // As research/captures/bds-redstone-burnout.txt: on the side of a block, a block over it powering dust on the first.
    let mut h = ready();
    let torch = [1, 65, 8];
    let lit = state("minecraft:redstone_torch", "torch_facing_direction=west");
    put(&mut h, &[([0, 65, 8], "stone", ""), ([1, 66, 8], "stone", ""), ([0, 66, 8], "redstone_wire", ""), (torch, "redstone_torch", "torch_facing_direction=west")]);
    let flips = |ticks: &[Vec<RawPacket>]| -> Vec<(usize, bool)> {
        let of = |sent: &Vec<RawPacket>| block_changes(sent).into_iter().filter(|(pos, _)| *pos == torch).map(|(_, block)| block == lit).collect::<Vec<_>>();
        ticks.iter().enumerate().flat_map(|(tick, sent)| of(sent).into_iter().map(move |lit| (tick, lit))).collect()
    };
    let fizzes = |ticks: &[Vec<RawPacket>]| ticks.iter().flat_map(|sent| all::<LevelSoundEvent>(sent)).filter(|sound| sound.sound_id.starts_with("fizz")).count();
    let first = watch(&mut h, 80);
    let flipped: Vec<(usize, bool)> = flips(&first).into_iter().skip(1).collect();
    assert_eq!(flipped.len(), 15, "{flipped:?}");
    assert!(flipped.windows(2).all(|pair| pair[1].0 == pair[0].0 + 2 && pair[1].1 != pair[0].1), "every 2 ticks: {flipped:?}");
    assert_eq!((flipped[0].1, flipped[14].1, fizzes(&first)), (false, false, 0), "out without a fizz");
    assert!(block_changes(&first[flipped[0].0]).contains(&([0, 66, 8], dust(15))), "the dust shows the torch as it was");

    put(&mut h, &[([2, 65, 8], "stone", "")]);
    let again = watch(&mut h, 50);
    let flipped = flips(&again);
    assert_eq!((flipped.len(), flipped.first().map(|flip| flip.1), flipped.last().map(|flip| flip.1), fizzes(&again)), (16, Some(true), Some(false), 1), "{flipped:?}");
    put(&mut h, &[([2, 65, 8], "air", "")]);
    assert_eq!(flips(&watch(&mut h, 10)), [], "not within 34 ticks of burning out");
}
