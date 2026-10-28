//! Shots that break decorated pots and chorus flowers while `projectilesCanBreakBlocks` is on
//! (research/captures/bds-shotblocks.txt).

use crate::common;

use std::sync::mpsc;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddItemEntity, LevelSoundEvent};
use common::{Harness, all};
use sapling_api::{Block, Item, RuleValue};

const DISPENSER: [i32; 3] = [3, 65, 6];
const AIMED: [i32; 3] = [6, 65, 6];

/// Fires an arrow from a dispenser at `block`, three blocks east of it; what the shot sent.
fn shoot_at(block: Block, rule: bool) -> (Harness, Vec<RawPacket>) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        assert!(ctx.set_game_rule("projectilesCanBreakBlocks", RuleValue::Bool(rule)));
        for (pos, block) in [([6, 64, 6], Block::new("end_stone")), (DISPENSER, Block::new("dispenser").with("facing_direction=5")), (AIMED, block)] {
            assert!(ctx.update_block(pos, &block), "{block:?}");
        }
        assert!(ctx.set_container_item(DISPENSER, 0, Some(&Item::new("minecraft:arrow", 1))));
    });
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.update_block([3, 65, 7], &Block::new("redstone_block"))));
    let sent = (0..30).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    (h, sent)
}

fn name_at(h: &mut Harness, pos: [i32; 3]) -> String {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ctx.block(pos).map(|block| block.name).unwrap_or_default()).unwrap());
    h.tick_all();
    rx.recv().unwrap()
}

#[test]
fn an_arrow_shatters_a_decorated_pot_into_four_bricks() {
    let (mut h, sent) = shoot_at(Block::new("decorated_pot"), true);
    assert_eq!(name_at(&mut h, AIMED), "minecraft:air");
    assert!(all::<LevelSoundEvent>(&sent).iter().any(|sound| sound.sound_id == "shatter_pot" && sound.position.x == AIMED[0] as f32), "heard from its corner");
    assert_eq!(all::<AddItemEntity>(&sent).len(), 4, "a brick for each plain face");
}

#[test]
fn an_arrow_breaks_a_chorus_flower_which_leaves_itself() {
    let (mut h, sent) = shoot_at(Block::new("chorus_flower"), true);
    assert_eq!(name_at(&mut h, AIMED), "minecraft:air");
    assert_eq!(all::<AddItemEntity>(&sent).len(), 1);
}

#[test]
fn with_the_rule_off_a_pot_stays() {
    let (mut h, _) = shoot_at(Block::new("decorated_pot"), false);
    assert_eq!(name_at(&mut h, AIMED), "minecraft:decorated_pot");
}

#[test]
fn an_arrow_leaves_pointed_dripstone_alone() {
    let (mut h, _) = shoot_at(Block::new("pointed_dripstone").with("dripstone_thickness=tip,hanging=0"), true);
    assert_eq!(name_at(&mut h, AIMED), "minecraft:pointed_dripstone", "a trident alone breaks it");
}
