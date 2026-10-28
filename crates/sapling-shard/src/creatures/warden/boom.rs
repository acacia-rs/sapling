//! A warden's sonic boom at prey it cannot walk to, as BDS's boomed at the actor caged on a
//! pillar (research/captures/bds-warden-findings.md, "Sonic boom"): it stands charging, heard
//! `sonic_charge` a tick in; 35 ticks in its prey is hurt through armour and thrown, and it is
//! heard `sonic_boom`; 61 ticks in it is done, and booms again no sooner than 41 ticks on.

use bevy_ecs::prelude::*;

use super::HEAD;
use crate::body::Body;
use crate::components::{Layer, Position, Tick};
use crate::creatures::{hunting, paths};
use crate::{entities, late_sounds, vitals};

/// Prey this far across and up at most is boomed at.
const ACROSS: f32 = 15.0;
const UP: f32 = 20.0;
const CHARGED: u64 = 35;
pub(super) const LASTS: u64 = 61;
pub(super) const AGAIN: u64 = 41;
/// Its pack's `attack_damage`, on normal.
const HARM: f32 = 10.0;

/// Whether the warden at `body` may boom at prey at `feet`: in range, with no way to it on foot.
pub(super) fn can_boom(world: &World, layer: Entity, body: &Body, feet: [f32; 3]) -> bool {
    let near = (feet[0] - body.at[0]).hypot(feet[2] - body.at[2]) <= ACROSS && (feet[1] - body.at[1]).abs() <= UP;
    let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return false };
    let cell = |at: [f32; 3]| at.map(|c| c.floor() as i32);
    near && !paths::is_clear(level, body.at, feet) && paths::route(level, cell(body.at), cell(feet)).is_none()
}

/// A tick of the boom at `prey` the warden began in `since`.
pub(super) fn tick(world: &mut World, entity: Entity, layer: Entity, id: u64, kind: &str, body: &Body, since: u64, prey: Entity) {
    let now = world.resource::<Tick>().0;
    let cell = body.at.map(|c| c.floor() as i32);
    match now - since {
        1 => entities::tell_viewers(world, entity, layer, cell, vec![hunting::sound("sonic_charge", id, kind, body, HEAD)]),
        CHARGED => {
            let Some(feet) = world.get::<Position>(prey).map(|position| position.feet) else { return };
            if let Some(harm) = hunting::on_difficulty(world, HARM).filter(|_| (feet[0] - body.at[0]).hypot(feet[2] - body.at[2]) <= ACROSS) {
                vitals::boomed(world, prey, harm, kind, body.at);
            }
            late_sounds::play_later(world, 1, layer, cell, hunting::sound("sonic_boom", id, kind, body, HEAD));
        }
        _ => {}
    }
}
