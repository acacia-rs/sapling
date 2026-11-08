//! A squid out of water, as Linux BDS's goals pick its way there (it is not moved by it, only turned):
//! over a block that is not solid `SquidDiveGoal` (start 0x5932d00) aims it down, `(x, -1, z)` with x
//! and z in -0.5..0.5, at 0.5 a second; on the ground `SquidOutOfWaterGoal` (start 0x5933ab0) takes a
//! random way, 0.2 across and -0.1..0.1 up (0x7211330). Each is picked once, when its goal starts.
//! It turns a tenth of the way to its way's yaw and pitch each tick, the pitch from the way it had
//! when the tick began (research/captures/bds-f14esquid.txt, the squid on the tray).

use std::f32::consts::TAU;

use sapling_world::World as Level;

use crate::blocks::Dice;
use crate::body::Body;

const DIVE_SPEED: f32 = 0.5 / 20.0;
const ACROSS: f32 = 0.2;
const UP: [f32; 2] = [-0.1, 0.2];

/// The goal a squid out of water runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Ashore {
    Diving,
    Stranded,
}

/// Which goal runs for a squid out of water at `body`.
pub(super) fn goal(level: &Level, body: &Body) -> Ashore {
    let [x, y, z] = body.at.map(|c| c.floor() as i32);
    let below = level.in_height(y - 1) && level.registry().get(level.block(x, y - 1, z)).is_some_and(|state| state.is_solid());
    if below { Ashore::Stranded } else { Ashore::Diving }
}

/// The way `goal` takes on starting.
pub(super) fn way(dice: &mut Dice, goal: Ashore) -> [f32; 3] {
    match goal {
        Ashore::Diving => {
            let (x, z) = (dice.fraction() - 0.5, dice.fraction() - 0.5);
            let long = (x * x + z * z + 1.0).sqrt();
            [x / long * DIVE_SPEED, -1.0 / long * DIVE_SPEED, z / long * DIVE_SPEED]
        }
        Ashore::Stranded => {
            let angle = dice.fraction() * TAU;
            let up = dice.fraction() * UP[1] + UP[0];
            [angle.cos() * ACROSS, up, angle.sin() * ACROSS]
        }
    }
}

/// The yaw and pitch, in degrees, that a way points along.
pub(super) fn aim(way: [f32; 3]) -> [f32; 2] {
    let across = way[0].hypot(way[2]);
    [(-way[0]).atan2(way[2]).to_degrees(), -across.atan2(way[1]).to_degrees()]
}
