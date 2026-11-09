//! Squids and glow squids in water, as BDS 1.26.52's `SquidIdleGoal::tick` and
//! `Squid::postAiStep` swim them (research/captures/bds-gameplay-findings.md): the goal picks a way every
//! 201 ticks, at once on starting and whenever the squid is above its band and the way rises, and
//! each tick the squid's speed closes 0.22 of the gap to that way after water's 0.8 drag. It
//! turns a tenth of the way to where it is going each tick.
// bds-swimmers.txt's spurts were its pool's flow: in still water (bds-u1squid.txt) BDS's squid and
// glow squid swam a steady 0.017 a tick at most, as sapling's.
//! Hit by a player it inks (`SquidInkCloud` after its `HurtAnimation`) and, in water within 10 of
//! the hitter, takes the way of `SquidFleeGoal` (Linux BDS 0x5933130): 3 a second, less a fifth of
//! each block past 5, straight away from the hitter's eyes (research/captures/bds-f14esquid.txt).
//! It flees while in water and within 10 of the hitter, the idle goal picking at once after; above
//! its band a rising flight loses its climb and keeps its way across. Out of water: ashore.rs; pressed
//! against a block, short of fleeing: pressed.rs.

mod ashore;
mod pressed;

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_world::World as Level;

use crate::blocks::Dice;
use crate::body::{Body, Liquid};
use crate::components::{Layer, Position};
use crate::sync::Told;
use ashore::Ashore;

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
/// A flight's length in blocks a second, from hitters this near, falling off by a fifth a block past `FLEE_FULL`.
const FLEE: f32 = 3.0;
const FLEE_FULL: f32 = 5.0;
const FLEES_WITHIN: f32 = 10.0;

#[derive(Component)]
pub(super) struct Squid {
    way: [f32; 3],
    idle: u32,
    /// Its yaw and pitch, in degrees, unwrapped as BDS keeps them.
    facing: [f32; 2],
    /// The player it flees, and the goal it runs out of water.
    fleeing: Option<Entity>,
    ashore: Option<Ashore>,
    pressed: bool,
}

pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if KINDS.contains(&kind) {
        world.entity_mut(entity).insert(Squid { way: [0.0; 3], idle: KEEPS, facing: [0.0; 2], fleeing: None, ashore: None, pressed: false });
    }
}

/// Before its move: in water a squid swims on its way, now and then another; out of it it turns.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity) -> bool {
    let (Some(body), Some(squid)) = (world.get::<Body>(entity).copied(), world.get::<Squid>(entity)) else { return false };
    let (mut way, idle, facing, fleeing, was_ashore, was_pressed) = (squid.way, squid.idle, squid.facing, squid.fleeing, squid.ashore, squid.pressed);
    if body.liquid() != Some(Liquid::Water) {
        let Some(goal) = world.get::<Layer>(layer).map(|layer| ashore::goal(&layer.world, &body)) else { return true };
        let pitch = ashore::aim(way)[1];
        if was_ashore != Some(goal) {
            way = ashore::way(&mut world.get_resource_or_insert_with(Dice::default), goal);
        }
        let target = [ashore::aim(way)[0], pitch];
        let facing = std::array::from_fn(|axis| facing[axis] + (target[axis] - facing[axis]) * TURNS);
        if let Some(mut squid) = world.get_mut::<Squid>(entity) {
            (squid.way, squid.facing, squid.fleeing, squid.ashore, squid.idle, squid.pressed) = (way, facing, None, Some(goal), KEEPS, false);
        }
        face(world, entity, facing);
        return true;
    }
    let still = fleeing.filter(|hitter| flees_from(world, *hitter, &body));
    let (ended, fleeing) = (fleeing.is_some() && still.is_none(), still);
    let band = world.get::<Layer>(layer).map(|layer| band(&layer.world, &body));
    let pressed = fleeing.is_none() && body.collided();
    if pressed && !was_pressed {
        way = pressed::way(world, layer, &body);
    }
    let above = band.is_some_and(|[_, high]| body.at[1] > high && way[1] > 0.0);
    let picks = fleeing.is_none() && !pressed && (idle >= KEEPS || above || ended || was_pressed);
    if fleeing.is_some() && above {
        way[1] = 0.0;
    }
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
        (squid.way, squid.idle, squid.facing, squid.fleeing, squid.ashore, squid.pressed) = (way, if picks || fleeing.is_some() { 0 } else { idle + 1 }, facing, fleeing, None, pressed);
    }
    if let Some(mut moving) = world.get_mut::<Body>(entity) {
        moving.fly(std::array::from_fn(|axis| speed[axis] - body.motion[axis] * WATER_DRAG));
    }
    face(world, entity, facing);
    true
}

fn face(world: &mut World, entity: Entity, facing: [f32; 2]) {
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.face(facing[0], facing[0]);
        told.pitch(facing[1]);
    }
}

/// Whether a squid at `body` still flees `hitter`: it is there and within 10 of its eyes.
fn flees_from(world: &World, hitter: Entity, body: &Body) -> bool {
    let Some(at) = world.get::<Position>(hitter) else { return false };
    let eyes = [at.feet[0], at.feet[1] + sapling_data::EYE_HEIGHT, at.feet[2]];
    (0..3).map(|axis| (body.at[axis] - eyes[axis]).powi(2)).sum::<f32>() < FLEES_WITHIN * FLEES_WITHIN
}

/// A squid hit by `hitter`, whose eyes are at `from`: its ink goes out after the hit's packet at the
/// head of `packets`, and in water it flees.
pub(super) fn struck(world: &mut World, entity: Entity, hitter: Entity, from: [f32; 3], id: u64, packets: &mut Vec<Bytes>) {
    let Some(body) = world.get::<Body>(entity).copied() else { return };
    if world.get::<Squid>(entity).is_none() {
        return;
    }
    let ink = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::SquidInkCloud, data: 0, fire_at_position: None };
    packets.insert(packets.len().min(1), sapling_net::encode(&ink));
    let away: [f32; 3] = std::array::from_fn(|axis| body.at[axis] - from[axis]);
    let far = away.iter().map(|c| c * c).sum::<f32>().sqrt();
    if body.liquid() != Some(Liquid::Water) || far >= FLEES_WITHIN || far <= 0.0 {
        return;
    }
    let length = if far > FLEE_FULL { FLEE - (far - FLEE_FULL) / FLEE_FULL } else { FLEE };
    if let Some(mut squid) = world.get_mut::<Squid>(entity) {
        (squid.way, squid.idle, squid.fleeing) = (away.map(|c| c / far * length.max(0.0) / 20.0), 0, Some(hitter));
    }
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
