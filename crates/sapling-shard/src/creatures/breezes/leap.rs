//! A breeze's leap as BDS's `jump_around_target` picks it (bdsre 149fb7240, 14890ff00, 149fb78e0,
//! 14890f450): prey 4 to 20 off, a spot 2 to 2.83 off it behind the way it looks, give or take a
//! spread, snapped to a floor within 10 up or down, and the first of the shuffled angles whose
//! parabola meets that floor's top + 1 at 1.4 or less. The parabola has no first-tick lift, so the
//! breeze lands past it. All five launches in research/captures/bds-breeze.txt fit to 1e-4.

use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use sapling_api::Effect;

use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, Position};
use super::super::{afflictions, hunting, legs};

/// It leaps at prey further than the first and nearer than the second.
const VALID: [f32; 2] = [4.0, 20.0];
/// The pack's landing distance is 4 to 8; BDS takes these square roots of it.
const LANDS: [f32; 2] = [2.0, std::f32::consts::SQRT_2 * 2.0];
/// Degrees; the spot's bearing is off by half this times a normal draw.
const SPREAD: f32 = 90.0;
/// Air cells it needs above its feet, how far up or down a floor is looked for, and how much
/// above a spot it may see instead where it does not see the spot.
const HEADROOM: i32 = 4;
const SNAP: i32 = 10;
const SEES_OVER: i32 = 4;
const ANGLES: [f32; 5] = [40.0, 55.0, 60.0, 75.0, 80.0];
const FASTEST: f32 = 1.4;
const GRAVITY: f32 = 0.08;
/// Jump boost's power a level, added after the solve (research/captures/bds-g10breeze.txt, V: 0.5).
const BOOST: f32 = 0.1;

/// The speed of a leap at prey standing at `feet`, picked as the breeze readies; none where it
/// may not leap.
pub(super) fn pick(world: &mut World, entity: Entity, layer: Entity, body: &Body, feet: [f32; 3]) -> Option<[f32; 3]> {
    let far = hunting::distance(body.at, feet);
    if far <= VALID[0] || far >= VALID[1] {
        return None;
    }
    let looks = looks(world, entity);
    let boost = afflictions::level(world, entity, Effect::JumpBoost).map_or(0.0, |level| BOOST * f32::from(level + 1));
    world.get_resource_or_insert_with(Dice::default);
    world.resource_scope(|world, mut dice: Mut<Dice>| {
        let level = &world.get::<Layer>(layer)?.world;
        let [x, y, z] = body.at.map(|c| c.floor() as i32);
        if !(1..=HEADROOM).all(|up| level.registry().get(level.block(x, y + up, z)).is_some_and(|state| state.is_air())) {
            return None;
        }
        let [x, air, z] = floor(level, spot(&mut dice, feet, looks))?;
        if level.registry().get(level.block(x, air, z)).is_some_and(|state| state.is_lava()) {
            return None;
        }
        let eyes = [body.at[0], body.at[1] + hunting::HEAD * body.size[1], body.at[2]];
        let sees = |up: i32| hunting::sees(level, eyes, [x as f32, (air + up) as f32, z as f32]);
        if !sees(2) && !sees(1 + SEES_OVER) {
            return None;
        }
        let to = aim(body.at, [x, air + 1, z]);
        let mut angles = ANGLES;
        for at in 1..angles.len() {
            angles.swap(at, dice.roll(at as u32 + 1) as usize);
        }
        angles.into_iter().find_map(|angle| launch(body.at, to, angle)).map(|speed| boosted(speed, boost))
    })
}

/// `speed` with jump boost's `boost` added along it, kept to its rise (149fb78e0 after the solve).
fn boosted([x, y, z]: [f32; 3], boost: f32) -> [f32; 3] {
    let length = (x * x + y * y + z * z).sqrt();
    let up = if length >= 1e-4 { y / length } else { 0.0 };
    [x, y + boost * up, z]
}

/// The yaw the breeze's prey looks along; BDS's default where it has none is 0.
fn looks(world: &World, entity: Entity) -> f32 {
    let Some((prey, _)) = hunting::quarry(world, entity) else { return 0.0 };
    world.get::<Position>(prey).map(|at| at.head_yaw).or_else(|| world.get::<legs::Legs>(prey).map(|legs| legs.yaw)).unwrap_or(0.0)
}

/// The spot it would land on, before it is snapped to a floor: a normal draw, then a fraction.
fn spot(dice: &mut Dice, feet: [f32; 3], looks: f32) -> [f32; 3] {
    let bearing = (looks + 180.0 + SPREAD * dice.gaussian() * 0.5).to_radians();
    let far = LANDS[0] + dice.fraction() * (LANDS[1] - LANDS[0]);
    [feet[0] - far * bearing.sin(), feet[1], feet[2] + far * bearing.cos()]
}

/// The air cell over a floor in `spot`'s column nearest its height, below before above.
fn floor(level: &Level, spot: [f32; 3]) -> Option<[i32; 3]> {
    let [x, y, z] = spot.map(|c| c.floor() as i32);
    let solid = |y: i32| level.registry().get(level.block(x, y, z)).is_some_and(|state| state.is_solid());
    (0..SNAP).flat_map(|i| [y - i, y + i]).find(|&y| solid(y) && !solid(y + 1)).map(|y| [x, y + 1, z])
}

/// Where BDS aims for the cell `[x, y, z]`: its middle pulled half a block towards `from`.
fn aim(from: [f32; 3], [x, y, z]: [i32; 3]) -> [f32; 3] {
    let (middle_x, middle_z) = (x as f32 + 0.5, z as f32 + 0.5);
    // BDS's pull takes its x for both axes (length sqrt(2 dx²)): kept, the launches show it.
    let dx = middle_x - from[0];
    let length = (dx * dx + dx * dx).sqrt();
    let pull = if length >= 1e-4 { dx / length * 0.5 } else { 0.0 };
    [middle_x - pull, y as f32, middle_z - pull]
}

/// The speed from `from` at `angle` degrees whose parabola passes `to`, if at most [`FASTEST`].
fn launch(from: [f32; 3], to: [f32; 3], angle: f32) -> Option<[f32; 3]> {
    let [dx, dy, dz] = std::array::from_fn(|axis| to[axis] - from[axis]);
    let across_squared = dx * dx + dz * dz;
    let (sin, cos) = angle.to_radians().sin_cos();
    let speed = (across_squared * GRAVITY / (2.0 * sin * cos * across_squared.sqrt() - 2.0 * dy * cos * cos)).abs().sqrt();
    (speed <= FASTEST).then(|| {
        let yaw = dz.atan2(dx);
        [speed * cos * yaw.cos(), speed * sin, speed * cos * yaw.sin()]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_launches() {
        let close = |a: [f32; 3], b: [f32; 3]| (0..3).all(|axis| (a[axis] - b[axis]).abs() < 5e-4);
        let first = [0.499_999_94, -19.0, -7.5];
        assert!(close(launch(first, aim(first, [0, -18, -2]), 40.0).unwrap(), [0.0, 0.501_268, 0.597_478]));
        let fourth = [2.521_084, -19.0, -4.122_596];
        assert!(close(launch(fourth, aim(fourth, [-1, -18, -3]), 80.0).unwrap(), [-0.126_371, 0.891_687, 0.093_629]));
    }

    #[test]
    fn jump_boost_lifts_a_launch_by_its_power_along_it() {
        // bds-g10breeze.txt's three launches under jump boost V: 55, 75 and 40 degrees solved, 0.5 added.
        for (angle, across, shown) in [(55.0_f32, 0.521_81_f32, 1.154_72_f32), (75.0, 0.250_25, 1.416_84), (40.0, 0.705_74, 0.913_46)] {
            let rise = across * angle.to_radians().tan();
            let [_, y, _] = boosted([across, rise, 0.0], 0.5);
            assert!((y - shown).abs() < 5e-4, "{angle}: {y} against {shown}");
        }
    }
}
