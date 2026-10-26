//! How a mob walks, as BDS's were measured to (docs/research/mob-attacks.md, "Walking" and "A
//! block in the way"): it turns, its legs come up to speed, and it jumps a block in its way.
//! Where it walks to: hunting.rs, strolling.rs.

use bevy_ecs::prelude::*;

use acacia_physics::constants::DEFAULT_JUMP_HEIGHT;

use crate::body::Body;
use crate::components::Layer;
use crate::sync::Told;

/// Its legs come up to speed by this much of it a tick; the push of a tick is 0.98 of its speed squared.
const PACE: f32 = 0.1;
const STRIDE: f32 = 0.98;
/// Degrees its body turns in a tick.
const TURN: f32 = 60.0;
/// In the air its legs push 0.02 of what they do on the ground at speed 1.
const AIRBORNE: f32 = 0.02;
/// It jumps at a block whose face is this near the middle of its box, if the block is over half
/// a block high with two free above.
const STEP_NEAR: f32 = 0.5;
const STEP: f32 = 0.5;

#[derive(Component, Default)]
pub(super) struct Legs {
    pub yaw: f32,
    pace: f32,
}

/// Whether a step is before a mob going `facing`: see [`STEP_NEAR`].
fn at_step(world: &World, layer: Entity, body: &Body, facing: f32) -> bool {
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    let [x, y, z] = [body.at[0] - facing.sin() * STEP_NEAR, body.at[1], body.at[2] + facing.cos() * STEP_NEAR].map(|c| c.floor() as i32);
    let top = |up: i32| level.registry().get(level.block(x, y + up, z)).and_then(|state| state.boxes.iter().map(|b| b.max[1]).reduce(f32::max));
    level.in_height(y + 2) && top(0).is_some_and(|top| top > STEP) && top(1).is_none() && top(2).is_none()
}

/// Before the mob's move: it turns towards `to` and its legs push it there at `speed`, unless it
/// is within `near` of it.
pub(super) fn walk(world: &mut World, entity: Entity, layer: Entity, body: &Body, to: [f32; 2], speed: f32, near: f32) {
    let speed = speed * super::afflictions::pace(world, entity);
    let Some(mut legs) = world.get_mut::<Legs>(entity) else { return };
    let (dx, dz) = (to[0] - body.at[0], to[1] - body.at[2]);
    let turn = ((-dx).atan2(dz).to_degrees() - legs.yaw + 180.0).rem_euclid(360.0) - 180.0;
    legs.yaw += turn.clamp(-TURN, TURN);
    legs.pace = (legs.pace + PACE).min(1.0);
    let stride = if body.on_ground { STRIDE * (speed * legs.pace).powi(2) } else { AIRBORNE * STRIDE * speed * legs.pace };
    let (yaw, push) = (legs.yaw, if dx.hypot(dz) < near { 0.0 } else { stride });
    let facing = yaw.to_radians();
    let leap = body.on_ground && push > 0.0 && at_step(world, layer, body, facing);
    if let Some(mut body) = world.get_mut::<Body>(entity) {
        body.walk([-facing.sin() * push, facing.cos() * push], leap.then_some(DEFAULT_JUMP_HEIGHT));
    }
    if let Some(mut told) = world.get_mut::<Told>(entity) {
        told.face(yaw, yaw);
    }
}

/// The mob stands: its legs start from nothing when it next walks.
pub(super) fn rest(world: &mut World, entity: Entity) {
    if let Some(mut legs) = world.get_mut::<Legs>(entity) {
        legs.pace = 0.0;
    }
}
