//! How a dolphin swims, as BDS's in `dolphin` (research/captures/bds-dolphin-findings.md): its
//! body turns 10 degrees a tick at most (7 of 256 told) and it is pushed 0.022 along its heading,
//! water keeping 0.8 of its speed (`Physics::SWIMMER`): 0.11 a tick at most. About, the push is
//! across, with as much again up or down towards its spot; at its prey, along the way it faces,
//! pitched at it. With its feet in the top water cell it bobs down 0.086 a tick (32142, 38937).

use bevy_ecs::prelude::*;

use super::{WATER, harm};
use crate::body::Body;
use crate::components::Layer;
use crate::creatures::legs::Legs;
use crate::sync::Told;

pub(super) const PUSH: f32 = 0.022;
const TURN: f32 = 10.0;
const BOB: f32 = 0.086;

/// Turns the dolphin's body towards `to` by up to [`TURN`]: its yaw now, in degrees.
pub(super) fn turn(world: &mut World, entity: Entity, body: &Body, to: [f32; 3]) -> f32 {
    let (dx, dz) = (to[0] - body.at[0], to[2] - body.at[2]);
    let Some(mut legs) = world.get_mut::<Legs>(entity) else { return 0.0 };
    if dx.hypot(dz) > 1e-4 {
        let wanted = ((-dx).atan2(dz).to_degrees() - legs.yaw + 180.0).rem_euclid(360.0) - 180.0;
        legs.yaw += wanted.clamp(-TURN, TURN);
    }
    legs.yaw
}

/// Has the dolphin's next tick add `across` along `yaw` and `up`, and tells its facing.
pub(super) fn push(world: &mut World, entity: Entity, yaw: f32, across: f32, up: f32) {
    let facing = yaw.to_radians();
    if let Some(mut body) = world.get_mut::<Body>(entity) {
        body.fly([-facing.sin() * across, up, facing.cos() * across]);
    }
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.face(yaw, yaw);
        told.pitch(-up.atan2(across).to_degrees());
    }
}

/// About: across along its heading, and up or down towards `to`.
pub(super) fn roam(world: &mut World, entity: Entity, body: &Body, to: [f32; 3]) {
    let yaw = turn(world, entity, body, to);
    push(world, entity, yaw, PUSH, (to[1] - body.at[1]).clamp(-1.0, 1.0) * PUSH);
}

/// At its prey: one push along its heading, pitched at `to`.
pub(super) fn chase(world: &mut World, entity: Entity, body: &Body, to: [f32; 3]) {
    let yaw = turn(world, entity, body, to);
    let pitch = (to[1] - body.at[1]).atan2((to[0] - body.at[0]).hypot(to[2] - body.at[2]));
    push(world, entity, yaw, PUSH * pitch.cos(), PUSH * pitch.sin());
}

/// Whether `cell` holds the top of the water: water with air over it.
pub(super) fn is_top(world: &World, layer: Entity, [x, y, z]: [i32; 3]) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    harm::holds(level, [x, y, z], WATER) && level.registry().get(level.block(x, y + 1, z)).is_some_and(|state| state.is_air())
}

/// With its feet in the top water cell, it is pushed down [`BOB`] on top of its swim.
pub(super) fn bob(world: &mut World, entity: Entity, layer: Entity, body: &Body) {
    if is_top(world, layer, body.at.map(|c| c.floor() as i32))
        && let Some(mut body) = world.get_mut::<Body>(entity)
    {
        body.swim_up(-BOB);
    }
}
