//! Shots that break decorated pots and chorus flowers while `projectilesCanBreakBlocks` is on
//! (research/captures/bds-shotblocks.txt).

use crate::common;

use std::sync::mpsc;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddItemEntity, EntityEvent, EntityEventEventId, LevelEvent, LevelEventEvent, LevelSoundEvent, MoveEntityDelta, SetEntityMotion};
use common::shooting::{ALEX, added, click_air, ready, release};
use common::{Harness, all};
use sapling_api::{Block, GameMode, Item, RuleValue, Slot};

const DISPENSER: [i32; 3] = [3, 65, 6];
const AIMED: [i32; 3] = [6, 65, 6];

/// Fires an arrow from a dispenser at `block`, three blocks east of it; what the shot sent.
fn shoot_at(block: Block, rule: bool) -> (Harness, Vec<RawPacket>) {
    let (h, ticks) = shoot_at_by_tick(block, rule);
    (h, ticks.concat())
}

/// [`shoot_at`], what was sent each tick.
fn shoot_at_by_tick(block: Block, rule: bool) -> (Harness, Vec<Vec<RawPacket>>) {
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
    let sent = (0..30).map(|_| h.tick_all().to(1).to_vec()).collect();
    (h, sent)
}

/// It shows breaking the tick the arrow meets it, and goes, heard shattering, the tick after (bds-p3pot.txt).
#[test]
fn a_shot_pot_shows_breaking_a_tick_before_it_shatters() {
    let (mut h, ticks) = shoot_at_by_tick(Block::new("decorated_pot"), true);
    let cracked = ticks.iter().position(|sent| all::<LevelEvent>(sent).iter().any(|event| event.event == LevelEventEvent::ParticleDestroy)).expect("cracked");
    let shattered = ticks.iter().position(|sent| all::<LevelSoundEvent>(sent).iter().any(|sound| sound.sound_id == "shatter_pot")).expect("shattered");
    assert_eq!(shattered, cracked + 1);
    assert_eq!(name_at(&mut h, AIMED), "minecraft:air");
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

fn told_motions(sent: &[RawPacket], id: u64) -> Vec<[f32; 3]> {
    all::<SetEntityMotion>(sent).iter().filter(|motion| motion.runtime_entity_id == id).map(|motion| [motion.velocity.x, motion.velocity.y, motion.velocity.z]).collect()
}

#[test]
fn an_arrow_whose_block_goes_falls_and_sticks_in_the_one_below() {
    let (mut h, sent) = shoot_at(Block::new("end_stone"), false);
    let arrow = added(&sent, "minecraft:arrow").pop().expect("the arrow").runtime_id;
    h.remote.run(|ctx| assert!(ctx.update_block(AIMED, &Block::new("air"))));
    let fell: Vec<RawPacket> = (0..30).flat_map(|_| h.tick_all().to(1).to_vec()).collect();
    let heights: Vec<f32> = all::<MoveEntityDelta>(&fell).iter().filter(|delta| delta.runtime_entity_id == arrow).filter_map(|delta| delta.y).collect();
    assert_eq!(heights.last(), Some(&65.0), "onto the floor: {heights:?}");
    let shakes = all::<EntityEvent>(&fell).into_iter().filter(|event| event.runtime_entity_id == arrow && event.event_id == EntityEventEventId::ArrowShake).count();
    assert_eq!(shakes, 1, "stuck again");
}

#[test]
fn a_flame_arrow_primes_tnt_and_bounces_back_off_it() {
    let mut h = ready(GameMode::Creative, &[("minecraft:bow", 1, 0), ("minecraft:arrow", 1, 0)], 20.0);
    let bow = Item { enchantments: vec![(21, 1)], ..Item::new("minecraft:bow", 1) };
    h.remote.run(move |ctx| {
        assert!(ctx.set_item(ALEX, Slot::Main(0), Some(&bow)));
        assert!(ctx.update_block([0, 65, 3], &Block::new("tnt")));
    });
    h.tick_all();
    h.receive(1, &click_air());
    (0..20).for_each(|_| drop(h.tick_all()));
    h.receive(1, &release());
    let ticks: Vec<Vec<RawPacket>> = (0..7).map(|_| h.tick_all().to(1).to_vec()).collect();
    // BDS tells the ignite the tick the arrow meets the TNT and primes it the tick after (bds-g10flame.txt).
    let ignited = ticks.iter().position(|sent| all::<LevelEvent>(sent).iter().any(|event| event.event == LevelEventEvent::SoundIgnite)).expect("ignited");
    assert!(added(&ticks[ignited], "minecraft:tnt").is_empty() && added(&ticks[ignited + 1], "minecraft:tnt").len() == 1);
    let sent: Vec<RawPacket> = ticks.concat();
    assert_eq!(added(&sent, "minecraft:tnt").len(), 1, "primed");
    let arrow = added(&sent, "minecraft:arrow").pop().expect("the arrow").runtime_id;
    let motions = told_motions(&sent, arrow);
    assert!(motions.iter().any(|motion| motion[2] < 0.0), "{motions:?}");
    assert!(!all::<EntityEvent>(&sent).iter().any(|event| event.runtime_entity_id == arrow && event.event_id == EntityEventEventId::ArrowShake), "not stuck");
}

#[test]
fn an_arrow_leaves_pointed_dripstone_alone() {
    let (mut h, _) = shoot_at(Block::new("pointed_dripstone").with("dripstone_thickness=tip,hanging=0"), true);
    assert_eq!(name_at(&mut h, AIMED), "minecraft:pointed_dripstone", "a trident alone breaks it");
}
