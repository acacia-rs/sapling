//! Squids and glow squids in water, as BDS 1.26.52's `SquidIdleGoal::tick` and
//! `Squid::postAiStep` swim them (research/captures/bds-gameplay-findings.md): the goal picks a way every
//! 201 ticks, at once on starting and whenever the squid is above its band and the way rises, and
//! each tick the squid's speed closes 0.22 of the gap to that way after water's 0.8 drag. It
//! turns a tenth of the way to where it is going each tick.
// TODO: BDS's spurts (a push of 0.0715 along -z every 8-15 ticks for a few seconds,
// research/captures/bds-swimmers.txt) come from none of the squid goals or postAiStep; fleeing a
// hit (`squid_flee`), ink, `squid_dive` and `squid_out_of_water` are not copied.

use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use crate::blocks::Dice;
use crate::body::{Body, Liquid};
use crate::components::Layer;
use crate::sync::Told;

const KINDS: &[&str] = &["minecraft:squid", "minecraft:glow_squid"];
const WATER: [&str; 2] = ["minecraft:water", "minecraft:flowing_water"];
/// Ticks of a way before the next is picked.
const KEEPS: u32 = 200;
/// The way's length, 0.5 a second.
const SPEED: f32 = 0.5 / 20.0;
/// A tick closes this much of the gap between the squid's speed and its way.
const CLOSES: f32 = 0.22;
const WATER_DRAG: f32 = 0.8;
const TURNS: f32 = 0.1;

#[derive(Component)]
pub(super) struct Squid {
    way: [f32; 3],
    idle: u32,
    /// Its yaw and pitch, in degrees, unwrapped as BDS keeps them.
    facing: [f32; 2],
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if KINDS.contains(&kind) {
        world.entity_mut(entity).insert(Squid { way: [0.0; 3], idle: KEEPS, facing: [0.0; 2] });
    }
}

/// Before its move: in water a squid swims on its way, now and then another.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity) -> bool {
    let (Some(body), Some(squid)) = (world.get::<Body>(entity).copied(), world.get::<Squid>(entity)) else { return false };
    if body.liquid() != Some(Liquid::Water) {
        return true;
    }
    let (mut way, idle, facing) = (squid.way, squid.idle, squid.facing);
    let band = world.get::<Layer>(layer).map(|layer| band(&layer.world, &body));
    let picks = idle >= KEEPS || band.is_some_and(|[_, high]| body.at[1] > high && way[1] > 0.0);
    if let (true, Some(band)) = (picks, band) {
        way = pick(&mut world.get_resource_or_insert_with(Dice::default), &body, band);
    }
    let speed: [f32; 3] = std::array::from_fn(|axis| body.motion[axis] * WATER_DRAG * (1.0 - CLOSES) + way[axis] * CLOSES);
    let across = speed[0].hypot(speed[2]);
    let facing = [
        facing[0] + ((-speed[0]).atan2(speed[2]).to_degrees() - facing[0]) * TURNS,
        facing[1] + (-across.atan2(speed[1]).to_degrees() - facing[1]) * TURNS,
    ];
    if let Some(mut squid) = world.get_mut::<Squid>(entity) {
        (squid.way, squid.idle, squid.facing) = (way, if picks { 0 } else { idle + 1 }, facing);
    }
    if let Some(mut moving) = world.get_mut::<Body>(entity) {
        moving.fly(std::array::from_fn(|axis| speed[axis] - body.motion[axis] * WATER_DRAG));
    }
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.face(facing[0], facing[0]);
        told.pitch(facing[1]);
    }
    true
}

/// A way across at random and up or down towards a height in the squid's band, 0.5 a second.
fn pick(dice: &mut Dice, body: &Body, [low, high]: [f32; 2]) -> [f32; 3] {
    let (x, z) = (dice.fraction() - 0.5, dice.fraction() - 0.5);
    let y = ((high - low) * dice.fraction() + low - body.at[1]) * 0.1;
    let long = (x * x + y * y + z * z).sqrt();
    let scale = if long > 0.0 { SPEED / long } else { 1.0 / 20.0 };
    [x * scale, y * scale, z * scale]
}

/// The heights a squid keeps to: 5 to 1.5 below the first cell over it that is not water, but
/// half a block over the floor of its water at least.
fn band(level: &Level, body: &Body) -> [f32; 2] {
    let [x, y, z] = body.at.map(|c| c.floor() as i32);
    let is_water = |y: i32| level.in_height(y) && level.registry().get(level.block(x, y, z)).is_some_and(|state| WATER.contains(&state.name));
    let top = (y..).find(|&y| !is_water(y)).unwrap_or(y) as f32;
    let floor = (((y - 64)..=y).rev().find(|&y| !is_water(y)).unwrap_or(y - 64) + 1) as f32;
    let (low, high) = (top - 5.0, top - 1.5);
    [if low < floor { floor + 0.5 } else { low }, if high < floor { floor + 0.5 } else { high }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_squid_settles_at_the_speed_bds_s_drifted_at() {
        // bds-swimmers.txt: a squid's steps closed 0.62 of the gap a tick and settled at 0.0146.
        let (mut motion, way) = ([0.0f32; 3], [SPEED, 0.0, 0.0]);
        for _ in 0..80 {
            motion = std::array::from_fn(|axis| motion[axis] * WATER_DRAG * (1.0 - CLOSES) + way[axis] * CLOSES);
        }
        assert!((motion[0] - 0.01463).abs() < 1e-4, "{motion:?}");
    }
}
