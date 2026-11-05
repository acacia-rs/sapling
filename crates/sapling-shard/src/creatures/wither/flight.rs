//! A wither's flight. Up or down (research/captures/bds-m3wither.txt, 30034 on): it keeps 0.6 of the
//! speed it last moved by after gravity 0.08 and drag 0.98, and while under its prey's position plus
//! 5 (its prey's alone once armoured; a player's position is its eyes) it is pushed up 0.3 less 0.6 of
//! that, so it rises 0.3677 a tick and bobs 0.4 about 5 over the prey's eyes. Across it closes in at
//! 0.3 till it is 3 off.

use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::Stage;

const GRAVITY: f32 = 0.08;
const FALL_DRAG: f32 = 0.98;
const KEPT: f32 = 0.6;
const SPEED: f32 = 0.3;
const KEEPS_OFF: f32 = 3.0;
const KEEPS_OVER: f32 = 5.0;

/// Where it measures its height from: a player's eyes, a mob's feet.
fn height(world: &World, prey: Entity, feet: [f32; 3]) -> f32 {
    feet[1] + if world.get::<Stage>(prey).is_some() { sapling_data::EYE_HEIGHT } else { 0.0 }
}

/// Its speed up or down for this tick's move.
pub(super) fn rise(world: &World, body: &Body, prey: Option<(Entity, [f32; 3])>, armoured: bool) -> f32 {
    let mut y = (body.motion[1] - GRAVITY) * FALL_DRAG * KEPT;
    if let Some((prey, feet)) = prey {
        let over = height(world, prey, feet);
        if body.at[1] < over || !armoured && body.at[1] < over + KEEPS_OVER {
            y = y.max(0.0);
            y += SPEED - y * KEPT;
        }
    }
    y
}

/// Its speed across, closing in on its prey till it is near.
pub(super) fn close_in(body: &Body, prey: Option<[f32; 3]>) -> [f32; 2] {
    let [mut x, mut z] = [body.motion[0], body.motion[2]];
    if let Some(prey) = prey {
        let across = [prey[0] - body.at[0], prey[2] - body.at[2]];
        let long = across[0].hypot(across[1]);
        if long > KEEPS_OFF {
            x += across[0] / long * SPEED - x * KEPT;
            z += across[1] / long * SPEED - z * KEPT;
        }
    }
    [x, z]
}
