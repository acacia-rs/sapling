use super::*;

/// A void world with stone at y 64 and an oak slab (bottom half) at (2, 65, 0).
fn level() -> Level {
    let mut level = Level::void();
    let stone = level.block_id("minecraft:stone").unwrap();
    level.fill([-4, 64, -4], [4, 64, 4], stone);
    let slab = level.registry().find("minecraft:oak_slab", "minecraft:vertical_half=bottom").unwrap();
    level.set_block(2, 65, 0, slab);
    level
}

fn fall(mut body: Body, physics: &Physics, ticks: usize) -> Body {
    let level = level();
    (0..ticks).for_each(|_| _ = body.step(&level, physics, &mut || 0.5));
    body
}

#[test]
fn a_body_falls_onto_the_floor_and_rests_on_it() {
    let body = fall(Body::new([0.5, 68.0, 0.5], [0.0; 3], [0.98, 0.98]), &Physics::FALLING_BLOCK, 40);
    assert_eq!((body.at, body.on_ground), ([0.5, 65.0, 0.5], true));
    let on_slab = fall(Body::new([2.5, 68.0, 0.5], [0.0; 3], [0.25, 0.25]), &Physics::ITEM, 60);
    assert_eq!(on_slab.at[1], 65.5, "a slab's top is half a block up");
}

#[test]
fn an_orb_lands_with_no_fall_left_and_crawls_on_without_bouncing() {
    let level = level();
    let mut orb = Body::new([0.5, 66.0, 0.5], [0.0, 0.3, 0.15], [0.25, 0.25]);
    let mut landed = None;
    for tick in 0..40 {
        _ = orb.step(&level, &Physics::ORB, &mut || 0.5);
        if orb.on_ground && landed.is_none() {
            landed = Some(tick);
            assert_eq!(orb.motion[1], 0.0, "BDS told motion y -0.0 as its orb landed (bds-n13net.txt)");
        }
        assert!(landed.is_none() || orb.on_ground, "stays down from tick {landed:?}, at {tick}");
    }
    assert!(landed.is_some() && orb.motion[2] > 0.0, "it slides on, slowing");
}

#[test]
fn falling_takes_the_games_ticks_and_speed() {
    // 0.04 a tick, 2% drag: three blocks take 13 ticks from rest; BDS's sand, announced a
    // tick into its fall, landed 12 after that.
    let body = Body::new([0.5, 68.0, 0.5], [0.0; 3], [0.98, 0.98]);
    let ticks = (1..).find(|ticks| fall(body, &Physics::FALLING_BLOCK, *ticks).on_ground).unwrap();
    assert_eq!(ticks, 13);
}

#[test]
fn a_body_pushed_up_onto_a_block_is_told_standing_only_a_tick_later() {
    // BDS's stand on a piston pushing up: 66.0 not on the ground, then nothing until it falls (bds-shove.txt).
    let mut level = level();
    let stone = level.block_id("minecraft:stone").unwrap();
    level.set_block(0, 65, 0, stone);
    let mut body = Body::new([0.5, 65.5, 0.5], [0.0; 3], [0.5, 1.975]);
    body.push(&level, [0.0, 0.5, 0.0], &[]);
    body.step(&level, &Physics::MOB, &mut || 0.5);
    assert_eq!((body.at[1], body.on_ground), (66.0, false));
    body.step(&level, &Physics::MOB, &mut || 0.5);
    assert_eq!((body.at[1], body.on_ground), (66.0, true));
}

#[test]
fn a_wall_stops_a_body_and_ground_friction_slows_it() {
    let mut level = level();
    let stone = level.block_id("minecraft:stone").unwrap();
    level.set_block(1, 65, 3, stone);
    let mut body = Body::new([0.5, 65.0, 3.5], [0.5, 0.0, 0.0], [0.25, 0.25]);
    body.step(&level, &Physics::ITEM, &mut || 0.5);
    assert!((body.at[0] - 0.875).abs() < 1e-6 && body.motion[0] == 0.0, "stopped at the wall: {body:?}");
    let mut sliding = Body::new([-2.5, 65.0, 0.5], [0.2, 0.0, 0.0], [0.25, 0.25]);
    sliding.step(&level, &Physics::ITEM, &mut || 0.5);
    assert!(sliding.on_ground && (sliding.motion[0] - 0.2 * 0.588).abs() < 1e-6, "{sliding:?}");
}

#[test]
fn swimmers_and_bats_settle_at_their_kinds_speed() {
    // bds-wander-findings.md: a cod pushed 0.0098 a tick settled at 0.049, a turtle pushed 0.0141 at
    // 0.141, a bat pushed 0.1 up at 0.25.
    let mut level = Level::void();
    let (stone, water) = (level.block_id("minecraft:stone").unwrap(), level.block_id("minecraft:water").unwrap());
    level.fill([-16, 59, -2], [16, 59, 2], stone);
    level.fill([-16, 60, -2], [16, 64, 2], water);
    for (physics, push, settles) in [(&Physics::SWIMMER, [0.0098, 0.0, 0.0], 0.049), (&Physics::TURTLE, [0.0141, 0.0, 0.0], 0.141), (&Physics::BAT, [0.0, 0.1, 0.0], 0.25)] {
        let mut body = Body::new([-14.5, if physics.fall_drag == 0.6 { 70.0 } else { 62.0 }, 0.5], [0.0; 3], [0.5, 0.3]);
        for _ in 0..80 {
            body.fly(push);
            body.step(&level, physics, &mut || 0.5);
        }
        let speed = body.motion[0] + body.motion[1];
        assert!((speed - settles).abs() < 1e-3, "{speed} for {settles}: {body:?}");
    }
}

#[test]
fn a_mob_made_in_water_sinks_by_the_waters_rule_from_its_first_tick() {
    // bds-r1water.txt: a zombie summoned in water went down 0.02, 0.036, 0.0488 (sapling 0.0784 first).
    let mut level = Level::void();
    let (stone, water) = (level.block_id("minecraft:stone").unwrap(), level.block_id("minecraft:water").unwrap());
    level.fill([-2, 59, -2], [2, 59, 2], stone);
    level.fill([-2, 60, -2], [2, 64, 2], water);
    let mut zombie = Body::new([0.5, 61.0, 0.5], [0.0; 3], [0.6, 1.95]);
    let moved: Vec<f32> = (0..3).map(|_| zombie.step(&level, &Physics::MOB, &mut || 0.5)[1]).collect();
    assert!(moved.iter().zip([-0.02, -0.036, -0.0488]).all(|(got, bds)| (got - bds).abs() < 1e-4), "{moved:?}");
}

#[test]
fn a_mobs_jump_is_told_after_its_ticks_drag_and_gravity_as_on_bds() {
    // bds-chase.txt: a zombie's jump of 0.42 up told 0.3332, its 0.114 across 0.0623.
    let level = level();
    let mut body = Body::new([-2.5, 65.0, 0.5], [0.0; 3], [0.6, 1.95]);
    (0..2).for_each(|_| _ = body.step(&level, &Physics::MOB, &mut || 0.5));
    body.walk([0.114, 0.0], Some(0.42));
    body.step(&level, &Physics::MOB, &mut || 0.5);
    let held = body.held_motion(&Physics::MOB);
    assert!((held[1] - 0.3332).abs() < 1e-4 && (held[0] - 0.114 * Physics::MOB.ground_drag).abs() < 1e-6, "{held:?}");
    let mut landed = body;
    (0..15).for_each(|_| _ = landed.step(&level, &Physics::MOB, &mut || 0.5));
    assert!(landed.on_ground && (landed.held_motion(&Physics::MOB)[1] + 0.0784).abs() < 1e-6);
}

#[test]
fn a_chicken_falls_as_slowly_as_bds_dropped_one() {
    // research/captures/bds-bodies.txt: a chicken let go 10 up in the open.
    let level = level();
    let mut body = Body::new([0.5, 75.0, 0.5], [0.0; 3], [0.4, 0.7]);
    let moved: Vec<f32> = (0..5).map(|_| body.step(&level, &Physics::CHICKEN, &mut || 0.5)[1]).collect();
    let bds = [-0.078419, -0.093145, -0.101809, -0.106903, -0.109899];
    assert!(moved.iter().zip(bds).all(|(a, b)| (a - b).abs() < 1e-4), "{moved:?}");
}

#[test]
fn a_mob_in_a_block_lower_than_a_full_one_rises_onto_it_and_stays_in_a_full_one() {
    // research/captures/bds-lifts.txt: a cow stood on as a slab was set rose to its top the tick after; in stone it stayed.
    let mut level = level();
    let stone = level.block_id("minecraft:stone").unwrap();
    level.set_block(0, 65, 2, stone);
    let mut cow = Body::new([2.5, 65.0, 0.5], [0.0; 3], [0.9, 1.3]);
    cow.step(&level, &Physics::MOB, &mut || 0.5);
    assert_eq!((cow.at[1], cow.on_ground), (65.5, true));
    let mut walled = Body::new([0.5, 65.0, 2.5], [0.0; 3], [0.9, 1.3]);
    walled.step(&level, &Physics::MOB, &mut || 0.5);
    assert_eq!(walled.at[1], 65.0);
    for (name, top) in [("minecraft:soul_sand", 65.875), ("minecraft:stonecutter_block", 65.5625), ("minecraft:enchanting_table", 65.75)] {
        let block = level.block_id(name).unwrap();
        level.set_block(-2, 65, 0, block);
        let mut cow = Body::new([-1.5, 65.0, 0.5], [0.0; 3], [0.9, 1.3]);
        cow.step(&level, &Physics::MOB, &mut || 0.5);
        assert_eq!(cow.at[1], top, "{name}");
    }
}

#[test]
fn a_mob_in_a_full_block_goes_out_its_nearest_open_side_flush() {
    // research/captures/bds-m3inblock.txt: a cow at x 12.8 in a stone set at its feet went to 13.45 that tick.
    let mut level = level();
    let stone = level.block_id("minecraft:stone").unwrap();
    level.set_block(0, 65, 2, stone);
    let step = |at: [f32; 3], level: &Level| {
        let mut cow = Body::new(at, [0.0; 3], [0.9, 1.3]);
        cow.step(level, &Physics::MOB, &mut || 0.5);
        cow.at
    };
    // The side plus half the width in f32: BDS's 0.16 one was told at 1.4499999.
    assert_eq!(step([0.8, 65.0, 2.5], &level), [1.0 + 0.45, 65.0, 2.5]);
    assert_eq!(step([0.5, 65.0, 2.5], &level), [0.5, 65.0, 3.0 + 0.45], "a tie goes south (bds-bodies.txt)");
    for (x, z) in [(0, 3), (0, 1), (1, 2), (-1, 2)] {
        level.set_block(x, 65, z, stone);
    }
    assert_eq!(step([0.5, 65.0, 2.5], &level), [0.5, 65.0, 2.5], "walled in all round it stays (bds-lifts.txt)");
}

/// A zombie standing 0.1 short of a block walking at it at 0.3 a tick: where it is after a tick.
fn walked_at(top: Option<&str>, physics: &Physics) -> [f32; 3] {
    let mut level = level();
    if let Some(name) = top {
        let block = level.block_id(name).unwrap();
        level.set_block(0, 65, 2, block);
    }
    let mut zombie = Body::new([0.5, 65.0, 1.6], [0.0; 3], [0.6, 1.9]);
    zombie.on_ground = true;
    zombie.step(&level, physics, &mut || 0.5);
    zombie.walk([0.0, 0.3], None);
    zombie.step(&level, physics, &mut || 0.5);
    zombie.at
}

#[test]
fn a_walking_mob_steps_up_onto_a_block_as_high_as_its_max_auto_step() {
    let on = walked_at(Some("minecraft:daylight_detector"), &Physics::MOB);
    assert!(on[1] == 65.375 && on[2] > 1.7, "onto a detector's 0.375: {on:?}");
    let full = walked_at(Some("minecraft:stone"), &Physics::MOB);
    assert!(full[1] == 65.0 && full[2] <= 1.7 + 1e-5, "a full block stops it: {full:?}");
    assert_eq!(walked_at(Some("minecraft:stone"), &Physics::LONG_STEP)[1], 66.0, "an enderman's 1.0625 takes a full block");
    let carpet = walked_at(Some("minecraft:white_carpet"), &Physics { step: 0.0, ..Physics::MOB });
    assert_eq!(carpet[1], 65.0, "with no step a carpet stops it");
}

#[test]
fn a_mob_whose_box_clears_an_edge_stood_on_it_that_tick_and_falls_the_next() {
    let level = level();
    let mut zombie = Body::new([5.25, 65.0, 0.5], [0.0; 3], [0.6, 1.9]);
    zombie.on_ground = true;
    zombie.step(&level, &Physics::MOB, &mut || 0.5);
    zombie.walk([0.12, 0.0], None);
    zombie.step(&level, &Physics::MOB, &mut || 0.5);
    assert!(zombie.at[0] - 0.3 > 5.0 && zombie.at[1] == 65.0 && zombie.on_ground, "off the edge, still stood (bds-bc1step.txt): {zombie:?}");
    let x = zombie.at[0];
    zombie.walk([0.12, 0.0], None);
    zombie.step(&level, &Physics::MOB, &mut || 0.5);
    assert!((zombie.at[1] - (65.0 - 0.0784)).abs() < 1e-5 && !zombie.on_ground, "one tick's fall: {zombie:?}");
    assert!(zombie.at[0] - x > 0.12, "walked on at its ground pace: {zombie:?}");
}

#[test]
fn a_move_a_block_cuts_short_is_a_collision_and_one_alongside_a_block_is_not() {
    // BDS's CollisionFlagComponent (FinalizeMoveSystem): any axis of the move cut short.
    let level = level();
    let mut body = Body::new([1.5, 65.0, 0.5], [0.0; 3], [0.6, 0.6]);
    body.shove(&level, [0.5, 0.0, 0.0]);
    assert!(body.collided() && (body.at[0] - 1.7).abs() < 1e-5, "stopped by the slab: {body:?}");
    body.shove(&level, [0.0, 0.0, 0.1]);
    assert!(!body.collided(), "slid along it, touching: {body:?}");
    body.shove(&level, [0.0, -0.1, 0.0]);
    assert!(body.collided(), "the floor stopped it: {body:?}");
}
