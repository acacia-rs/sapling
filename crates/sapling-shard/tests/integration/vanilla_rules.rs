//! A world's game rules switching vanilla off: BDS's that name a mechanic, and sapling's own
//! (`sapling:hunger`, ..). Each is checked against the same scene with the rule on.

use crate::common;

use std::sync::mpsc;

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddItemEntity, GameRulesChanged, UpdateAttributes};
use acacia_proto::types::InputData;
use common::{Harness, all, input_at};
use sapling_api::{Block, Difficulty, Effect, EntityId, PlayerId, RuleValue, ShardCtx};

const ALEX: PlayerId = PlayerId(1);

fn ask<T: Send + 'static>(h: &mut Harness, ask: impl FnOnce(&mut ShardCtx<'_>) -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    h.remote.run(move |ctx| tx.send(ask(ctx)).expect("asked"));
    h.tick_all();
    rx.recv().expect("answered")
}

/// Alex in a world with `rule` set to `on`.
fn ruled(rule: &'static str, on: bool) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    assert!(ask(&mut h, move |ctx| ctx.set_game_rule(rule, RuleValue::Bool(on))), "{rule}");
    h
}

fn told(sent: &[RawPacket], name: &str) -> Vec<f32> {
    all::<UpdateAttributes>(sent).iter().flat_map(|update| &update.attributes).filter(|attribute| attribute.name == name).map(|attribute| attribute.current).collect()
}

#[test]
fn saplings_own_rules_are_set_and_listed_but_never_told() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_game_rule("sapling:hunger", RuleValue::Bool(false))));
    assert!(all::<GameRulesChanged>(h.tick_all().to(1)).is_empty());
    assert_eq!(ask(&mut h, |ctx| ctx.game_rule("SAPLING:HUNGER")), Some(("sapling:hunger".to_owned(), RuleValue::Bool(false))));
    h.remote.run(|ctx| assert!(ctx.set_game_rule("fallDamage", RuleValue::Bool(false))));
    assert_eq!(all::<GameRulesChanged>(h.tick_all().to(1)).len(), 1, "BDS's own are told");
}

/// Health told to Alex after a 12-block fall (vitals.rs).
fn fallen(mut h: Harness) -> Vec<f32> {
    for height in [73.0, 77.0, 73.0, 69.0] {
        h.receive(1, &input_at([0.5, height, 0.5]));
        h.tick_all();
    }
    let mut landing = input_at([0.5, 65.0, 0.5]);
    (landing.input_data, landing.delta.y) = (vec![InputData::VerticalCollision], -1.0);
    h.receive(1, &landing);
    (0..3).flat_map(|_| told(h.tick_all().to(1), "minecraft:health")).collect()
}

#[test]
fn fall_damage_off_spares_a_fall() {
    assert_eq!(fallen(ruled("fallDamage", true)), [15.0]);
    assert!(fallen(ruled("fallDamage", false)).is_empty());
}

/// Food told to Alex over two seconds of a strong hunger effect.
fn starved(mut h: Harness) -> Vec<f32> {
    h.remote.run(|ctx| assert!(ctx.give_effect(ALEX, Effect::Hunger, 100, 40, false)));
    (0..40).flat_map(|_| told(h.tick_all().to(1), "minecraft:player.hunger")).collect()
}

#[test]
fn hunger_off_keeps_a_player_fed() {
    assert!(!starved(ruled("sapling:hunger", true)).is_empty());
    assert!(starved(ruled("sapling:hunger", false)).is_empty());
}

fn broken_dirt_drops(on: bool) -> usize {
    let mut h = ruled("doTileDrops", on);
    h.remote.run(|ctx| assert!(ctx.set_block([3, 65, 0], &Block::new("dirt")) && ctx.break_block([3, 65, 0])));
    (0..3).map(|_| all::<AddItemEntity>(h.tick_all().to(1)).len()).sum()
}

#[test]
fn tile_drops_off_breaks_blocks_without_items() {
    assert_eq!((broken_dirt_drops(true), broken_dirt_drops(false)), (1, 0));
}

/// What is in the cell a walled-in water source opens onto, a second after the wall broke.
fn opened_onto(on: bool) -> Option<String> {
    let mut h = ruled("sapling:liquidFlow", on);
    h.remote.run(|ctx| {
        for (at, name) in [([3, 65, 0], "stone"), ([5, 65, 0], "stone"), ([4, 65, 1], "stone"), ([4, 65, -1], "stone"), ([4, 65, 0], "water")] {
            assert!(ctx.set_block(at, &Block::new(name)));
        }
    });
    h.tick_all();
    h.remote.run(|ctx| assert!(ctx.break_block([3, 65, 0])));
    (0..20).for_each(|_| drop(h.tick_all()));
    ask(&mut h, |ctx| ctx.block([3, 65, 0])).map(|block| block.name)
}

#[test]
fn liquid_flow_off_keeps_water_where_it_stands() {
    assert!(opened_onto(true).is_some_and(|name| name.contains("water")));
    assert_eq!(opened_onto(false).as_deref(), Some("minecraft:air"));
}

/// How far across a zombie six blocks from Alex goes in two seconds.
fn zombie_walks(on: bool) -> f32 {
    let mut h = ruled("sapling:mobAi", on);
    let zombie: EntityId = ask(&mut h, |ctx| {
        // Hunting needs a difficulty, and night keeps the sun off it (hunting.rs).
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        ctx.spawn_entity("minecraft:zombie", [6.5, 65.0, 0.5])
    })
    .expect("a zombie");
    (0..40).for_each(|_| drop(h.tick_all()));
    let feet = ask(&mut h, move |ctx| ctx.entity(zombie).expect("still there").feet);
    (feet[0] - 6.5).abs() + (feet[2] - 0.5).abs()
}

#[test]
fn mob_ai_off_leaves_a_mob_standing() {
    assert!(zombie_walks(true) > 1.0);
    assert!(zombie_walks(false) < 0.01);
}
