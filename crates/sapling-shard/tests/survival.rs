//! Survival's own rules for blocks: a break takes the time the block and the tool make it, and a
//! place comes out of the hand as the server knows it.

mod common;

use std::sync::{Arc, Mutex};

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddItemEntity, InventoryContent, LevelEvent, LevelEventEvent, UpdateBlock};
use acacia_proto::nbt::Value;
use acacia_proto::types::{Action, ItemV4Extra};
use common::{Harness, all, block_actions, click, hash};
use sapling_api::{Item, PlayerId, Slot};
use sapling_data::Stack;

const STONE: &str = "minecraft:stone";
const DIRT: &str = "minecraft:dirt";
const AIR: &str = "minecraft:air";
const NEAR: [i32; 3] = [1, 64, 0];
const ABOVE: [i32; 3] = [1, 65, 0];
const ALEX: PlayerId = PlayerId(1);

/// The layer-0 blocks among `packets` as (position, block).
fn blocks(packets: &[RawPacket]) -> Vec<([i32; 3], u32)> {
    all::<UpdateBlock>(packets).iter().filter(|u| u.layer == 0).map(|u| ([u.position.x, u.position.y, u.position.z], u.block_runtime_id)).collect()
}

/// Alex, in survival (the default), with `item` in the hand, and Steve watching.
fn with_in_hand(item: Option<Item>) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.enter(2, "Steve");
    h.remote.run(move |ctx| assert!(ctx.set_item(ALEX, Slot::Main(0), item.as_ref())));
    h.tick_all();
    h
}

fn held(h: &mut Harness) -> Option<Item> {
    let seen = Arc::new(Mutex::new(None));
    let out = seen.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = ctx.held_item(ALEX));
    h.tick_all();
    seen.lock().unwrap().take()
}

#[test]
fn a_break_takes_the_time_the_block_and_the_tool_need() {
    // Stone by hand is 150 ticks, with a wooden pickaxe 23.
    let mut h = with_in_hand(Some(Item::new("minecraft:wooden_pickaxe", 1)));
    h.receive(1, &block_actions(&[(Action::StartBreak, NEAR)]));
    let sent = h.tick_all();
    let cracks: Vec<LevelEvent> = all::<LevelEvent>(sent.to(2)).into_iter().filter(|event| event.event == LevelEventEvent::BlockStartBreak).collect();
    assert_eq!((cracks.len(), cracks[0].data, [cracks[0].position.x, cracks[0].position.y, cracks[0].position.z]), (1, 65535 / 23, [1.0, 64.0, 0.0]), "the watcher sees it crack");

    // Finished after five ticks: refused, and the breaker is shown the block.
    (0..5).for_each(|_| drop(h.tick_all()));
    h.receive(1, &block_actions(&[(Action::PredictBreak, NEAR)]));
    let sent = h.tick_all();
    assert_eq!((blocks(sent.to(1)), blocks(sent.to(2))), (vec![(NEAR, hash(STONE)), (ABOVE, hash(AIR))], vec![]));

    // Started again and given its time (a few ticks of slack for the network), it breaks.
    h.receive(1, &block_actions(&[(Action::StartBreak, NEAR)]));
    (0..21).for_each(|_| drop(h.tick_all()));
    h.receive(1, &block_actions(&[(Action::ContinueBreak, NEAR), (Action::PredictBreak, NEAR)]));
    let sent = h.tick_all();
    assert_eq!(blocks(sent.to(2)), [(NEAR, hash(AIR))]);
    let left = all::<AddItemEntity>(sent.to(2));
    let cobblestone = sapling_data::item(&Stack { name: "minecraft:cobblestone", count: 1, damage: 0, block: None, tag: None }, None).unwrap();
    assert_eq!((left.len(), left[0].item.network_id, left[0].item.count), (1, cobblestone.network_id, 1), "a pickaxe makes stone leave cobblestone");
    // BDS tosses it up from somewhere in the cell, 0.15 in from its sides (docs/research/items.md).
    let at = [left[0].position.x - 1.0, left[0].position.y - 64.0, left[0].position.z];
    assert!(at.iter().all(|c| (0.15..=0.85).contains(c)) && left[0].velocity.y == 0.2, "{:?} {:?}", left[0].position, left[0].velocity);
    let windows = all::<InventoryContent>(sent.to(1));
    let ItemV4Extra::Default(Some(pickaxe)) = &windows[0].input[0].extra else { panic!("{:?}", windows[0].input[0]) };
    assert_eq!(pickaxe.nbt.as_ref().expect("a tag").nbt.value.get("Damage"), Some(&Value::Int(1)), "the break cost the pickaxe a use");

    // A break that was never started, one given up, and one finished on another block.
    for early in [vec![(Action::PredictBreak, [2, 64, 0])], vec![(Action::StartBreak, [2, 64, 0]), (Action::AbortBreak, [2, 64, 0])], vec![(Action::StartBreak, [2, 64, 1])]] {
        h.receive(1, &block_actions(&early));
        (0..30).for_each(|_| drop(h.tick_all()));
        h.receive(1, &block_actions(&[(Action::PredictBreak, [2, 64, 0])]));
        assert!(blocks(h.tick_all().to(2)).is_empty(), "{early:?}");
    }
    let stops = |sent: &[RawPacket]| all::<LevelEvent>(sent).iter().filter(|event| event.event == LevelEventEvent::BlockStopBreak).count();
    h.receive(1, &block_actions(&[(Action::StartBreak, [2, 64, 0]), (Action::AbortBreak, [2, 64, 0])]));
    assert_eq!(stops(h.tick_all().to(2)), 1, "a break given up stops cracking");
}

#[test]
fn bare_hands_are_slow_and_bedrock_does_not_break() {
    let mut h = with_in_hand(None);
    h.remote.run(|ctx| assert!(ctx.set_block([2, 64, 0], &sapling_api::Block::new("bedrock"))));
    h.tick_all();
    for (pos, breaks) in [(NEAR, false), ([2, 64, 0], false)] {
        h.receive(1, &block_actions(&[(Action::StartBreak, pos)]));
        (0..30).for_each(|_| drop(h.tick_all()));
        h.receive(1, &block_actions(&[(Action::PredictBreak, pos)]));
        assert_eq!(!blocks(h.tick_all().to(2)).is_empty(), breaks, "{pos:?} after 30 ticks");
    }
    h.receive(1, &block_actions(&[(Action::StartBreak, NEAR)]));
    (0..148).for_each(|_| drop(h.tick_all()));
    h.receive(1, &block_actions(&[(Action::PredictBreak, NEAR)]));
    let sent = h.tick_all();
    assert_eq!(blocks(sent.to(2)), [(NEAR, hash(AIR))], "stone by hand, after its 150 ticks");
    assert!(all::<AddItemEntity>(sent.to(2)).is_empty(), "which leaves nothing");
}

#[test]
fn a_placed_block_turns_to_its_placer_and_takes_the_place_of_grass() {
    let registry = sapling_world::BlockRegistry::vanilla();
    let state = |name: &str, properties: &str| registry.network_hash(registry.find(name, properties).expect("a vanilla state"));
    let mut h = with_in_hand(Some(Item::new("minecraft:oak_stairs", 2)));
    h.remote.run(|ctx| assert!(ctx.set_block([2, 65, 0], &sapling_api::Block::new("short_grass"))));
    h.tick_all();
    // Alex looks south (yaw 0) and clicks the top of a block: the stairs rise to the south.
    h.receive(1, &click(NEAR, 1, None));
    assert_eq!(blocks(h.tick_all().to(2)), [(ABOVE, state("minecraft:oak_stairs", "minecraft:corner=none,upside_down_bit=0,weirdo_direction=2"))]);
    // A click on the grass puts the stairs where the grass was, not on top of it.
    h.receive(1, &click([2, 65, 0], 1, None));
    assert_eq!(blocks(h.tick_all().to(2))[0].0, [2, 65, 0]);
}

#[test]
fn a_door_is_two_cells_that_are_placed_opened_and_broken_together() {
    let mut h = with_in_hand(Some(Item::new("minecraft:spruce_door", 1)));
    let top = [1, 66, 0];
    h.receive(1, &click(NEAR, 1, None));
    let placed = blocks(h.tick_all().to(2));
    assert_eq!([placed[0].0, placed[1].0], [ABOVE, top], "{placed:?}");
    assert_ne!(placed[0].1, placed[1].1, "the upper cell says that it is one");

    // The hand is empty now: a click on either cell opens both.
    h.receive(1, &click(top, 2, None));
    let opened = blocks(h.tick_all().to(2));
    assert_eq!([opened[0].0, opened[1].0], [top, ABOVE], "{opened:?}");
    assert!(!placed.iter().any(|cell| opened.iter().any(|open| open.1 == cell.1)), "both changed state");

    h.remote.run(|ctx| ctx.set_game_mode(ALEX, sapling_api::GameMode::Creative));
    h.tick_all();
    h.receive(1, &block_actions(&[(Action::PredictBreak, ABOVE)]));
    let mut gone = blocks(h.tick_all().to(2));
    gone.sort();
    assert_eq!(gone, [(ABOVE, hash(AIR)), (top, hash(AIR))]);
}

#[test]
fn a_second_door_hinges_on_the_other_side_and_a_bed_lies_ahead_of_its_placer() {
    let registry = sapling_world::BlockRegistry::vanilla();
    let is_hinged_right = |hash: u32| registry.get(registry.runtime_id_from_hash(hash).expect("a block")).expect("a state").properties.contains("door_hinge_bit=1");
    let mut h = with_in_hand(Some(Item::new("minecraft:spruce_door", 2)));
    // Alex looks south, so east is to the left: the door placed west of the first is its pair.
    h.receive(1, &click([2, 64, 0], 1, None));
    let first = blocks(h.tick_all().to(2));
    h.receive(1, &click(NEAR, 1, None));
    let second = blocks(h.tick_all().to(2));
    assert_eq!([first.len(), second.len()], [2, 2], "{first:?} {second:?}");
    assert!(first.iter().all(|cell| !is_hinged_right(cell.1)) && second.iter().all(|cell| is_hinged_right(cell.1)));

    let mut h = with_in_hand(Some(Item::new("minecraft:bed", 1)));
    let head = [1, 65, 1];
    h.receive(1, &click(NEAR, 1, None));
    let placed = blocks(h.tick_all().to(2));
    assert_eq!([placed[0].0, placed[1].0], [ABOVE, head], "{placed:?}");
    assert_ne!(placed[0].1, placed[1].1, "the head says that it is one");
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, sapling_api::GameMode::Creative));
    h.tick_all();
    h.receive(1, &block_actions(&[(Action::PredictBreak, head)]));
    let mut gone = blocks(h.tick_all().to(2));
    gone.sort();
    assert_eq!(gone, [(ABOVE, hash(AIR)), (head, hash(AIR))]);
}

#[test]
fn sand_falls_a_torch_goes_with_its_block_and_a_button_comes_out_again() {
    let mut h = with_in_hand(None);
    let (pillar, sand, torch, button) = ([2, 65, 0], [2, 66, 0], [2, 67, 0], [2, 65, 1]);
    h.remote.run(move |ctx| {
        let set = |ctx: &mut sapling_api::ShardCtx<'_>, pos, block: &str| assert!(ctx.set_block(pos, &sapling_api::Block::new(block)), "{block}");
        set(ctx, pillar, "dirt");
        set(ctx, sand, "sand");
        assert!(ctx.set_block(torch, &sapling_api::Block::new("torch").with("torch_facing_direction=top")));
        set(ctx, [2, 64, 1], "stone");
        ctx.set_block(button, &sapling_api::Block::new("stone_button").with("button_pressed_bit=0,facing_direction=1"));
        ctx.set_game_mode(ALEX, sapling_api::GameMode::Creative);
    });
    h.tick_all();

    // The dirt goes: the sand falls into its cell, and the torch that stood on the sand breaks
    // and leaves a torch.
    h.receive(1, &block_actions(&[(Action::PredictBreak, pillar)]));
    let sent: Vec<RawPacket> = (0..20).flat_map(|_| h.tick_all().to(2).to_vec()).collect();
    let after: std::collections::HashMap<_, _> = blocks(&sent).into_iter().collect();
    let now = [pillar, sand, torch].map(|pos| after.get(&pos).copied());
    assert_eq!(now, [Some(hash("minecraft:sand")), Some(hash(AIR)), Some(hash(AIR))], "{after:?}");
    assert_eq!(all::<AddItemEntity>(&sent).len(), 1, "the torch lies there");

    let pressed = |sent: &[RawPacket]| blocks(sent).iter().filter(|(pos, _)| *pos == button).count();
    h.receive(1, &click(button, 1, None));
    assert_eq!(pressed(h.tick_all().to(2)), 1, "in");
    let released: usize = (0..25).map(|_| pressed(h.tick_all().to(2))).sum();
    assert_eq!(released, 1, "and out again within the second");
}

#[test]
fn water_runs_into_the_room_a_broken_block_leaves_and_dries_up_without_its_source() {
    let registry = sapling_world::BlockRegistry::vanilla();
    let water = |depth: u8| registry.network_hash(registry.find("minecraft:water", &format!("liquid_depth={depth}")).expect("water of that depth"));
    let mut h = with_in_hand(Some(Item::new(STONE, 1)));
    let (source, dam) = ([4, 65, 0], [3, 65, 0]);
    h.remote.run(move |ctx| {
        let walls = [dam, [5, 65, 0], [4, 65, 1], [4, 65, -1]];
        walls.iter().for_each(|wall| assert!(ctx.set_block(*wall, &sapling_api::Block::new("stone"))));
        assert!(ctx.set_block(source, &sapling_api::Block::new("water")));
        ctx.set_game_mode(ALEX, sapling_api::GameMode::Creative);
    });
    h.tick_all();

    // The dam goes: five ticks later the water is in its cell, then a cell further and thinner.
    h.receive(1, &block_actions(&[(Action::PredictBreak, dam)]));
    let ran: Vec<_> = (0..12).flat_map(|_| blocks(h.tick_all().to(2))).collect();
    assert!(ran.contains(&(dam, water(1))) && ran.contains(&([2, 65, 0], water(2))), "{ran:?}");

    // A stone in the source's cell: the stream thins out and is gone.
    h.remote.run(|ctx| ctx.set_game_mode(ALEX, sapling_api::GameMode::Survival));
    h.tick_all();
    h.receive(1, &click([4, 64, 0], 1, None));
    (0..200).for_each(|_| drop(h.tick_all()));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let out = seen.clone();
    h.remote.run(move |ctx| *out.lock().unwrap() = [source, dam, [2, 65, 0]].map(|pos| ctx.block(pos).map(|block| block.name)).to_vec());
    h.tick_all();
    assert_eq!(*seen.lock().unwrap(), [Some(STONE.to_string()), Some(AIR.to_string()), Some(AIR.to_string())]);
}

#[test]
fn a_place_is_the_block_in_the_servers_hand_and_takes_one_of_it() {
    let mut h = with_in_hand(Some(Item::new(DIRT, 2)));
    // The client says stone; the hand holds dirt.
    h.receive(1, &click(NEAR, 1, Some(STONE)));
    let sent = h.tick_all();
    assert_eq!(blocks(sent.to(2)), [(ABOVE, hash(DIRT))]);
    let windows = all::<InventoryContent>(sent.to(1));
    assert_eq!((windows.len(), windows[0].input[0].count), (4, 1), "the placer is shown its stack, one short");

    // A cell that is taken: nothing is placed and nothing is spent.
    h.receive(1, &click(NEAR, 1, Some(DIRT)));
    let sent = h.tick_all();
    assert!(blocks(sent.to(2)).is_empty());
    assert_eq!(all::<InventoryContent>(sent.to(1))[0].input[0].count, 1);

    // The last one goes; then the hand is empty and a client's word places nothing.
    h.receive(1, &click(ABOVE, 1, Some(DIRT)));
    assert_eq!(blocks(h.tick_all().to(2)), [([1, 66, 0], hash(DIRT))]);
    assert_eq!(held(&mut h), None);
    h.receive(1, &click([1, 66, 0], 1, Some(DIRT)));
    let sent = h.tick_all();
    assert!(blocks(sent.to(2)).is_empty());
    assert_eq!(blocks(sent.to(1)), [([1, 66, 0], hash(DIRT)), ([1, 67, 0], hash(AIR))], "shown what is there");
}
