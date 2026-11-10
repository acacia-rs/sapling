//! A mob climbing a ladder or vine (acacia-physics `climbable`; not scaffolding): one whose move in a
//! climbable cell is stopped across by a block rises 0.2 in its next tick, in place of falling.
//! BDS's zombies crowded into a walled vine cell went up it so, 0.2 a tick from the tick after the
//! first push to its top, and were heard on the way (research/captures/bds-ai2push.txt); pathing
//! mobs never climb one (bds-af2climb.txt).

use acacia_physics::WorldView;

use super::Body;
use crate::terrain::physics_view;
use sapling_world::World as Level;

pub(super) const CLIMB: f32 = 0.2;

/// Before a mob's move: a climb its last move started sets its rise.
pub(super) fn rise(body: &mut Body) {
    if std::mem::take(&mut body.climbing) {
        body.motion[1] = CLIMB;
    }
}

/// After a mob's move by `wanted`: whether it pressed across on a block from a climbable cell.
pub(super) fn pressed(level: &Level, body: &mut Body, wanted: [f32; 3]) {
    let stopped = |axis: usize| wanted[axis] != 0.0 && body.motion[axis] == 0.0;
    body.climbing = (stopped(0) || stopped(2)) && is_climbing(level, body);
}

/// Whether the cell a body's feet are in is one it climbs.
pub(crate) fn is_climbing(level: &Level, body: &Body) -> bool {
    physics_view(level, |view| view.block(body.at.map(|c| c.floor() as i32)).climbable)
}
