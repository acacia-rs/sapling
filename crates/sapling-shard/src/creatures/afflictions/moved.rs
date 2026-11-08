//! Effects that move a mob, as BDS's zombie moved (research/captures/bds-a3fx.txt): levitation II
//! lifted it towards 0.1 a tick, a fifth of the way each tick and then the fall's drag (0.0196,
//! 0.03497, 0.04701 ... 0.0907), from the tick after; slow falling made its gravity 0.01 once falling
//! (bds-a3fx2.txt: each tick's fall (v + 0.01) x 0.98 to 1e-5); jump boost III made its jump 0.72 up
//! (told 0.6272).

use bevy_ecs::prelude::*;
use sapling_api::Effect;

use crate::body::Body;

/// What a level of levitation lifts a mob towards, and of jump boost adds to its jump.
const LIFTS: f32 = 0.05;
const BOOSTS: f32 = 0.1;

/// Before the mob's move: its body takes its levitation and its slow falling, or neither.
pub(in crate::creatures) fn float(world: &mut World, mob: Entity) {
    let speed = super::level(world, mob, Effect::Levitation).map_or(0.0, |level| LIFTS * (f32::from(level) + 1.0));
    let slowly = super::level(world, mob, Effect::SlowFalling).is_some();
    if let Some(mut body) = world.get_mut::<Body>(mob) {
        body.levitate(speed);
        body.fall_slowly(slowly);
    }
}

/// How fast the mob's jump `rise` goes up under its jump boost.
pub(in crate::creatures) fn jump(world: &World, mob: Entity, rise: f32) -> f32 {
    rise + super::level(world, mob, Effect::JumpBoost).map_or(0.0, |level| BOOSTS * (f32::from(level) + 1.0))
}
