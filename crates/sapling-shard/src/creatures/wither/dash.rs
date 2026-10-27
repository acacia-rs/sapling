//! An armoured wither's dash (research/captures/bds-witherdash.txt, both runs): 197 ticks after its
//! half-health blast, and 320 after that (one interval seen), it flies straight across at its prey
//! where it stood, 2 a tick for 9 ticks (told 1.82, after the tick's drag), through it and on,
//! then stops dead. Who it meets on the way is hit once (15, the doc's reading).
// TODO: the 20 ticks BDS's wither may spend aiming, how its height changes on the way and what the
// hit costs were not seen (the actor was warded); the first run's dash went through an iron golem.

use bevy_ecs::prelude::*;

use crate::body::Body;
use crate::components::{InLayer, Position, Tick, in_world};
use crate::vitals;

const FIRST: u64 = 197;
const EVERY: u64 = 320;
const SPEED: f32 = 2.0;
const LASTS: u64 = 9;
const HARM: f32 = 15.0;
const THROW: f32 = 0.4;
/// A player this near its middle across, and within its height, is met.
const MEETS: f32 = 1.5;

#[derive(Default)]
pub(super) struct Dash {
    next: Option<u64>,
    /// The tick it ends, its way across, and who it has met.
    under_way: Option<(u64, [f32; 2], Vec<Entity>)>,
}

impl Dash {
    /// It was armoured just now.
    pub(super) fn arm(&mut self, now: u64) {
        self.next = Some(now + FIRST);
    }
}

/// Before its move: a tick of its dash, or the start of one that is due; false when it is not dashing.
pub(super) fn steer(world: &mut World, wither: Entity, layer: Entity, dash: &mut Dash, body: &Body, prey: Option<[f32; 3]>) -> bool {
    let now = world.resource::<Tick>().0;
    if dash.under_way.is_none()
        && let (Some(due), Some(prey)) = (dash.next, prey)
        && now >= due
    {
        let across = [prey[0] - body.at[0], prey[2] - body.at[2]];
        let long = across[0].hypot(across[1]).max(1e-4);
        dash.under_way = Some((now + LASTS, [across[0] / long, across[1] / long], Vec::new()));
        dash.next = Some(now + EVERY);
    }
    let Some((ends, way, met)) = dash.under_way.as_mut() else { return false };
    let going = now < *ends;
    if let Some(mut moving) = world.get_mut::<Body>(wither) {
        // Its flight's drag comes off before the move here (body/physics.rs `FLYER`).
        let speed = SPEED / crate::body::Physics::FLYER.air_drag;
        moving.motion = if going { [way[0] * speed, 0.0, way[1] * speed] } else { [0.0; 3] };
    }
    if going {
        for player in in_world(world) {
            let feet = world.get::<Position>(player).map(|at| at.feet).filter(|_| world.get::<InLayer>(player).is_some_and(|on| on.0 == layer));
            let meets = feet.is_some_and(|feet| (feet[0] - body.at[0]).hypot(feet[2] - body.at[2]) < MEETS && feet[1] < body.at[1] + body.size[1] && body.at[1] < feet[1] + 1.8);
            if meets && !met.contains(&player) {
                met.push(player);
                vitals::bitten(world, player, HARM, super::KIND, [way[0] * THROW, THROW, way[1] * THROW]);
            }
        }
    } else {
        dash.under_way = None;
    }
    true
}
