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
