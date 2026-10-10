//! Pistons against research/captures/bds-redstone-pistons.txt and bds-slime.txt (harness: common/mod.rs).

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

/// Puts `piston` at x 0 of the row `z`, 3 above the ground, with `blocks` by their place from it.
fn hung(z: i32, piston: &'static str, blocks: &[([i32; 2], &'static str)]) -> Vec<([i32; 3], &'static str, &'static str)> {
    [([0, 68, z], piston, EAST)].into_iter().chain(blocks.iter().map(|([x, up], name)| ([*x, 68 + up, z], *name, ""))).collect()
}

#[test]
fn slime_and_honey_take_what_they_touch_but_not_each_other_obsidian_or_glazed_terracotta() {
    let mut h = ready();
    let rigs = [
        hung(0, "piston", &[([1, 0], "slime"), ([2, 0], "stone"), ([1, 1], "stone"), ([1, -1], "stone")]),
        hung(3, "piston", &[([1, 0], "slime"), ([1, 1], "obsidian"), ([1, -1], "white_glazed_terracotta")]),
        hung(6, "piston", &[([1, 0], "slime"), ([1, 1], "honey_block"), ([1, 2], "stone")]),
        hung(9, "piston", &[([1, 0], "honey_block"), ([1, 1], "stone")]),
        hung(12, "piston", &[([1, 0], "slime"), ([1, 1], "stone"), ([2, 1], "obsidian")]),
        hung(15, "piston", &[([1, 0], "slime"), ([1, 1], "slime"), ([1, 2], "stone"), ([2, 1], "stone")]),
    ];
    put(&mut h, &rigs.concat());
    watch(&mut h, 4);
    put(&mut h, &[0, 3, 6, 9, 12, 15].map(|z| ([0, 67, z], "redstone_block", "")));
    let changed = block_changes(&watch(&mut h, 10).concat());
    let went = |x: i32, up: i32, z: i32, name: &str| changed.contains(&([x, 68 + up, z], hash(&format!("minecraft:{name}"))));
    let stayed = |up: i32, z: i32| !changed.iter().any(|(pos, _)| pos[0] >= 1 && pos[1] == 68 + up && pos[2] == z);
    assert!(went(2, 0, 0, "slime") && went(3, 0, 0, "stone") && went(2, 1, 0, "stone") && went(2, -1, 0, "stone"), "slime takes what is before, on and under it");
    assert!(went(2, 0, 3, "slime") && stayed(1, 3) && stayed(-1, 3), "and leaves obsidian and glazed terracotta");
    assert!(went(2, 0, 6, "slime") && stayed(1, 6) && stayed(2, 6), "and honey");
    assert!(went(2, 0, 9, "honey_block") && went(2, 1, 9, "stone"), "honey takes too");
    assert!(stayed(0, 12) && stayed(1, 12), "what it holds is held back by obsidian, and so is it");
    assert!(went(2, 0, 15, "slime") && went(2, 1, 15, "slime") && went(2, 2, 15, "stone") && went(3, 1, 15, "stone"), "slime takes slime and what that holds");
}

#[test]
fn slime_counts_what_it_holds_to_the_12_and_a_sticky_piston_draws_it_back_with_that() {
    let mut h = ready();
    let row = |z: i32, last: i32| hung(z, "piston", &[([1, 0], "slime"), ([1, 1], "stone")]).into_iter().chain((2..=last).map(move |x| ([x, 68, z], "stone", "")));
    let mut rigs: Vec<_> = row(0, 11).chain(row(3, 12)).collect();
    rigs.extend(hung(6, "sticky_piston", &[([1, 0], "slime"), ([2, 0], "stone"), ([1, 1], "stone")]));
    rigs.extend(hung(9, "sticky_piston", &[([1, 0], "stone"), ([2, 0], "slime"), ([2, 1], "stone")]));
    put(&mut h, &rigs);
    watch(&mut h, 4);
    put(&mut h, &[0, 3, 6, 9].map(|z| ([0, 67, z], "redstone_block", "")));
    let changed = block_changes(&watch(&mut h, 10).concat());
    assert!(changed.contains(&([12, 68, 0], hash("minecraft:stone"))) && changed.contains(&([2, 69, 0], hash("minecraft:stone"))), "12 go");
    assert!(!changed.iter().any(|(pos, _)| pos[2] == 3 && pos[1] >= 68), "13 stay");

    put(&mut h, &[6, 9].map(|z| ([0, 67, z], "air", "")));
    let back = block_changes(&watch(&mut h, 10).concat());
    let came = |pos: [i32; 3], name: &str| back.contains(&(pos, hash(&format!("minecraft:{name}"))));
    assert!(came([1, 68, 6], "slime") && came([2, 68, 6], "stone") && came([1, 69, 6], "stone"), "slime comes back with what it holds");
    assert!(came([1, 68, 9], "stone") && !back.iter().any(|(pos, _)| pos[0] == 3 && pos[2] == 9), "a stone does not bring the slime behind it");
}

#[test]
fn a_piston_pushes_what_stands_before_its_arm_and_its_blocks_as_on_bds() {
    // research/captures/bds-shove.txt: a stand in the arm's way went 0.25, then 0.5; one behind a
    // pushed stone ended before the stone; one against obsidian stopped at it.
    let mut h = ready();
    put(&mut h, &[([0, 65, 2], "piston", EAST), ([0, 65, 4], "piston", EAST), ([1, 65, 4], "stone", ""), ([0, 65, 6], "piston", EAST), ([2, 65, 6], "obsidian", ""), ([-4, 65, 2], "piston", "facing_direction=1")]);
    h.remote.run(|ctx| {
        for at in [[1.5, 65.0, 2.5], [2.5, 65.0, 4.5], [1.5, 65.0, 6.5], [-3.5, 66.0, 2.5]] {
            assert!(ctx.spawn_entity("minecraft:armor_stand", at).is_some());
        }
    });
    watch(&mut h, 4);
    put(&mut h, &[([0, 66, 2], "redstone_block", ""), ([0, 66, 4], "redstone_block", ""), ([0, 66, 6], "redstone_block", ""), ([-5, 65, 2], "redstone_block", "")]);
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    for _ in 0..10 {
        let seen = seen.clone();
        h.remote.run(move |ctx| seen.lock().unwrap().push(ctx.entities().iter().map(|stand| if stand.feet[0] < 0.0 { stand.feet[1] } else { stand.feet[0] }).collect::<Vec<f32>>()));
        h.tick_all();
    }
    let seen = seen.lock().unwrap().clone();
    let steps = |stand: usize| {
        let mut places: Vec<f32> = seen.iter().map(|feet| feet[stand]).collect();
        places.dedup();
        places
    };
    assert_eq!([steps(0), steps(1), steps(2)], [vec![1.5, 1.75, 2.25], vec![2.5, 2.75, 3.25], vec![1.5, 1.75]]);
    assert_eq!(steps(3).last(), Some(&67.0), "the one on a piston that pushes up stands on its arm: {:?}", steps(3));
}
