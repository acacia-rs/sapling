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
