//! Group U of the fourteenth parity brief: pistons carrying and flinging what stands on their blocks
//! (research/captures/bds-u3carry.txt), hoppers and comparators with minecarts (bds-ffcart.txt), and
//! `/damage` on vehicles (bds-ffburn.txt).

use std::sync::{Arc, Mutex};

use crate::common;
use crate::riding::spawned;

use common::Harness;
use sapling_api::{Block, BlockPos, DamageCause, EntityId, Item, MobSlot};

fn ready(blocks: &'static [(BlockPos, &'static str, &'static str)]) -> Harness {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    put(&mut h, blocks);
    ticks(&mut h,2);
    h
}

fn ticks(h: &mut Harness, count: usize) {
    (0..count).for_each(|_| _ = h.tick_all());
}

fn put(h: &mut Harness, blocks: &'static [(BlockPos, &'static str, &'static str)]) {
    h.remote.run(move |ctx| {
        for (pos, name, properties) in blocks {
            assert!(ctx.set_block(*pos, &Block::new(*name).with(*properties)), "{pos:?} {name}");
        }
    });
}

/// The feet of `ids` after each of the next `ticks` ticks.
fn track(h: &mut Harness, ids: Vec<EntityId>, ticks: usize) -> Vec<Vec<[f32; 3]>> {
    (0..ticks)
        .map(|_| {
            let seen = Arc::new(Mutex::new(Vec::new()));
            let slot = seen.clone();
            let ids = ids.clone();
            common::asking::ask(h, move |ctx| *slot.lock().unwrap() = ids.iter().map(|id| ctx.entity(*id).map_or([f32::NAN; 3], |info| info.feet)).collect());
            let feet = seen.lock().unwrap().clone();
            feet
        })
        .collect()
}

fn steps(seen: &[Vec<[f32; 3]>], body: usize, axis: usize) -> Vec<f32> {
    let mut places: Vec<f32> = seen.iter().map(|feet| feet[body][axis]).collect();
    places.dedup();
    places
}

#[test]
fn a_stand_on_a_stone_pushed_sideways_goes_with_it_then_before_its_front_and_one_pulled_goes_back() {
    // bds-u3carry.txt: on a pushed stone 3.5, 4.5 in the tick it starts, then 5.25 before its front,
    // falling; on a pulled one 4.5, 4.0, 3.0.
    let mut h = ready(&[([0, 65, 0], "piston", "facing_direction=4"), ([1, 65, 0], "stone", ""), ([0, 65, 4], "sticky_piston", "facing_direction=4"), ([1, 65, 4], "stone", "")]);
    let pushed = spawned(&mut h, &[("minecraft:armor_stand", [1.5, 66.0, 0.5])]);
    ticks(&mut h,2);
    put(&mut h, &[([0, 66, 0], "redstone_block", ""), ([0, 66, 4], "redstone_block", "")]);
    let seen = track(&mut h, pushed, 30);
    assert_eq!(steps(&seen, 0, 0), [1.5, 2.5, 3.25]);
    assert_eq!(seen.last().map(|feet| feet[0][1]), Some(65.0), "it falls off");
    let pulled = spawned(&mut h, &[("minecraft:armor_stand", [2.5, 66.0, 4.5])]);
    ticks(&mut h,2);
    h.remote.run(|ctx| assert!(ctx.set_block([0, 66, 4], &Block::air())));
    let seen = track(&mut h, pulled, 10);
    assert_eq!(steps(&seen, 0, 0), [2.5, 2.0, 1.0]);
}

#[test]
fn slime_pushed_up_flings_what_stands_on_it_and_stone_only_lifts_it() {
    // bds-ffshove.txt: SetEntityMotion y 0.9016 on slime, a lift of 1 on stone.
    let mut h = ready(&[([6, 65, 0], "piston", "facing_direction=1"), ([6, 66, 0], "slime", ""), ([9, 65, 0], "piston", "facing_direction=1"), ([9, 66, 0], "stone", "")]);
    let stands = spawned(&mut h, &[("minecraft:armor_stand", [6.5, 67.0, 0.5]), ("minecraft:armor_stand", [9.5, 67.0, 0.5])]);
    ticks(&mut h,2);
    put(&mut h, &[([7, 65, 0], "redstone_block", ""), ([10, 65, 0], "redstone_block", "")]);
    let seen = track(&mut h, stands, 12);
    let highest = |body: usize| seen.iter().map(|feet| feet[body][1]).fold(f32::MIN, f32::max);
    assert!(highest(0) > 68.5, "flung: {:?}", steps(&seen, 0, 1));
    assert_eq!(highest(1), 68.0);
}

const RAIL: BlockPos = [3, 65, 0];

#[test]
fn a_hopper_fills_a_cart_under_it_and_one_under_a_cart_empties_it() {
    // bds-ffcart.txt: a hopper over a chest minecart filled it; one under emptied it.
    let mut h = ready(&[(RAIL, "rail", ""), ([3, 66, 0], "hopper", "facing_direction=0"), ([6, 65, 0], "rail", ""), ([6, 64, 0], "hopper", "facing_direction=0")]);
    let carts = spawned(&mut h, &[("minecraft:chest_minecart", [3.5, 65.0, 0.5]), ("minecraft:chest_minecart", [6.5, 65.0, 0.5])]);
    let below = carts[1];
    h.remote.run(move |ctx| {
        assert!(ctx.set_container_item([3, 66, 0], 0, Some(&Item::new("minecraft:dirt", 1))));
        assert_eq!(ctx.set_mob_item(below, MobSlot::Inventory(0), Some(&Item::new("minecraft:dirt", 1))), Ok(()));
    });
    ticks(&mut h,20);
    let slots = Arc::new(Mutex::new((None, None)));
    let seen = slots.clone();
    common::asking::ask(&mut h, move |ctx| *seen.lock().unwrap() = (ctx.container_item([3, 66, 0], 0), ctx.container_item([6, 64, 0], 0)));
    let (above, under) = slots.lock().unwrap().clone();
    assert_eq!(above, None, "the hopper over the cart gave its dirt");
    assert_eq!(under.map(|item| item.count), Some(1), "the hopper under the cart took its dirt");
}

#[test]
fn a_comparator_by_a_detector_rail_reads_a_hopper_carts_fullness() {
    // bds-ffcart.txt: an empty hopper minecart read 0, one stack 3, at the rail's 20-tick checks.
    let mut h = ready(&[(RAIL, "detector_rail", ""), ([4, 65, 0], "unpowered_comparator", "minecraft:cardinal_direction=west"), ([5, 65, 0], "redstone_wire", "")]);
    let cart = spawned(&mut h, &[("minecraft:hopper_minecart", [3.5, 65.0, 0.5])])[0];
    let dust = |h: &mut Harness| {
        let signal = Arc::new(Mutex::new(None));
        let seen = signal.clone();
        common::asking::ask(h, move |ctx| *seen.lock().unwrap() = ctx.block([5, 65, 0]).map(|dust| dust.properties.clone()));
        let signal = signal.lock().unwrap().clone();
        signal.unwrap_or_default()
    };
    ticks(&mut h,30);
    assert!(dust(&mut h).contains("redstone_signal=0"), "an empty cart reads nothing");
    h.remote.run(move |ctx| assert_eq!(ctx.set_mob_item(cart, MobSlot::Inventory(0), Some(&Item::new("minecraft:dirt", 64))), Ok(())));
    ticks(&mut h,45);
    assert!(dust(&mut h).contains("redstone_signal=3"), "{}", dust(&mut h));
}

#[test]
fn a_cart_faces_the_way_it_rolls_round_a_curve() {
    // bds-u3yaw.txt: east 0, on the curve -45 (315), north -90 (270).
    let mut h = ready(&[([0, 65, 0], "rail", "rail_direction=1"), ([1, 65, 0], "rail", "rail_direction=1"), ([2, 65, 0], "rail", "rail_direction=1"), ([3, 65, 0], "rail", "rail_direction=8"), ([3, 65, -1], "rail", "rail_direction=0"), ([3, 65, -2], "rail", "rail_direction=0"), ([3, 65, -3], "rail", "rail_direction=0"), ([3, 65, -4], "rail", "rail_direction=0")]);
    let cart = spawned(&mut h, &[("minecraft:minecart", [0.5, 65.0, 0.5])])[0];
    h.remote.run(move |ctx| assert!(ctx.set_velocity(cart, [0.4, 0.0, 0.0])));
    let yaws: Vec<f32> = (0..20).map(|_| common::asking::ask(&mut h, move |ctx| ctx.entity(cart).map_or(f32::NAN, |info| info.yaw))).collect();
    let near = |want: f32| yaws.iter().any(|yaw| (yaw - want).abs() < 1.5);
    assert!(near(0.0) && near(-45.0) && near(-90.0), "{yaws:?}");
    assert!((yaws.last().copied().unwrap_or(f32::NAN) + 90.0).abs() < 1.5, "{yaws:?}");
}

#[test]
fn damage_shakes_a_boat_and_a_cart_and_fire_primes_a_tnt_cart() {
    // bds-ffburn.txt, bds-fftnt.txt: /damage reached both; fire, lava or burning primed a TNT cart.
    let mut h = ready(&[(RAIL, "rail", "")]);
    let ids = spawned(&mut h, &[("minecraft:boat", [0.5, 65.0, 3.5]), ("minecraft:minecart", [3.5, 65.0, 6.5]), ("minecraft:tnt_minecart", [3.5, 65.0, 0.5])]);
    let took = Arc::new(Mutex::new(Vec::new()));
    let seen = took.clone();
    let hit = ids.clone();
    common::asking::ask(&mut h, move |ctx| *seen.lock().unwrap() = vec![ctx.hurt_entity(hit[0], 1.0, DamageCause::Attack), ctx.hurt_entity(hit[1], 1.0, DamageCause::Attack), ctx.hurt_entity(hit[2], 1.0, DamageCause::Fire)]);
    assert_eq!(*took.lock().unwrap(), [true, true, true]);
    let seen = track(&mut h, ids, 100);
    assert!(seen.last().is_some_and(|feet| feet[2][0].is_nan()), "the primed cart went off");
    assert!(seen[2].iter().all(|feet| !feet[0].is_nan()), "one hit leaves a boat and a cart, and the TNT cart burns first");
}
