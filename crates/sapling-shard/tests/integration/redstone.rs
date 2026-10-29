//! Redstone against research/captures/bds-redstone.txt (harness: common/mod.rs).

use crate::common;

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
    let together: Vec<([i32; 3], u32, u32)> = all::<acacia_proto::packets::UpdateSubchunkBlocks>(&placed).iter().flat_map(|update| update.blocks.iter().map(|cell| ([cell.position.x, cell.position.y, cell.position.z], cell.runtime_id, cell.flags))).collect();
    let halves = [([10, 65, 8], state("minecraft:iron_door", &lower), 3), ([10, 66, 8], state("minecraft:iron_door", &upper), 2)];
    assert_eq!(together, halves, "both halves in one update, the upper with flags 2, as /setblock on BDS (bds-held-use.txt)");

    // Powered at the upper half: both halves in one update, the sound from the upper's middle naming
    // the lower half as it was (research/captures/bds-rsparts.txt).
    put(&mut h, &[([11, 66, 8], "redstone_block", "")]);
    let opened: Vec<RawPacket> = watch(&mut h, 4).concat();
    let together: Vec<([i32; 3], u32)> = all::<acacia_proto::packets::UpdateSubchunkBlocks>(&opened).iter().flat_map(|update| update.blocks.iter().map(|cell| ([cell.position.x, cell.position.y, cell.position.z], cell.runtime_id))).collect();
    assert_eq!(together, [([10, 65, 8], state("minecraft:iron_door", &door(0, 1))), ([10, 66, 8], state("minecraft:iron_door", &upper))]);
    let sounds: Vec<(String, [f32; 3], i32)> = all::<LevelSoundEvent>(&opened).iter().map(|sound| (sound.sound_id.clone(), [sound.position.x, sound.position.y, sound.position.z], sound.extra_data)).collect();
    assert_eq!(sounds, [("door.open".to_owned(), [10.5, 66.5, 8.5], state("minecraft:iron_door", &lower) as i32)]);

    put(&mut h, &[([11, 66, 8], "air", "")]);
    let shut: Vec<RawPacket> = watch(&mut h, 4).concat();
    let sounds: Vec<(String, i32)> = all::<LevelSoundEvent>(&shut).iter().map(|sound| (sound.sound_id.clone(), sound.extra_data)).collect();
    assert_eq!(sounds, [("door.close".to_owned(), state("minecraft:iron_door", &door(0, 1)) as i32)]);
}

#[test]
fn ice_takes_no_power_where_packed_and_blue_ice_do() {
    // research/captures/bds-rsparts.txt: dust into each, a lamp past it.
    let mut h = ready();
    let mut rigs = Vec::new();
    for (index, block) in ["ice", "packed_ice", "blue_ice"].into_iter().enumerate() {
        let x = -6 + 2 * index as i32;
        rigs.extend([([x, 65, 5], "redstone_wire", ""), ([x, 65, 6], block, ""), ([x, 65, 7], "redstone_lamp", ""), ([x, 65, 4], "redstone_block", "")]);
    }
    put(&mut h, &rigs);
    let shown = watch(&mut h, 5);
    let lit: Vec<bool> = (0..3).map(|index| when(&shown, [-6 + 2 * index, 65, 7], hash("minecraft:lit_redstone_lamp")).is_some()).collect();
    assert_eq!(lit, [false, true, true]);
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

#[test]
fn a_circuit_a_stored_column_holds_works_once_the_column_is_loaded() {
    let dust_line = [([2, 64, 3], "minecraft:redstone_block"), ([3, 64, 3], "minecraft:redstone_wire"), ([4, 64, 3], "minecraft:redstone_lamp")];
    let storage = common::stored::floor_holding(Vec::new(), &dust_line);
    let mut h = Harness::with(common::config(), sapling_world::World::with_storage(storage));
    common::stored::enter_stored(&mut h, 1);
    (0..6).for_each(|_| drop(h.tick(1)));
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send([[3, 64, 3], [4, 64, 3]].map(|pos| (ctx.power_at(pos), ctx.block(pos).map(|block| block.name)))).unwrap());
    h.tick(1);
    assert_eq!(rx.recv().unwrap(), [(15, Some("minecraft:redstone_wire".to_owned())), (15, Some("minecraft:lit_redstone_lamp".to_owned()))]);
}

fn repeater(lit: bool, input: &str, delay: u8) -> u32 {
    state(if lit { "minecraft:powered_repeater" } else { "minecraft:unpowered_repeater" }, &format!("minecraft:cardinal_direction={input},repeater_delay={delay}"))
}

#[test]
fn a_repeater_passes_power_from_its_back_after_its_delay_and_its_dust_shows_it_an_evaluation_later() {
    // As research/captures/bds-redstone-repeaters.txt, the rigs at y 0: lines eastward, the last with its back to the north.
    let mut h = ready();
    let mut lines = Vec::new();
    for (z, input, delay) in [(0, "west", "0"), (2, "west", "2"), (4, "north", "0")] {
        let properties: &str = Box::leak(format!("minecraft:cardinal_direction={input},repeater_delay={delay}").into_boxed_str());
        lines.extend([([-2, 65, z], "redstone_wire", ""), ([-1, 65, z], "unpowered_repeater", properties), ([0, 65, z], "redstone_wire", ""), ([1, 65, z], "redstone_lamp", "")]);
        lines.push(([-3, 65, z], "redstone_block", ""));
    }
    put(&mut h, &lines);
    let on = watch(&mut h, 12);
    let fed = when(&on, [-2, 65, 0], dust(15)).expect("the dust behind");
    assert_eq!(when(&on, [-1, 65, 0], repeater(true, "west", 0)), Some(fed), "no delay: with its dust");
    assert_eq!(when(&on, [1, 65, 0], hash("minecraft:lit_redstone_lamp")), Some(fed), "the lamp with the repeater");
    assert_eq!(when(&on, [0, 65, 0], dust(15)), Some(fed + 2), "the dust past it an evaluation later");
    assert_eq!(when(&on, [-1, 65, 2], repeater(true, "west", 2)), Some(fed + 4), "two more evaluations");
    assert_eq!(when(&on, [1, 65, 2], hash("minecraft:lit_redstone_lamp")), Some(fed + 4));
    assert_eq!(when(&on, [1, 65, 4], hash("minecraft:lit_redstone_lamp")), None, "power at its side");

    put(&mut h, &[([-3, 65, 0], "air", "")]);
    let off = watch(&mut h, 10);
    let unfed = when(&off, [-2, 65, 0], dust(0)).expect("the dust behind");
    assert_eq!(when(&off, [-1, 65, 0], repeater(false, "west", 0)), Some(unfed));
    assert_eq!(when(&off, [1, 65, 0], hash("minecraft:redstone_lamp")), Some(unfed + 4));
}

#[test]
fn a_lit_repeater_at_the_side_of_another_holds_it_as_it_is() {
    let mut h = ready();
    let (main, lamp) = ([0, 65, 2], [1, 65, 2]);
    put(&mut h, &[
        ([-1, 65, 2], "redstone_wire", ""),
        (main, "unpowered_repeater", "minecraft:cardinal_direction=west,repeater_delay=0"),
        (lamp, "redstone_lamp", ""),
        ([0, 65, 1], "unpowered_repeater", "minecraft:cardinal_direction=north,repeater_delay=0"),
        ([-2, 65, 2], "redstone_block", ""),
    ]);
    assert!(when(&watch(&mut h, 6), main, repeater(true, "west", 0)).is_some());
    put(&mut h, &[([0, 65, 0], "redstone_block", "")]);
    watch(&mut h, 6);
    put(&mut h, &[([-2, 65, 2], "air", "")]);
    assert_eq!(when(&watch(&mut h, 8), main, repeater(false, "west", 0)), None, "held lit with nothing behind it");
    put(&mut h, &[([0, 65, 0], "air", "")]);
    let freed = watch(&mut h, 10);
    let unlocked = when(&freed, [0, 65, 1], repeater(false, "north", 0)).expect("the one at its side goes out");
    assert_eq!(when(&freed, main, repeater(false, "west", 0)), Some(unlocked + 2), "an evaluation after, as on BDS");
}

#[test]
fn a_click_gives_a_repeater_its_next_delay() {
    let mut h = ready();
    let pos = [2, 65, 2];
    put(&mut h, &[(pos, "unpowered_repeater", "minecraft:cardinal_direction=west,repeater_delay=2")]);
    watch(&mut h, 2);
    for delay in [3, 0, 1] {
        h.receive(1, &common::click(pos, 1, None));
        assert!(block_changes(h.tick_all().to(2)).contains(&(pos, repeater(false, "west", delay))), "{delay}");
    }
}

fn comparator(lit: bool, subtracts: bool) -> u32 {
    let properties = format!("minecraft:cardinal_direction=west,output_lit_bit={},output_subtract_bit={}", u8::from(lit), u8::from(subtracts));
    state(if lit { "minecraft:powered_comparator" } else { "minecraft:unpowered_comparator" }, &properties)
}

#[test]
fn a_comparator_passes_what_is_behind_it_unless_a_side_is_stronger_or_less_its_side() {
    // As research/captures/bds-redstone-comparators.txt, the rigs at y 5 and 10.
    let mut h = ready();
    let mut rigs: Vec<([i32; 3], &str, &str)> = (-5..=-1).map(|x| ([x, 65, 2], "redstone_wire", "")).collect();
    rigs.extend([([0, 65, 2], "unpowered_comparator", "minecraft:cardinal_direction=west"), ([1, 65, 2], "redstone_wire", ""), ([-6, 65, 2], "redstone_block", "")]);
    rigs.extend([([4, 65, 2], "unpowered_comparator", "minecraft:cardinal_direction=west,output_subtract_bit=1"), ([5, 65, 2], "redstone_wire", ""), ([3, 65, 2], "redstone_block", "")]);
    rigs.extend((3..=5).map(|z| ([4, 65, z], "redstone_wire", "")));
    put(&mut h, &rigs);
    let on = watch(&mut h, 8);
    let lit = when(&on, [0, 65, 2], comparator(true, false)).expect("lit");
    assert_eq!(when(&on, [-1, 65, 2], dust(11)), Some(lit), "with the dust behind it");
    assert_eq!(when(&on, [1, 65, 2], dust(11)), Some(lit + 2), "the dust past it an evaluation later, as strong");
    assert_eq!(when(&on, [5, 65, 2], dust(15)), Some(lit + 2), "nothing at the side yet");

    // A block of redstone at the side is no side input (research/captures/bds-rsparts.txt); dust from it is.
    put(&mut h, &[([0, 65, 1], "redstone_block", "")]);
    assert_eq!(when(&watch(&mut h, 8), [0, 65, 2], comparator(false, false)), None, "a block at the side");
    put(&mut h, &[([0, 65, 1], "redstone_wire", ""), ([0, 65, 0], "redstone_block", ""), ([4, 65, 6], "redstone_block", "")]);
    let sided = watch(&mut h, 8);
    let out = when(&sided, [0, 65, 2], comparator(false, false)).expect("a stronger side puts it out");
    assert_eq!(when(&sided, [1, 65, 2], dust(0)), Some(out + 2));
    assert!(when(&sided, [4, 65, 3], dust(13)).is_some());
    assert_eq!(when(&sided, [5, 65, 2], dust(2)), Some(out + 2), "15 less 13");
}

#[test]
fn a_comparator_reads_how_full_the_chest_behind_it_is() {
    let mut h = ready();
    put(&mut h, &[([-4, 65, 6], "chest", ""), ([-3, 65, 6], "unpowered_comparator", "minecraft:cardinal_direction=west"), ([-2, 65, 6], "redstone_wire", "")]);
    watch(&mut h, 4);
    h.remote.run(|ctx| (0..14).for_each(|slot| assert!(ctx.set_container_item([-4, 65, 6], slot, Some(&sapling_api::Item::new("minecraft:stone", 64))))));
    let filled = watch(&mut h, 8);
    assert!(when(&filled, [-3, 65, 6], comparator(true, false)).is_some());
    assert!(when(&filled, [-2, 65, 6], dust(8)).is_some(), "14 of 27 stacks, as on BDS");
    h.remote.run(|ctx| (0..14).for_each(|slot| assert!(ctx.set_container_item([-4, 65, 6], slot, None))));
    assert!(when(&watch(&mut h, 8), [-2, 65, 6], dust(0)).is_some());
}

#[test]
fn a_comparator_reads_a_cake_and_a_composter_by_their_state() {
    // As research/captures/bds-redstone-reads.txt.
    let mut h = ready();
    let west = "minecraft:cardinal_direction=west";
    put(&mut h, &[([-4, 65, 6], "cake", ""), ([-3, 65, 6], "unpowered_comparator", west), ([-2, 65, 6], "redstone_wire", "")]);
    put(&mut h, &[([-4, 65, 2], "composter", ""), ([-3, 65, 2], "unpowered_comparator", west), ([-2, 65, 2], "redstone_wire", "")]);
    assert!(when(&watch(&mut h, 8), [-2, 65, 6], dust(14)).is_some(), "a whole cake");
    put(&mut h, &[([-4, 65, 6], "cake", "bite_counter=3"), ([-4, 65, 2], "composter", "composter_fill_level=5")]);
    let changed = watch(&mut h, 8);
    assert!(when(&changed, [-2, 65, 6], dust(8)).is_some(), "three bites");
    assert!(when(&changed, [-2, 65, 2], dust(5)).is_some(), "a composter filled to 5");
    put(&mut h, &[([-4, 65, 6], "dispenser", "")]);
    assert!(when(&watch(&mut h, 8), [-2, 65, 6], dust(0)).is_some(), "the cake gone");
    h.remote.run(|ctx| assert!(ctx.set_container_item([-4, 65, 6], 0, Some(&sapling_api::Item::new("minecraft:stone", 64)))));
    assert!(when(&watch(&mut h, 8), [-2, 65, 6], dust(2)).is_some(), "a stack in a dispenser's 9 slots");
}

#[test]
fn a_click_turns_a_comparator_to_subtracting_and_back() {
    let mut h = ready();
    let pos = [2, 65, 2];
    put(&mut h, &[(pos, "unpowered_comparator", "minecraft:cardinal_direction=west")]);
    watch(&mut h, 2);
    // research/captures/bds-rsreads.txt: `power.off` naming the state it had as it starts subtracting.
    for (subtracts, sound) in [(true, "power.off"), (false, "power.on")] {
        h.receive(1, &common::click(pos, 1, None));
        let sent = h.tick_all().to(2).to_vec();
        assert!(block_changes(&sent).contains(&(pos, comparator(false, subtracts))), "{subtracts}");
        let heard: Vec<(String, i32)> = all::<LevelSoundEvent>(&sent).iter().map(|heard| (heard.sound_id.clone(), heard.extra_data)).collect();
        assert_eq!(heard, [(sound.to_owned(), comparator(false, !subtracts) as i32)]);
    }
}

#[test]
fn an_observer_pulses_2_ticks_after_the_cell_it_watches_changes_and_another_watching_it_follows() {
    // As research/captures/bds-redstone-observers.txt: watching west, powering east.
    let mut h = ready();
    let observer = |lit: bool| state("minecraft:observer", &format!("minecraft:facing_direction=west,powered_bit={}", u8::from(lit)));
    put(&mut h, &[
        ([0, 65, 2], "observer", "minecraft:facing_direction=west"),
        ([1, 65, 2], "redstone_wire", ""),
        ([2, 65, 2], "redstone_lamp", ""),
        ([0, 65, 5], "observer", "minecraft:facing_direction=west"),
        ([1, 65, 5], "observer", "minecraft:facing_direction=west"),
    ]);
    assert_eq!(when(&watch(&mut h, 8), [0, 65, 2], observer(true)), None, "nothing for being put");

    put(&mut h, &[([-1, 65, 2], "stone", ""), ([-1, 65, 5], "stone", "")]);
    let seen = watch(&mut h, 16);
    assert_eq!((when(&seen, [0, 65, 2], observer(true)), when(&seen, [0, 65, 2], observer(false))), (Some(2), Some(4)));
    let lit = when(&seen, [2, 65, 2], hash("minecraft:lit_redstone_lamp")).expect("the lamp past its dust");
    assert!((3..=4).contains(&lit), "at the evaluation after: {lit}");
    assert_eq!(when(&seen, [1, 65, 2], dust(15)), Some(lit + 2), "the dust an evaluation after the lamp");
    assert_eq!(when(&seen, [1, 65, 2], dust(0)), Some(lit + 4));
    assert_eq!((when(&seen, [1, 65, 5], observer(true)), when(&seen, [1, 65, 5], observer(false))), (Some(4), Some(6)), "by the first one turning on");
}
