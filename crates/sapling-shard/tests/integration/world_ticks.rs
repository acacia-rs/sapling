//! Coral dying, the nether vines, chorus flowers and budding amethyst (blocks/coral.rs,
//! blocks/growth/{kelp,chorus,amethyst}.rs; docs/research/world-ticks.md).

use crate::common;

use common::Harness;
use sapling_api::{Block, ShardCtx};

fn set(ctx: &mut ShardCtx<'_>, pos: [i32; 3], block: Block) {
    assert!(ctx.set_block(pos, &block), "{block:?} at {pos:?}");
}

fn names(h: &mut Harness, cells: Vec<[i32; 3]>) -> Vec<String> {
    let (sent, got) = std::sync::mpsc::channel();
    h.remote.run(move |ctx| sent.send(cells.iter().map(|at| ctx.block(*at).map_or(String::new(), |block| format!("{}[{}]", block.name, block.properties))).collect::<Vec<_>>()).unwrap());
    h.tick(1);
    got.recv().unwrap()
}

fn speed(h: &mut Harness, speed: i32) {
    h.remote.run(move |ctx| assert!(ctx.set_game_rule("randomTickSpeed", sapling_api::RuleValue::Int(speed))));
    h.tick(1);
}

#[test]
fn coral_out_of_water_dies_and_coral_in_water_lives() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        set(ctx, [4, 70, 0], Block::new("tube_coral_block"));
        set(ctx, [6, 70, 0], Block::new("water"));
        set(ctx, [6, 71, 0], Block::new("brain_coral_block"));
        set(ctx, [8, 70, 0], Block::new("stone"));
        set(ctx, [8, 71, 0], Block::new("fire_coral_fan"));
    });
    (0..50).for_each(|_| _ = h.tick(1));
    let early = names(&mut h, vec![[4, 70, 0]]);
    assert!(early[0].starts_with("minecraft:tube_coral_block"), "waits 60 ticks at least: {early:?}");
    (0..60).for_each(|_| _ = h.tick(1));
    let late = names(&mut h, vec![[4, 70, 0], [6, 71, 0], [8, 71, 0]]);
    assert!(late[0].starts_with("minecraft:dead_tube_coral_block"), "{late:?}");
    assert!(late[1].starts_with("minecraft:brain_coral_block"), "water beside: {late:?}");
    assert!(late[2].starts_with("minecraft:dead_fire_coral_fan"), "{late:?}");
}

#[test]
fn weeping_vines_grow_down_and_twisting_vines_up() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        set(ctx, [4, 80, 0], Block::new("netherrack"));
        set(ctx, [4, 79, 0], Block::new("weeping_vines").with("weeping_vines_age=1"));
        set(ctx, [6, 64, 0], Block::new("netherrack"));
        set(ctx, [6, 65, 0], Block::new("twisting_vines").with("twisting_vines_age=1"));
    });
    h.tick(1);
    speed(&mut h, 1000);
    (0..300).for_each(|_| _ = h.tick(1));
    let grown = names(&mut h, vec![[4, 78, 0], [4, 77, 0], [6, 66, 0], [6, 67, 0]]);
    assert!(grown[..2].iter().all(|name| name.starts_with("minecraft:weeping_vines")), "{grown:?}");
    assert!(grown[2..].iter().all(|name| name.starts_with("minecraft:twisting_vines")), "{grown:?}");
}

#[test]
fn a_chorus_flower_on_end_stone_grows_into_a_plant() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        set(ctx, [4, 64, 0], Block::new("end_stone"));
        set(ctx, [4, 65, 0], Block::new("chorus_flower").with("age=0"));
    });
    h.tick(1);
    speed(&mut h, 3000);
    (0..200).for_each(|_| _ = h.tick(1));
    let grown = names(&mut h, vec![[4, 65, 0], [4, 66, 0]]);
    assert!(grown[0].starts_with("minecraft:chorus_plant"), "{grown:?}");
}

#[test]
fn a_sponge_set_in_water_soaks_it_up_and_turns_wet() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let pool: Vec<[i32; 3]> = (3..=7).flat_map(|x| (-2..=2).map(move |z| [x, 70, z])).filter(|cell| *cell != [5, 70, 0]).collect();
    let cells = pool.clone();
    h.remote.run(move |ctx| {
        for x in 2..=8 {
            for z in -3..=3 {
                set(ctx, [x, 69, z], Block::new("stone"));
            }
        }
        cells.iter().for_each(|cell| set(ctx, *cell, Block::new("water")));
    });
    h.tick(1);
    h.remote.run(|ctx| set(ctx, [5, 70, 0], Block::new("sponge")));
    (0..3).for_each(|_| _ = h.tick(1));
    let mut looked = vec![[5, 70, 0]];
    looked.extend(pool);
    let after = names(&mut h, looked);
    assert!(after[0].starts_with("minecraft:wet_sponge"), "{after:?}");
    assert!(after[1..].iter().all(|name| !name.contains("water")), "{after:?}");
}

#[test]
fn budding_amethyst_grows_buds_into_clusters() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| set(ctx, [4, 70, 0], Block::new("budding_amethyst")));
    h.tick(1);
    speed(&mut h, 3000);
    (0..300).for_each(|_| _ = h.tick(1));
    let around = names(&mut h, vec![[4, 71, 0], [4, 69, 0], [3, 70, 0], [5, 70, 0], [4, 70, -1], [4, 70, 1]]);
    assert!(around.iter().any(|name| name.starts_with("minecraft:amethyst_cluster")), "{around:?}");
    assert!(around[0].is_empty() || !around[0].contains("amethyst") || around[0].contains("block_face=up"), "{around:?}");
}
