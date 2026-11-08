//! Minecarts as drilled on BDS (research/captures/bds-i2rails.txt): a detector rail pressed a tick
//! after a summoned cart's first tick and let go at the first 20-tick check that finds no cart, a cart's
//! height on its rail, and a killed cart leaving its item.

use std::sync::{Arc, Mutex};

use crate::common;
use crate::riding::spawned;

use acacia_proto::packets::AddItemEntity;
use common::{Harness, all};
use sapling_api::{Block, BlockPos};

const RAIL: BlockPos = [3, 65, 0];

/// Ticks once and tells whether the rail was pressed at the harness's turn in that tick.
fn step(h: &mut Harness) -> bool {
    let on = Arc::new(Mutex::new(false));
    let seen = on.clone();
    h.remote.run(move |ctx| *seen.lock().unwrap() = ctx.block(RAIL).is_some_and(|rail| rail.properties.contains("rail_data_bit=1")));
    h.tick(1);
    let on = *on.lock().unwrap();
    on
}

#[test]
fn a_detector_rail_is_pressed_a_tick_after_a_carts_first_and_let_go_at_its_next_check_after_the_cart_goes() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_block(RAIL, &Block::new("detector_rail"))));
    h.tick(1);
    h.tick(1);
    let cart = spawned(&mut h, &[("minecraft:minecart", [3.5, 65.0, 0.5])])[0];
    let states: Vec<bool> = (0..60)
        .map(|tick| {
            if tick == 25 {
                h.remote.run(move |ctx| assert!(ctx.kill_entity(cart)));
            }
            step(&mut h)
        })
        .collect();
    let pressed = states.iter().position(|on| *on).expect("pressed");
    assert_eq!(pressed, 1, "a tick after the cart's first: {states:?}");
    let let_go = states.iter().skip(pressed).position(|on| !*on).map(|after| after + pressed);
    assert_eq!(let_go, Some(pressed + 40), "checked 20 and 40 ticks after the press; the cart went at 25");
}

/// bds-i2rails.txt: summoned at y -26 a cart was shown at -25.65, then at -25.5 on its rail.
#[test]
fn a_minecart_rides_its_rail_with_its_middle_half_a_block_up() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| assert!(ctx.set_block(RAIL, &Block::new("rail"))));
    h.tick(1);
    let cart = spawned(&mut h, &[("minecraft:minecart", [3.5, 65.0, 0.5])])[0];
    h.tick(1);
    let feet = Arc::new(Mutex::new(None));
    let seen = feet.clone();
    h.remote.run(move |ctx| *seen.lock().unwrap() = ctx.entity(cart).map(|cart| cart.feet[1]));
    h.tick(1);
    let feet = feet.lock().unwrap().expect("the cart");
    assert!((feet - 65.15).abs() < 1e-4, "its box 0.7 high, its middle at 65.5: {feet}");
}

fn feet(h: &mut Harness, carts: Vec<sapling_api::EntityId>) -> Vec<[f32; 3]> {
    let feet = Arc::new(Mutex::new(Vec::new()));
    let seen = feet.clone();
    h.remote.run(move |ctx| *seen.lock().unwrap() = carts.iter().map(|cart| ctx.entity(*cart).expect("the cart").feet).collect());
    h.tick(1);
    let feet = feet.lock().unwrap().clone();
    feet
}

/// A rail along x from 2 to 12 at z 0 on stone, set as `/fill` does.
fn rail_line(h: &mut Harness) {
    h.remote.run(|ctx| (2..=12).for_each(|x| _ = ctx.set_block([x, 64, 0], &Block::new("stone"))));
    h.tick(1);
    h.remote.run(|ctx| (2..=12).for_each(|x| assert!(ctx.update_block([x, 65, 0], &Block::new("rail").with("rail_direction=1")))));
    h.tick(1);
}

/// bds-l2carts.txt: two carts summoned 0.4 apart each moved 0.0576 apart in their first tick and 0.0599
/// in the next, both pushed twice a tick and keeping a fifth of their motion.
#[test]
fn two_carts_summoned_overlapping_push_each_other_apart() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    rail_line(&mut h);
    let carts = spawned(&mut h, &[("minecraft:minecart", [4.5, 65.0, 0.5]), ("minecraft:minecart", [4.9, 65.0, 0.5])]);
    let start = feet(&mut h, carts.clone());
    let moved: Vec<[f32; 2]> = (0..2)
        .map(|_| {
            let before = feet(&mut h, carts.clone());
            let after = feet(&mut h, carts.clone());
            [after[0][0] - before[0][0], after[1][0] - before[1][0]]
        })
        .collect();
    assert!((start[1][0] - start[0][0] - 0.4).abs() < 1e-5, "not pushed in their first tick: {start:?}");
    for [west, east] in moved {
        assert!((west + 0.0599).abs() < 0.002 && (east - 0.0599).abs() < 0.002, "{west} {east}");
    }
}

/// bds-l2carts.txt: a cart summoned 0.4 from one at rest snapped onto its rail in its first tick, and both
/// were first pushed in the tick after, 0.0576 each.
#[test]
fn a_new_cart_is_pushed_only_from_the_tick_after_its_first() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    rail_line(&mut h);
    let first = spawned(&mut h, &[("minecraft:minecart", [4.5, 65.0, 0.5])])[0];
    (0..20).for_each(|_| _ = h.tick(1));
    let second = spawned(&mut h, &[("minecraft:minecart", [4.9, 65.0, 0.5])])[0];
    let first_tick = feet(&mut h, vec![first, second]);
    let next = feet(&mut h, vec![first, second]);
    assert!((first_tick[0][0] - 4.5).abs() < 1e-5 && (first_tick[1][0] - 4.9).abs() < 1e-5 && (first_tick[1][1] - 65.15).abs() < 1e-4, "{first_tick:?}");
    assert!((first_tick[0][0] - next[0][0] - 0.0576).abs() < 1e-4 && (next[1][0] - first_tick[1][0] - 0.0576).abs() < 1e-4, "{next:?}");
}

/// bds-l2carts.txt: a cart summoned 0.4 from one at rest pushed it exactly as hard as it was pushed, the
/// new cart's snap up onto its rail costing it no speed.
#[test]
fn a_cart_summoned_beside_one_at_rest_and_it_part_evenly() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    rail_line(&mut h);
    let first = spawned(&mut h, &[("minecraft:minecart", [4.5, 65.0, 0.5])])[0];
    (0..20).for_each(|_| _ = h.tick(1));
    let second = spawned(&mut h, &[("minecraft:minecart", [4.9, 65.0, 0.5])])[0];
    let at: Vec<[f32; 3]> = (0..6).flat_map(|_| feet(&mut h, vec![first, second])).collect();
    for tick in at.chunks(2) {
        assert!((4.5 - tick[0][0] - (tick[1][0] - 4.9)).abs() < 1e-4, "{at:?}");
    }
}

/// bds-l2carts.txt: a cart rolling at 0.4 a tick into one at rest stopped at its side, then both moved
/// 0.0576 apart and slowed by the rail's drag.
#[test]
fn a_rolling_cart_stops_at_the_side_of_one_at_rest_and_both_bounce_apart() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    rail_line(&mut h);
    // As BDS's, the rolling cart goes from 1.228 apart, out of push reach, straight to touching; at 9.5 it
    // came 1.004 apart, in reach (bds-p2carts.txt), and was pushed back before it touched.
    let carts = spawned(&mut h, &[("minecraft:minecart", [3.5, 65.0, 0.5]), ("minecraft:minecart", [6.5, 65.0, 0.5])]);
    let rolling = carts[0];
    h.remote.run(move |ctx| assert!(ctx.set_velocity(rolling, [0.4, 0.0, 0.0])));
    let mut met = None;
    for _ in 0..40 {
        let at = feet(&mut h, carts.clone());
        if met.is_none() && at[1][0] - at[0][0] < 0.99 {
            met = Some(at);
        }
    }
    let met = met.expect("met");
    assert!((met[1][0] - met[0][0] - 0.98).abs() < 0.01, "stopped at its side: {met:?}");
    let end = feet(&mut h, carts.clone());
    assert!(end[0][0] < met[0][0] && end[1][0] > met[1][0] + 0.3, "bounced apart: {met:?} {end:?}");
}

/// bds-l2carts.txt: a cart rolling east at 0.4 along a flat rail stopped at 12.51 against a stone at 13.
#[test]
fn a_cart_rolling_into_a_block_at_its_rails_end_stops_against_it() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    rail_line(&mut h);
    h.remote.run(|ctx| assert!(ctx.set_block([13, 65, 0], &Block::new("stone"))));
    let cart = spawned(&mut h, &[("minecraft:minecart", [8.5, 65.0, 0.5])])[0];
    h.remote.run(move |ctx| assert!(ctx.set_velocity(cart, [0.4, 0.0, 0.0])));
    (0..30).for_each(|_| _ = h.tick(1));
    let at = feet(&mut h, vec![cart])[0];
    assert!((at[0] - 12.51).abs() < 1e-3 && (at[1] - 65.15).abs() < 1e-3, "{at:?}");
}

/// bds-i2rails.txt, bds-l2carts.txt: a slope set by command beside the rails `/fill` laid joined them flat,
/// so a cart summoned on it stayed.
#[test]
fn a_slope_set_beside_rails_filled_after_it_turns_flat_and_its_cart_stays() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    h.remote.run(|ctx| {
        (1..=8).for_each(|x| _ = ctx.set_block([x, 64, 0], &Block::new("stone")));
        assert!(ctx.update_block([1, 65, 0], &Block::new("stone")));
        assert!(ctx.update_block([2, 65, 0], &Block::new("rail").with("rail_direction=3")));
    });
    h.tick(1);
    h.remote.run(|ctx| (3..=8).for_each(|x| assert!(ctx.update_block([x, 65, 0], &Block::new("rail").with("rail_direction=1")))));
    h.tick(1);
    let shape = Arc::new(Mutex::new(String::new()));
    let seen = shape.clone();
    h.remote.run(move |ctx| *seen.lock().unwrap() = ctx.block([2, 65, 0]).map(|rail| rail.properties).unwrap_or_default());
    h.tick(1);
    assert_eq!(*shape.lock().unwrap(), "rail_direction=1");
    let cart = spawned(&mut h, &[("minecraft:minecart", [2.5, 65.0, 0.5])])[0];
    let start = feet(&mut h, vec![cart]);
    (0..40).for_each(|_| _ = h.tick(1));
    assert!((feet(&mut h, vec![cart])[0][0] - start[0][0]).abs() < 1e-4, "stays");
}

#[test]
fn a_killed_minecart_leaves_its_item() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let cart = spawned(&mut h, &[("minecraft:minecart", [3.5, 65.0, 0.5])])[0];
    h.tick(1);
    h.tick(1);
    h.remote.run(move |ctx| assert!(ctx.kill_entity(cart)));
    let sent: Vec<_> = (0..2).flat_map(|_| h.tick(1)).collect();
    assert_eq!(all::<AddItemEntity>(&sent).len(), 1, "the minecart item");
}

/// bds-r3cart.txt: a cart off a powered runway at 1.18 went 0.4 a tick across as it fell 0.04, 0.08, ...,
/// then 0.2, 0.1, 0.05 on the ground.
#[test]
fn a_cart_off_a_rail_goes_at_most_0_4_across_with_no_drag_in_the_air_and_halves_on_the_ground() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    let cart = spawned(&mut h, &[("minecraft:minecart", [3.5, 66.0, 0.5])])[0];
    h.remote.run(move |ctx| assert!(ctx.set_velocity(cart, [1.2, 0.0, 0.0])));
    let at: Vec<[f32; 3]> = (0..14).map(|_| feet(&mut h, vec![cart])[0]).collect();
    let across: Vec<f32> = at.windows(2).map(|pair| pair[1][0] - pair[0][0]).collect();
    let landed = at.iter().position(|at| at[1] <= 65.0 + 1e-4).expect("lands");
    assert!(across[..landed].iter().all(|step| (step - 0.4).abs() < 1e-4), "{across:?} {at:?}");
    assert!((across[landed] - 0.2).abs() < 1e-4 && (across[landed + 1] - 0.1).abs() < 1e-4, "{across:?} {at:?}");
}

/// The x each tick of two carts summoned on the rail line `apart` east of 4.5, from their first tick.
fn parting(apart: f32, ticks: usize) -> Vec<[f32; 2]> {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    rail_line(&mut h);
    let pair = [("minecraft:minecart", [4.5, 65.0, 0.5]), ("minecraft:minecart", [4.5 + apart, 65.0, 0.5])];
    let carts = spawned(&mut h, Box::leak(Box::new(pair)));
    (0..ticks).map(|_| feet(&mut h, carts.clone())).map(|at| [at[0][0], at[1][0]]).collect()
}

/// bds-p2carts.txt: carts a block apart, their boxes 0.02 short of touching, pushed each other 0.0576,
/// then 0.05386 and 0.05171 as they drew apart.
#[test]
fn carts_a_block_apart_push_each_other_from_a_fifth_of_a_block_off() {
    let at = parting(1.0, 4);
    let steps: Vec<f32> = at.windows(2).map(|pair| pair[1][1] - pair[0][1]).collect();
    for (step, bds) in steps.iter().zip([0.0576, 0.05386, 0.05171]) {
        assert!((step - bds).abs() < 1e-4, "{steps:?}");
    }
}

/// bds-p2carts.txt: carts summoned 0.2 apart drew apart 0.0599 a tick until their boxes overlapped by 0.066,
/// were parted by that whole overlap each in one tick (0.12588) and stopped, then pushed on 0.04941.
#[test]
fn carts_overlapping_a_little_are_parted_at_once_and_stop() {
    let at = parting(0.2, 10);
    let steps: Vec<f32> = at.windows(2).map(|pair| pair[1][1] - pair[0][1]).collect();
    let jump = steps.iter().position(|step| *step > 0.1).expect("a parting tick");
    assert!((steps[jump] - 0.12588).abs() < 2e-4 && (steps[jump + 1] - 0.04941).abs() < 2e-4, "{steps:?}");
    assert!(at.iter().all(|[west, east]| (4.5 - west - (east - 4.7)).abs() < 1e-4), "evenly: {at:?}");
}

/// bds-p2carts.txt: a cart running up a slope against a stone set where the slope's next rail was stopped
/// at 12.51 against it, its box clear of the slope's own block, then rolled back down.
#[test]
fn a_cart_running_up_a_slope_into_a_block_stops_against_it_and_rolls_back() {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    rail_line(&mut h);
    h.remote.run(|ctx| {
        assert!(ctx.set_block([12, 65, 0], &Block::new("rail").with("rail_direction=2")));
        assert!(ctx.set_block([13, 65, 0], &Block::new("stone")) && ctx.set_block([13, 66, 0], &Block::new("stone")));
    });
    let cart = spawned(&mut h, &[("minecraft:minecart", [8.5, 65.0, 0.5])])[0];
    h.remote.run(move |ctx| assert!(ctx.set_velocity(cart, [0.4, 0.0, 0.0])));
    let at: Vec<[f32; 3]> = (0..30).map(|_| feet(&mut h, vec![cart])[0]).collect();
    let top = at.iter().map(|at| at[0]).fold(f32::MIN, f32::max);
    assert!((top - 12.51).abs() < 1e-3 && at[29][0] < top - 0.05 && at.iter().all(|at| at[1] < 66.0), "{at:?}");
}
