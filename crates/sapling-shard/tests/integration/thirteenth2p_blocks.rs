//! Blocks and redstone against the p2 drills: an observer clock, a tripwire hook's facing, a tool's
//! use resending nothing, a hand-placed vine's age and bone meal's scatter in one UpdateSubchunkBlocks.

use crate::common;

use std::sync::mpsc;

use acacia_proto::RawPacket;
use acacia_proto::packets::{UpdateBlock, UpdateSubchunkBlocks};
use common::{Harness, block_changes, click};
use sapling_api::{Block, GameMode, Item, PlayerId, Slot};

const WEST: u8 = 4;
const UP: u8 = 1;
const DOWN: u8 = 0;

fn state(name: &str, properties: &str) -> u32 {
    let registry = sapling_world::BlockRegistry::vanilla();
    registry.network_hash(registry.find(name, properties).unwrap_or_else(|| panic!("{name} [{properties}]")))
}

fn block(h: &mut Harness, pos: [i32; 3]) -> Block {
    let (sent, got) = mpsc::channel();
    h.remote.run(move |ctx| sent.send(ctx.block(pos).expect("loaded")).expect("the test waits"));
    h.tick_all();
    got.recv().expect("ran")
}

/// Alex in survival at the spawn holding `held`.
fn holding(held: &'static str) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Survival);
        assert!(ctx.set_item(PlayerId(1), Slot::Main(0), Some(&Item::new(held, 16))));
    });
    h.tick_all();
    h
}

fn set(h: &mut Harness, blocks: &'static [([i32; 3], &'static str)]) {
    h.remote.run(move |ctx| blocks.iter().for_each(|(pos, name)| assert!(ctx.set_block(*pos, &Block::new(*name)), "{name}")));
    h.tick_all();
}

/// research/captures/bds-p2rs.txt: two observers watching each other, in Java's order, pulse every 6 ticks.
#[test]
fn two_observers_watching_each_other_pulse_every_6_ticks() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let east = |lit: u8| state("minecraft:observer", &format!("minecraft:facing_direction=east,powered_bit={lit}"));
    h.remote.run(move |ctx| {
        assert!(ctx.set_block([3, 65, 4], &Block::new("observer").with("minecraft:facing_direction=east")));
        assert!(ctx.update_block([4, 65, 4], &Block::new("observer").with("minecraft:facing_direction=west")));
    });
    let ticks: Vec<Vec<RawPacket>> = (0..40).map(|_| h.tick_all().to(1).to_vec()).collect();
    let on: Vec<usize> = ticks.iter().enumerate().filter(|(_, sent)| block_changes(sent).contains(&([3, 65, 4], east(1)))).map(|(tick, _)| tick).collect();
    assert!(on.len() >= 4, "a clock: {on:?}");
    assert!(on.windows(2).all(|pair| pair[1] - pair[0] == 6), "{on:?}");
}

/// research/captures/bds-p2trip.txt: a hook put on a block's west face faced 1, on its east face 3.
#[test]
fn a_tripwire_hook_faces_off_the_face_it_was_put_on() {
    let mut h = holding("minecraft:tripwire_hook");
    set(&mut h, &[([2, 65, 0], "minecraft:stone")]);
    h.receive(1, &click([2, 65, 0], WEST, Some("minecraft:tripwire_hook")));
    h.tick(1);
    let hook = block(&mut h, [1, 65, 0]);
    assert!(hook.name == "minecraft:tripwire_hook" && hook.properties.contains("direction=1"), "{hook:?}");
}

/// research/captures/bds-p2place.txt: an end rod put on a block's west face faced 5, on its north face 3.
#[test]
fn an_end_rod_put_on_a_side_names_the_face_it_stands_on() {
    let mut h = holding("minecraft:end_rod");
    set(&mut h, &[([2, 65, 0], "minecraft:stone")]);
    h.receive(1, &click([2, 65, 0], WEST, Some("minecraft:end_rod")));
    h.tick(1);
    assert!(block(&mut h, [1, 65, 0]).properties.contains("facing_direction=5"));
}

/// research/captures/bds-p2tools.txt: a shovel's path was sent once, in layer 0, with no resend after.
#[test]
fn a_shovel_making_a_path_resends_nothing() {
    let mut h = holding("minecraft:iron_shovel");
    set(&mut h, &[([1, 64, 0], "minecraft:grass_block")]);
    h.receive(1, &click([1, 64, 0], UP, None));
    let sent = h.tick(1);
    assert_eq!(block(&mut h, [1, 64, 0]).name, "minecraft:grass_path");
    let updates: Vec<([i32; 3], u32)> = common::all::<UpdateBlock>(&sent).iter().map(|u| ([u.position.x, u.position.y, u.position.z], u.layer)).collect();
    assert_eq!(updates, [([1, 64, 0], 0)], "the path alone");
}

/// research/captures/bds-p2place.txt: glow berries put by hand took an age, 0 to 25, and their vine was
/// listed twice in one UpdateSubchunkBlocks.
#[test]
fn a_cave_vine_put_by_hand_takes_an_age_and_is_listed_twice() {
    let mut h = holding("minecraft:glow_berries");
    set(&mut h, &[([2, 66, 0], "minecraft:stone")]);
    let mut ages = Vec::new();
    for _ in 0..6 {
        h.receive(1, &click([2, 66, 0], DOWN, None));
        let sent = h.tick(1);
        let vine = block(&mut h, [2, 65, 0]);
        ages.extend(vine.properties.split(',').find_map(|pair| pair.strip_prefix("growing_plant_age=")?.parse::<u32>().ok()));
        let listed: usize = common::all::<UpdateSubchunkBlocks>(&sent).iter().map(|update| update.blocks.iter().filter(|cell| [cell.position.x, cell.position.y, cell.position.z] == [2, 65, 0]).count()).sum();
        assert_eq!(listed, 2, "{vine:?}");
        h.remote.run(|ctx| assert!(ctx.set_block([2, 65, 0], &Block::new("minecraft:air"))));
        h.tick_all();
    }
    assert!(ages.iter().all(|age| *age < 26) && ages.iter().any(|age| *age > 0), "{ages:?}");
}

/// research/captures/bds-p2meal.txt: bone meal on grass sent what came up in UpdateSubchunkBlocks.
#[test]
fn bone_meal_on_grass_sends_what_comes_up_in_one_update_a_sub_chunk() {
    let mut h = holding("minecraft:bone_meal");
    // One sub-chunk's field: a lone plant in another would come as an UpdateBlock.
    h.remote.run(|ctx| (1..=7).flat_map(|x| (1..=7).map(move |z| [x, 64, z])).for_each(|pos| assert!(ctx.set_block(pos, &Block::new("minecraft:grass_block")))));
    h.tick_all();
    h.receive(1, &click([3, 64, 3], UP, None));
    let sent = h.tick(1);
    let listed: usize = common::all::<UpdateSubchunkBlocks>(&sent).iter().map(|update| update.blocks.iter().filter(|cell| cell.position.y == 65).count()).sum();
    assert!(listed > 1, "the scatter in one: {listed}");
    assert!(common::all::<UpdateBlock>(&sent).iter().all(|update| update.position.y != 65), "not one block at a time");
}
