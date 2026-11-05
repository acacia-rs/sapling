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
    (0..40).for_each(|_| _ = h.tick(1));
    let early = names(&mut h, vec![[4, 70, 0]]);
    assert!(early[0].starts_with("minecraft:tube_coral_block"), "a block waits 45 ticks: {early:?}");
    (0..10).for_each(|_| _ = h.tick(1));
    let block = names(&mut h, vec![[4, 70, 0]]);
    assert!(block[0].starts_with("minecraft:dead_tube_coral_block"), "{block:?}");
    (0..160).for_each(|_| _ = h.tick(1));
    let late = names(&mut h, vec![[4, 70, 0], [6, 71, 0], [8, 71, 0]]);
    assert!(late[1].starts_with("minecraft:brain_coral_block"), "water beside: {late:?}");
    assert!(late[2].starts_with("minecraft:dead_fire_coral_fan"), "{late:?}");
}

/// research/captures/bds-e3coralmany.txt: filled coral plants died 5 to 6 ticks on, a set one alone
/// 58 to 141 (`CoralPlantBlock::neighborChanged` waits 5).
#[test]
fn a_dry_coral_plant_dies_five_ticks_after_its_neighbour_changes() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        set(ctx, [4, 70, 0], Block::new("stone"));
        set(ctx, [4, 71, 0], Block::new("tube_coral"));
        set(ctx, [5, 71, 0], Block::new("stone"));
    });
    (0..4).for_each(|_| _ = h.tick(1));
    let early = names(&mut h, vec![[4, 71, 0]]);
    assert!(early[0].starts_with("minecraft:tube_coral["), "{early:?}");
    (0..2).for_each(|_| _ = h.tick(1));
    let dead = names(&mut h, vec![[4, 71, 0]]);
    assert!(dead[0].starts_with("minecraft:dead_tube_coral["), "{dead:?}");
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
fn lava_with_air_over_it_sets_fire_beside_planks_near_it() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        for x in 3..=7 {
            for z in -2..=2 {
                set(ctx, [x, 64, z], Block::new("stone"));
                set(ctx, [x, 65, z], Block::new("stone"));
            }
        }
        set(ctx, [5, 65, 0], Block::new("lava"));
        set(ctx, [6, 66, 0], Block::new("oak_planks"));
    });
    h.tick(1);
    speed(&mut h, 3000);
    let around: Vec<[i32; 3]> = (4..=7).flat_map(|x| (66..=67).flat_map(move |y| (-1..=1).map(move |z| [x, y, z]))).collect();
    let lit = (0..40).any(|_| {
        (0..10).for_each(|_| _ = h.tick(1));
        names(&mut h, around.clone()).iter().any(|name| name.starts_with("minecraft:fire"))
    });
    assert!(lit, "no fire by the planks");
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
    let sent: Vec<acacia_proto::RawPacket> = (0..3).flat_map(|_| h.tick(1)).collect();
    // research/captures/bds-bkticks.txt: heard at the cell's corner.
    let absorbed = common::all::<acacia_proto::packets::LevelSoundEvent>(&sent).into_iter().find(|sound| sound.sound_id == "sponge.absorb").expect("the sponge heard");
    assert_eq!([absorbed.position.x, absorbed.position.y, absorbed.position.z], [5.0, 70.0, 0.0]);
    let mut looked = vec![[5, 70, 0]];
    looked.extend(pool);
    let after = names(&mut h, looked);
    assert!(after[0].starts_with("minecraft:wet_sponge"), "{after:?}");
    assert!(after[1..].iter().all(|name| !name.contains("water")), "{after:?}");
}

#[test]
fn a_sponge_reaches_seven_cells_along_a_channel() {
    // research/captures/bds-t3sponge.txt: a channel of 14 lost its first 7.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        for x in 0..=15 {
            (-1..=1).for_each(|z| set(ctx, [x, 69, z], Block::new("stone")));
            [-1, 1].into_iter().for_each(|z| set(ctx, [x, 70, z], Block::new("stone")));
        }
        set(ctx, [15, 70, 0], Block::new("stone"));
        (1..=14).for_each(|x| set(ctx, [x, 70, 0], Block::new("water")));
    });
    h.tick(1);
    h.remote.run(|ctx| set(ctx, [0, 70, 0], Block::new("sponge")));
    (0..2).for_each(|_| drop(h.tick(1)));
    let after = names(&mut h, vec![[7, 70, 0], [8, 70, 0]]);
    assert!(!after[0].contains("water") && after[1].contains("water"), "{after:?}");
}

#[test]
fn a_coral_fan_placed_dry_by_hand_is_dead_and_lies_by_where_it_is_from_the_placer() {
    // research/captures/bds-g3fan.txt: south of the feet direction 0, anywhere else 1.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| ctx.set_game_mode(sapling_api::PlayerId(1), sapling_api::GameMode::Creative));
    h.tick(1);
    for cell in [[0, 64, 2], [-2, 64, 0]] {
        h.receive(1, &common::click(cell, 1, Some("minecraft:tube_coral_fan")));
        h.tick(1);
    }
    let fans = names(&mut h, vec![[0, 65, 2], [-2, 65, 0]]);
    assert_eq!(fans, ["minecraft:dead_tube_coral_fan[coral_fan_direction=0]", "minecraft:dead_tube_coral_fan[coral_fan_direction=1]"]);
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
