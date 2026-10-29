//! Custom blocks a plugin registers: numbered after the vanilla states, shown by the hash a
//! client computes, placed from their item, broken in their own time and dropping what they say.

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddItemEntity, ItemRegistry};
use acacia_proto::types::Action as BlockAction;
use common::asking::ask;
use common::{Harness, all, block_actions, block_changes, click};
use sapling_api::{Block, BlockDrop, CustomBlock, Item, PlayerId, StateValues};

const ALEX: PlayerId = PlayerId(1);
const LAMP: &str = "shard_test:lamp";

/// Registers the lamp; its states' hashes, unlit then lit, and its item's network id.
fn register() -> ([u32; 2], i16) {
    let lamp = CustomBlock::new(LAMP, "Lamp", Vec::new()).state("shard_test:lit", StateValues::Bool).light_emission(10).break_seconds(Some(0.5)).drops(BlockDrop::Itself);
    assert!(sapling_shard::register_custom_block(&lamp));
    assert!(!sapling_shard::register_custom_block(&CustomBlock::new("minecraft:stone", "Stone", Vec::new())), "a vanilla name is refused");
    let hashes: Vec<u32> = sapling_world::custom_ids(LAMP).map(|id| sapling_world::custom_state(id).unwrap().hash).collect();
    let packets: Vec<RawPacket> = sapling_data::registry_packets().iter().map(|p| RawPacket::parse(p.clone()).unwrap()).collect();
    let registry: ItemRegistry = packets[0].decode().unwrap();
    let item = registry.itemstates.iter().find(|item| item.name == LAMP).expect("the lamp's item");
    assert!(!item.component_based && item.runtime_id < 0, "a block's item, numbered as BDS numbers them");
    ([hashes[0], hashes[1]], item.runtime_id)
}

#[test]
fn a_custom_block_is_set_shown_placed_broken_and_dropped() {
    let ([unlit, lit], item) = register();
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let lit_lamp = Block { name: LAMP.into(), properties: "shard_test:lit=1".into() };
    h.remote.run(move |ctx| assert!(ctx.set_block([1, 64, 0], &lit_lamp)));
    let shown = h.tick(1);
    assert!(block_changes(&shown).contains(&([1, 64, 0], lit)), "shown by its own hash");
    let block = ask(&mut h, |ctx| ctx.block([1, 64, 0]));
    assert_eq!(block.map(|block| (block.name, block.properties)), Some((LAMP.to_owned(), "shard_test:lit=1".to_owned())));

    h.remote.run(|ctx| _ = ctx.give(ALEX, &Item::new(LAMP, 2)));
    h.tick(1);
    h.receive(1, &click([2, 64, 0], 1, None));
    let placed = h.tick(1);
    assert!(block_changes(&placed).contains(&([2, 65, 0], unlit)), "placed from the hand in its first state");
    assert_eq!(ask(&mut h, |ctx| ctx.block([2, 65, 0])).map(|block| block.name), Some(LAMP.to_owned()));
    h.remote.run(|ctx| assert_eq!(ctx.held_item(ALEX).map(|item| item.count), Some(1)));

    h.receive(1, &block_actions(&[(BlockAction::StartBreak, [1, 64, 0])]));
    h.tick(1);
    h.receive(1, &block_actions(&[(BlockAction::PredictBreak, [1, 64, 0])]));
    h.tick(1);
    assert_eq!(ask(&mut h, |ctx| ctx.block([1, 64, 0])).map(|block| block.name), Some(LAMP.to_owned()), "half a second is ten ticks, not one");
    h.receive(1, &block_actions(&[(BlockAction::StartBreak, [1, 64, 0])]));
    (0..12).for_each(|_| drop(h.tick(1)));
    h.receive(1, &block_actions(&[(BlockAction::PredictBreak, [1, 64, 0])]));
    let broken: Vec<RawPacket> = (0..3).flat_map(|_| h.tick(1)).collect();
    assert_eq!(ask(&mut h, |ctx| ctx.block([1, 64, 0])).map(|block| block.name), Some("minecraft:air".to_owned()));
    assert!(all::<AddItemEntity>(&broken).iter().any(|drop| drop.item.network_id == item), "it drops itself");
}
