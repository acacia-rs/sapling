//! Group P of the fourteenth wave: ports left open (research/captures/bds-j3ride.txt; creatures/jockeys.rs).

use std::sync::{Arc, Mutex};

use crate::common;

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use common::asking::ask;
use common::{Harness, all};
use sapling_api::{Block, Difficulty, Effect, EntityId, GameMode, Mount, PlayerId, Target};

/// Alex in survival at the spawn at midnight on easy; a `rider` summoned with `event` seated on a
/// `mount` 8 south of him.
fn jockey(mount: &'static str, rider: &'static str, event: Option<&'static str>) -> (Harness, EntityId) {
    let mut h = Harness::new();
    h.enter(1, "Alex");
    ask(&mut h, |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        ctx.set_time(18000);
        ctx.set_game_mode(PlayerId(1), GameMode::Survival);
    });
    let mount = ask(&mut h, move |ctx| ctx.summon_as(None, mount, [0.5, 65.0, 6.5], 180.0, 0.0, None)).expect("a mount");
    let seated = ask(&mut h, move |ctx| {
        let rider = ctx.summon_as(None, rider, [0.5, 65.0, 6.5], 180.0, 0.0, event).expect("a rider");
        ctx.ride(&Target::Entity(rider), mount, Mount::RiderToVehicle)
    });
    assert!(seated);
    (h, mount)
}

#[test]
fn a_chicken_jockey_runs_at_its_prey_at_one_and_a_half_times_the_chickens_pace_and_its_rider_hits() {
    // BDS's chicken under a baby zombie came at the actor 0.3035 a tick, its 0.25 times `mount_pathing`'s
    // 1.5 (0.98 x 0.375² over 1 - 0.546), and the baby swung every 20 ticks.
    let (mut h, chicken) = jockey("minecraft:chicken", "minecraft:zombie", Some("minecraft:as_baby"));
    let z = Arc::new(Mutex::new(Vec::new()));
    let mut swings = 0;
    for _ in 0..60 {
        let kept = z.clone();
        h.remote.run(move |ctx| kept.lock().unwrap().push(ctx.entity(chicken).expect("the chicken").feet[2]));
        swings += all::<EntityEvent>(h.tick_all().to(1)).iter().filter(|event| event.event_id == EntityEventEventId::ArmSwing).count();
    }
    let steps: Vec<f32> = z.lock().unwrap().windows(2).map(|pair| pair[0] - pair[1]).collect();
    let fastest = steps.iter().copied().fold(0.0, f32::max);
    assert!((fastest - 0.3035).abs() < 0.01, "{steps:?}");
    assert!(swings >= 1, "the rider hit");
}

#[test]
fn a_small_slime_rises_through_lava() {
    // bds-j3slime.txt: BDS's small slimes rose 0.02, 0.03, 0.035 ... a tick from the bottom of a
    // two-deep pool, about 0.3 in ten ticks, by turns; sapling's sank.
    let mut h = Harness::new();
    h.enter(1, "Alex");
    ask(&mut h, |ctx| {
        ctx.set_difficulty(Difficulty::Easy);
        for y in 64..=67 {
            for x in 1..=5 {
                for z in -2i32..=2 {
                    let inside = y > 64 && (2..=4).contains(&x) && z.abs() <= 1;
                    let block = if !inside { "minecraft:stone" } else if y <= 66 { "minecraft:lava" } else { "minecraft:air" };
                    assert!(ctx.set_block([x, y, z], &Block::new(block)));
                }
            }
        }
    });
    let slime = ask(&mut h, |ctx| {
        let slime = ctx.summon_as(None, "minecraft:slime", [3.5, 65.0, 0.5], 0.0, 0.0, Some("minecraft:spawn_small")).expect("a slime");
        assert!(ctx.give_entity_effect(slime, Effect::FireResistance, 1200, 0));
        slime
    });
    let heights: Vec<f32> = (0..30).map(|_| ask(&mut h, move |ctx| ctx.entity(slime).expect("the slime").feet[1])).collect();
    assert!(heights.iter().copied().fold(0.0, f32::max) > 65.3 && heights.iter().all(|&y| y > 64.99), "{heights:?}");
}
