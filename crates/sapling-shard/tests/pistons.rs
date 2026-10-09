//! Pistons against research/captures/bds-redstone-pistons.txt (harness: common/mod.rs).

mod common;

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


const EAST: &str = "facing_direction=4";

fn arm(sticky: bool) -> u32 {
    state(if sticky { "minecraft:sticky_piston_arm_collision" } else { "minecraft:piston_arm_collision" }, EAST)
}

fn sounds(ticks: &[Vec<RawPacket>], name: &str) -> Vec<usize> {
    ticks.iter().enumerate().filter(|(_, sent)| all::<LevelSoundEvent>(sent).iter().any(|sound| sound.sound_id == name)).map(|(tick, _)| tick).collect()
}

#[test]
fn a_powered_piston_pushes_a_block_over_4_ticks_and_draws_back_without_it() {
    let mut h = ready();
    put(&mut h, &[([0, 65, 2], "piston", EAST), ([1, 65, 2], "stone", "")]);
    watch(&mut h, 4);
    put(&mut h, &[([0, 66, 2], "redstone_block", "")]);
    let out = watch(&mut h, 10);
    let moved = when(&out, [2, 65, 2], hash("minecraft:moving_block")).expect("the stone goes as a moving block");
    assert!(block_changes(&out[moved]).contains(&([1, 65, 2], arm(false))), "with the arm where it was");
    assert_eq!(sounds(&out, "piston.out"), [moved + 1]);
    assert_eq!(when(&out, [2, 65, 2], hash("minecraft:stone")), Some(moved + 2), "a block again");

    put(&mut h, &[([0, 66, 2], "air", "")]);
    let back = watch(&mut h, 10);
    let gone = when(&back, [1, 65, 2], hash("minecraft:air")).expect("the arm goes");
    assert_eq!(sounds(&back, "piston.in"), [gone + 1]);
    assert_eq!(when(&back, [2, 65, 2], hash("minecraft:air")), None, "the stone stays where it was pushed");
}

#[test]
fn a_piston_pushes_12_blocks_and_not_13_or_obsidian_and_breaks_a_torch() {
    let mut h = ready();
    let mut rigs: Vec<([i32; 3], &str, &str)> = vec![([-7, 65, 0], "piston", EAST), ([-7, 65, 2], "piston", EAST), ([-7, 65, 4], "piston", EAST), ([-7, 65, 6], "piston", EAST)];
    rigs.extend((-6..=5).map(|x| ([x, 65, 0], "stone", "")));
    rigs.extend((-6..=6).map(|x| ([x, 65, 2], "stone", "")));
    rigs.extend([([-6, 65, 4], "obsidian", ""), ([-6, 65, 6], "redstone_torch", "")]);
    put(&mut h, &rigs);
    watch(&mut h, 4);
    put(&mut h, &[([-7, 66, 0], "redstone_block", ""), ([-7, 66, 2], "redstone_block", ""), ([-7, 66, 4], "redstone_block", ""), ([-7, 66, 6], "redstone_block", "")]);
    let out: Vec<RawPacket> = watch(&mut h, 10).concat();
    let changed = block_changes(&out);
    assert!(changed.contains(&([6, 65, 0], hash("minecraft:stone"))) && changed.contains(&([-6, 65, 0], arm(false))), "12 go");
    assert!(!changed.iter().any(|(pos, _)| pos[2] == 2 && pos[1] == 65), "13 stay: {changed:?}");
    assert!(!changed.iter().any(|(pos, _)| pos[2] == 4 && pos[1] == 65), "obsidian stays");
    assert!(changed.contains(&([-6, 65, 6], arm(false))) && !changed.iter().any(|(pos, _)| *pos == [-5, 65, 6]), "the torch is broken, not pushed");
}

#[test]
fn a_sticky_piston_brings_its_block_back_and_a_chest_goes_with_what_it_holds() {
    let mut h = ready();
    put(&mut h, &[
        ([0, 65, 2], "sticky_piston", EAST),
        ([1, 65, 2], "stone", ""),
        ([0, 65, 5], "piston", EAST),
        ([1, 65, 5], "chest", ""),
        ([3, 65, 5], "unpowered_comparator", "minecraft:cardinal_direction=west"),
        ([4, 65, 5], "redstone_wire", ""),
    ]);
    h.remote.run(|ctx| (0..14).for_each(|slot| assert!(ctx.set_container_item([1, 65, 5], slot, Some(&sapling_api::Item::new("minecraft:stone", 64))))));
    watch(&mut h, 4);
    put(&mut h, &[([0, 66, 2], "redstone_block", ""), ([0, 66, 5], "redstone_block", "")]);
    let out = watch(&mut h, 14);
    assert!(when(&out, [1, 65, 2], arm(true)).is_some() && when(&out, [2, 65, 2], hash("minecraft:stone")).is_some());
    assert!(when(&out, [4, 65, 5], dust(8)).is_some(), "the comparator reads the chest where it was pushed: 14 of 27 stacks");

    put(&mut h, &[([0, 66, 2], "air", "")]);
    let back = watch(&mut h, 10);
    let pulled = when(&back, [1, 65, 2], hash("minecraft:moving_block")).expect("the stone comes back as a moving block");
    assert_eq!((when(&back, [2, 65, 2], hash("minecraft:air")), when(&back, [1, 65, 2], hash("minecraft:stone"))), (Some(pulled), Some(pulled + 2)));
}
