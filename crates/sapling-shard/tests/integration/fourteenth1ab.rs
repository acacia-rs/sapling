//! Group AB of the fourteenth wave: a falling mob's landing heard by sculk (bds-ab1land.txt), a
//! squid pressed against a wall (bds-ab1squid.txt; creatures/squids/pressed.rs), a dolphin swimming
//! with a player (creatures/dolphins/company.rs), the wither's middle head aiming (bds-ab1wither2.txt;
//! wither/heads.rs) and a frog making for land (bds-f14efish.txt; creatures/frogs/landward.rs).

use crate::common;

use acacia_proto::nbt::Value;
use acacia_proto::packets::{AddEntity, MoveEntityDelta};
use acacia_proto::types::InputData;
use common::{Harness, all, asking::ask, input_at};
use sapling_api::{Block, GameMode, PlayerId};

const SENSOR: [i32; 3] = [4, 65, 0];

/// The event type the sensor kept, if it heard one.
fn heard(h: &mut Harness) -> Option<i32> {
    let nbt = ask(h, |ctx| ctx.block_entity(SENSOR).and_then(|data| data.get::<Value>().cloned())).expect("a block actor");
    match nbt.get("VibrationListener").and_then(|listener| listener.get("event")) {
        Some(Value::Int(event)) => Some(*event),
        _ => None,
    }
}

/// A cow put in a glass shaft 1 wide at height `y`, a sensor 4 off: what the sensor heard 30 ticks on.
fn cow_heard_from(y: f32) -> Option<i32> {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(move |ctx| {
        assert!(ctx.update_block(SENSOR, &Block::new("minecraft:sculk_sensor")));
        for y in 65..=69 {
            for [x, z] in [[-1, 2], [1, 2], [0, 1], [0, 3]] {
                assert!(ctx.update_block([x, y, z], &Block::new("minecraft:glass")));
            }
        }
    });
    (0..4).for_each(|_| drop(h.tick_all()));
    h.remote.run(move |ctx| assert!(ctx.spawn_entity("minecraft:cow", [0.5, y, 2.5]).is_some()));
    (0..30).for_each(|_| drop(h.tick_all()));
    heard(&mut h)
}

#[test]
fn a_cow_landing_from_a_drop_is_heard_as_hit_ground_and_one_put_on_the_ground_is_not() {
    // bds-ab1land.txt: drops of 0.3 to 3 were heard at frequency 2 (dust at the comparator's hit_ground
    // power, as an item's landing), the cow summoned on the ground not at all.
    assert_eq!(cow_heard_from(65.0), None, "on the ground");
    assert_eq!(cow_heard_from(65.3), Some(31), "hit_ground after 0.3");
    assert_eq!(cow_heard_from(67.0), Some(31), "hit_ground after 2");
}

/// Glass walls round water filling x `x0..=x0+w`, z `z0..=z0+d`, y `low..=high`, dug into the ground.
fn pool(ctx: &mut sapling_api::ShardCtx<'_>, [x0, z0]: [i32; 2], [w, d]: [i32; 2], [low, high]: [i32; 2]) {
    for x in x0 - 1..=x0 + w + 1 {
        for z in z0 - 1..=z0 + d + 1 {
            for y in low - 1..=high {
                let inside = (x0..=x0 + w).contains(&x) && (z0..=z0 + d).contains(&z) && y >= low;
                assert!(ctx.set_block([x, y, z], &Block::new(if inside { "minecraft:water" } else { "minecraft:glass" })));
            }
        }
    }
}

#[test]
fn a_squid_against_a_wall_creeps_along_it_at_a_tenth_of_its_swim() {
    // bds-ab1squid.txt: squids in a pool 5 across crept along its walls at 0.0009 a tick (0.0018 a
    // told move every other tick) for seconds, where they swam at 0.0146 in open water.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        pool(ctx, [10, 10], [4, 4], [60, 64]);
        for at in [[10.5, 61.0, 10.5], [13.5, 61.0, 13.5], [12.5, 62.0, 11.5]] {
            assert!(ctx.spawn_entity("minecraft:squid", at).is_some());
        }
    });
    let mut tracks: Vec<Vec<[f32; 3]>> = Vec::new();
    for _ in 0..1200 {
        let squids: Vec<[f32; 3]> = ask(&mut h, |ctx| ctx.entities().into_iter().filter(|entity| entity.kind == "minecraft:squid").map(|entity| entity.feet).collect());
        tracks.resize(squids.len().max(tracks.len()), Vec::new());
        squids.into_iter().enumerate().for_each(|(squid, feet)| tracks[squid].push(feet));
    }
    let creeps = |track: &Vec<[f32; 3]>| {
        let steps: Vec<f32> = track.windows(2).map(|pair| (0..3).map(|axis| (pair[1][axis] - pair[0][axis]).powi(2)).sum::<f32>().sqrt()).collect();
        let (mut run, mut longest) = (0, 0);
        for step in steps {
            run = if step > 0.0 && step < 0.0016 { run + 1 } else { 0 };
            longest = longest.max(run);
        }
        longest
    };
    let longest: Vec<usize> = tracks.iter().map(creeps).collect();
    assert!(longest.iter().any(|run| *run >= 40), "a squid creeps along a wall for 2 s: {longest:?}");
}

#[test]
fn a_dolphin_swims_with_a_swimming_player() {
    // `SwimWithEntityGoal` (company.rs): a player swimming within 20 is taken at a draw of 1 in 10 a
    // tick and kept up with, 3 to its side and 2 to 6 ahead.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_game_mode(PlayerId(1), GameMode::Creative);
        pool(ctx, [-3, -4], [46, 8], [60, 64]);
        assert!(ctx.spawn_entity("minecraft:dolphin", [2.5, 61.0, 2.5]).is_some());
    });
    let (mut x, mut dolphin) = (0.5f32, [0.0f32; 3]);
    for tick in 0..450 {
        let mut input = input_at([x, 62.0, 0.5]);
        input.delta.x = 0.08;
        input.input_data = if tick == 0 { vec![InputData::StartSwimming] } else { Vec::new() };
        h.receive(1, &input);
        x += 0.08;
        dolphin = ask(&mut h, |ctx| ctx.entities().into_iter().find(|entity| entity.kind == "minecraft:dolphin").map(|entity| entity.feet)).expect("the dolphin");
    }
    assert!(dolphin[0] > 26.0 && (dolphin[0] - x).abs() < 12.0, "kept up with Alex at x {x}: {dolphin:?}");
}

#[test]
fn the_withers_middle_head_aims_its_told_head_yaw_at_its_prey() {
    // bds-ab1wither2.txt: 483 of 566 told head yaws within 5 degrees of the prey's, the body apart,
    // turning 10 degrees a tick. Here it flies over Alex, so its head swings round.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        ctx.set_difficulty(sapling_api::Difficulty::Normal);
        assert!(ctx.spawn_entity("minecraft:wither", [0.5, 66.0, 8.5]).is_some());
        assert!(ctx.give_effect(PlayerId(1), sapling_api::Effect::Resistance, 1000, 4, false));
    });
    let added: Vec<AddEntity> = (0..201).flat_map(|_| all::<AddEntity>(h.tick_all().to(1))).collect();
    let id = added.iter().find(|added| added.entity_type == "minecraft:wither").expect("the wither shown").runtime_id;
    let body = added.iter().find(|added| added.runtime_id == id).map(|added| added.body_yaw).unwrap_or(0.0);
    let alex = ask(&mut h, |ctx| ctx.position(PlayerId(1)).map(|at| at.feet)).expect("Alex");
    let mut at = ask(&mut h, |ctx| ctx.entities().into_iter().find(|entity| entity.kind == "minecraft:wither").map(|entity| entity.feet)).expect("the wither");
    let (mut head, mut heads, mut aimed) = (None, Vec::new(), 0);
    for _ in 0..120 {
        for moved in all::<MoveEntityDelta>(h.tick_all().to(1)).into_iter().filter(|moved| moved.runtime_entity_id == id) {
            assert_eq!(moved.rot_y, None, "the body holds still");
            head = moved.rot_z.or(head);
            at = [moved.x.unwrap_or(at[0]), at[1], moved.z.unwrap_or(at[2])];
        }
        let wanted = (at[0] - alex[0]).atan2(alex[2] - at[2]).to_degrees();
        let off = |degrees: f32| (degrees - wanted + 540.0).rem_euclid(360.0) - 180.0;
        let degrees = head.map(|byte| f32::from(byte) * 360.0 / 256.0);
        aimed += usize::from(degrees.is_some_and(|degrees| off(degrees).abs() <= 6.0));
        heads.extend(degrees);
    }
    let wrapped = |degrees: f32| ((degrees + 540.0).rem_euclid(360.0) - 180.0).abs();
    let swing = heads.windows(2).map(|pair| wrapped(pair[1] - pair[0])).fold(0.0f32, f32::max);
    let parted = heads.iter().map(|head| wrapped(head - body)).fold(0.0f32, f32::max);
    // 10 degrees a tick, told in 256ths.
    assert!(aimed >= 60 && swing <= 12.0 && parted >= 90.0, "aimed at Alex on {aimed} ticks, swung {swing} at most, {parted} off the body: {heads:?}");
}

#[test]
fn a_frog_in_water_makes_for_land_across_the_floor_and_up_the_wall() {
    // bds-f14efish.txt: from the middle of a pool 13 across the frog walked the floor at up to 0.11
    // a tick, rose up the wall at up to 0.14 and was out on the rim 16 s in.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        pool(ctx, [10, -6], [12, 12], [59, 64]);
        assert!(ctx.spawn_entity("minecraft:frog", [16.5, 59.0, 0.5]).is_some());
    });
    let mut track = Vec::new();
    for _ in 0..400 {
        track.push(ask(&mut h, |ctx| ctx.entities().into_iter().find(|entity| entity.kind == "minecraft:frog").map(|entity| entity.feet)).expect("the frog"));
    }
    let Some(out) = track.iter().position(|feet| feet[1] >= 65.0) else { panic!("on land in 20 s: {:?}", track.iter().step_by(10).collect::<Vec<_>>()) };
    let fastest = track[..out].windows(2).map(|pair| (pair[1][0] - pair[0][0]).hypot(pair[1][2] - pair[0][2])).fold(0.0f32, f32::max);
    assert!(out > 100 && fastest < 0.12, "out at tick {out}, fastest across {fastest}");
}
