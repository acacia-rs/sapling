//! Blocks whose state follows their neighbours: fences join, walls rise, stairs turn corners.

use crate::common;

use std::collections::HashMap;

use acacia_proto::packets::UpdateBlock;
use common::{Harness, all, click};
use sapling_api::{Block, Item, PlayerId, Slot};

const ALEX: PlayerId = PlayerId(1);
const ABOVE: [i32; 3] = [1, 65, 0];

/// Alex in survival, looking south, with a stack of `item`; `blocks` set around without any
/// reshaping, as a plugin would.
fn with(item: &str, blocks: &'static [([i32; 3], &'static str, &'static str)]) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    let item = Item::new(item, 8);
    h.remote.run(move |ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&item)));
        blocks.iter().for_each(|(pos, block, states)| assert!(ctx.set_block(*pos, &Block::new(*block).with(*states)), "{block}"));
    });
    h.tick_all();
    h
}

/// Places on top of the platform block under `ABOVE`; the states Steve then sees, by position.
fn place(h: &mut Harness) -> HashMap<[i32; 3], String> {
    h.receive(1, &click([1, 64, 0], 1, None));
    let registry = sapling_world::BlockRegistry::vanilla();
    let updates = all::<UpdateBlock>(h.tick_all().to(2));
    let state = |update: &UpdateBlock| registry.get(registry.runtime_id_from_hash(update.block_runtime_id)?).map(|state| state.properties.to_owned());
    updates.iter().filter(|update| update.layer == 0).filter_map(|update| Some(([update.position.x, update.position.y, update.position.z], state(update)?))).collect()
}

#[test]
fn a_fence_joins_the_fence_beside_it_and_that_one_joins_back() {
    let mut h = with("minecraft:oak_fence", &[([2, 65, 0], "oak_fence", ""), ([1, 65, -1], "stone", "")]);
    let seen = place(&mut h);
    let placed = &seen[&ABOVE];
    assert!(placed.contains("connection_east=1") && placed.contains("connection_north=1") && placed.contains("connection_west=0"), "{placed}");
    assert!(seen[&[2, 65, 0]].contains("connection_west=1"), "{seen:?}");
}

#[test]
fn a_wall_reaches_to_a_block_and_stands_on_a_post_where_it_does_not_run_straight() {
    let mut h = with("minecraft:cobblestone_wall", &[([2, 65, 0], "stone", ""), ([1, 66, 0], "stone", "")]);
    let placed = &place(&mut h)[&ABOVE];
    assert!(placed.contains("wall_connection_type_east=tall") && placed.contains("wall_connection_type_west=none") && placed.contains("wall_post_bit=1"), "{placed}");
}

#[test]
fn a_wall_with_a_post_on_a_straight_wall_raises_a_post_under_it() {
    const STRAIGHT: &str = "wall_connection_type_east=none,wall_connection_type_north=short,wall_connection_type_south=short,wall_connection_type_west=none,wall_post_bit=0";
    let mut h = with("minecraft:cobblestone_wall", &[([1, 65, -1], "stone", ""), ([1, 65, 1], "stone", ""), (ABOVE, "cobblestone_wall", STRAIGHT)]);
    h.receive(1, &click(ABOVE, 1, None));
    let registry = sapling_world::BlockRegistry::vanilla();
    let updates = all::<UpdateBlock>(h.tick_all().to(2));
    let below: Vec<_> = updates.iter().filter(|update| [update.position.x, update.position.y, update.position.z] == ABOVE).filter_map(|update| registry.get(registry.runtime_id_from_hash(update.block_runtime_id)?)).map(|state| state.properties).collect();
    assert_eq!(below, [STRAIGHT.replace("wall_post_bit=0", "wall_post_bit=1")]);
}

#[test]
fn a_rail_between_two_rails_curves_to_both_and_they_turn_to_it() {
    let mut h = with("minecraft:rail", &[([2, 65, 0], "rail", "rail_direction=0"), ([1, 65, 1], "rail", "rail_direction=1")]);
    let seen = place(&mut h);
    let shape = |pos: [i32; 3]| seen.get(&pos).cloned().unwrap_or_default();
    assert_eq!([shape(ABOVE), shape([2, 65, 0]), shape([1, 65, 1])], ["rail_direction=6", "rail_direction=1", "rail_direction=0"].map(String::from), "{seen:?}");
}

#[test]
fn stairs_turn_a_corner_against_stairs_across_them() {
    // Alex looks south; the stair in front of the new one faces east.
    let mut h = with("minecraft:oak_stairs", &[([1, 65, 1], "oak_stairs", "minecraft:corner=none,upside_down_bit=0,weirdo_direction=0")]);
    let placed = &place(&mut h)[&ABOVE];
    assert!(placed.contains("minecraft:corner=outer_left"), "{placed}");
}
