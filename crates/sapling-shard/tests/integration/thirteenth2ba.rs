//! Against the group BA drills: a harnessed happy ghast's driver holding jump (`vertical_movement_action`,
//! research/captures/bds-ba2ghast.txt). The z 4.5 boats are spawn_push.rs's unit tests.

use crate::common;

use acacia_proto::packets::PlayerAuthInput;
use acacia_proto::types::{InputData, Vec2f};
use common::raising::{click_on, feet_of};
use common::shooting::{FEET, ready, watch};
use common::input_at;
use sapling_api::GameMode;

/// What the ghast's driver pushes it by in the last tick of `ticks` with `input`, past 0.91 of its speed.
fn push(h: &mut common::Harness, ghast: u64, input: &PlayerAuthInput, ticks: usize) -> f32 {
    let heights: Vec<f32> = (0..ticks).map(|_| {
        h.receive(1, input);
        feet_of(h, ghast)[1]
    }).collect();
    let [a, b, c] = heights[ticks - 3..] else { unreachable!() };
    (c - b) - 0.91 * (b - a)
}

/// bds-ba2ghast.txt: jump alone lifts it 0.008 more a tick; with a walk straight up the sum is cut to its
/// flying speed, 0.016, as a walk alone.
#[test]
fn a_happy_ghast_rises_while_its_driver_holds_jump() {
    let mut h = ready(GameMode::Creative, &[("minecraft:white_harness", 1, 0)], 0.0);
    let ghast = common::asking::ask(&mut h, |ctx| ctx.summon_as(None, "minecraft:happy_ghast", [0.5, 66.0, 4.0], 0.0, 0.0, Some("minecraft:spawn_adult"))).expect("summoned").0;
    watch(&mut h, 2);
    click_on(&mut h, ghast);
    click_on(&mut h, ghast);
    let jump = PlayerAuthInput { input_data: vec![InputData::Jumping, InputData::JumpDown, InputData::WantUp], pitch: -90.0, ..input_at(FEET) };
    let up = push(&mut h, ghast, &jump, 6);
    assert!((up - 0.008).abs() < 1e-4, "jump alone: {up}");
    let leap = PlayerAuthInput { move_vector: Vec2f { x: 0.0, z: 1.0 }, ..jump };
    let up = push(&mut h, ghast, &leap, 6);
    assert!((up - 0.016).abs() < 1e-4, "walk and jump: {up}");
}
