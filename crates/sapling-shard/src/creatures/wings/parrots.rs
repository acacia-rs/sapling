//! A parrot flies as BDS's `FlyMoveControl` (Linux 0xbdf41f0) moved its in bds-x2parrot.txt: aloft, nothing
//! pulls it down or slows its rise and fall, across it keeps 0.91 a tick (up to 0.2 a tick), and each
//! tick it is pushed 0.02 (0.05 x its `movement` 0.4) at the centre of the next cell on its way, its
//! whole speed quartered first when it is within a third of its box's sides of it (takeoffs rose
//! exactly 0.02, 0.42 and 1.00125 over 1, 6 and 11 ticks). With no flight
//! under way its gravity is back (`AFFECTED_BY_GRAVITY`): it falls as a chicken does, `(v - 0.08) x 0.98
//! x 0.6` down to 0.114 a tick, and sits where it lands until it flies again.

use acacia_proto::types::MetadataFlags1 as Flags;
use bevy_ecs::prelude::*;

use super::{REACHED, Wings};
use crate::blocks::Dice;
use crate::body::{Body, Physics};
use crate::components::{Layer, Tick};

pub(in crate::creatures) const KIND: &str = "minecraft:parrot";
/// A parrot not flying takes off to a new spot at 1 in this a tick, while its last flight ended under
/// [`RESTLESS`] ticks ago: `RandomFlyingGoal::canUse` (Linux 0x57601e0) rolls `nextInt(120)` only
/// while a counter of the mob's is under 100. bds-x2parrot.txt: all 8 takeoffs came 7 to 98 ticks
/// after a landing, and none in the 350 to 570 ticks each of the three then sat (the counter's reset
/// at a flight's end is read off that capture, not decompiled).
const TAKES_OFF: u32 = 120;
const RESTLESS: u64 = 100;
/// What its speed keeps where it nears the cell it flies at.
const NEARING: f32 = 0.25;

/// Before its move: a parrot sitting or falling may take off; one flying flies on at its spot, or
/// stops once there or after [`super::GIVES_UP`] ticks. True: nothing else steers it.
pub(super) fn steer(world: &mut World, entity: Entity, layer: Entity) -> bool {
    let (Some(body), Some(wings)) = (world.get::<Body>(entity).copied(), world.get::<Wings>(entity)) else { return true };
    let now = world.resource::<Tick>().0;
    let (spot, until, landed) = (wings.spot, wings.until, wings.landed);
    let spot = match spot {
        Some(spot) if now < until && distance(spot, body.at) >= REACHED => spot,
        Some(_) => return land(world, entity, now),
        None if now >= landed + RESTLESS || world.get_resource_or_insert_with(Dice::default).roll(TAKES_OFF) != 0 => return true,
        None => {
            let Some(spot) = super::pick(world, layer, body.at, &super::PARROT) else { return true };
            if let Some(mut wings) = world.get_mut::<Wings>(entity) {
                (wings.spot, wings.until) = (Some(spot), now + super::GIVES_UP);
            }
            spot
        }
    };
    let next = match world.get::<Wings>(entity).and_then(|wings| wings.next) {
        Some(next) if distance(next, body.at) >= passes(&body) => next,
        _ => {
            let Some(level) = world.get::<Layer>(layer).map(|layer| &layer.world) else { return true };
            let next = waypoint(level, &body, spot);
            if let Some(mut wings) = world.get_mut::<Wings>(entity) {
                wings.next = Some(next);
            }
            next
        }
    };
    super::fly_to(world, entity, &super::PARROT, &body, next, 1.0);
    true
}

fn land(world: &mut World, entity: Entity, now: u64) -> bool {
    if let Some(mut wings) = world.get_mut::<Wings>(entity) {
        (wings.spot, wings.next, wings.landed) = (None, None, now);
    }
    true
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|axis| (a[axis] - b[axis]).powi(2)).sum::<f32>().sqrt()
}

/// The centre of the next cell on the way to `spot`, or the spot in its own cell: straight up off a
/// floor first, as BDS's rose a block before it moved across.
fn waypoint(level: &sapling_world::World, body: &Body, spot: [f32; 3]) -> [f32; 3] {
    let [from, to] = [body.at, spot].map(|at| at.map(|c| c.floor() as i32));
    if from == to {
        return spot;
    }
    let below = [from[0], from[1] - 1, from[2]];
    let floored = level.in_height(below[1]) && level.registry().get(level.block(below[0], below[1], below[2])).is_some_and(|state| !state.boxes.is_empty());
    let step: [i32; 3] = std::array::from_fn(|axis| (to[axis] - from[axis]).signum());
    let next = if floored && step[1] > 0 { [from[0], from[1] + 1, from[2]] } else { std::array::from_fn(|axis| from[axis] + step[axis]) };
    next.map(|c| c as f32 + 0.5)
}

/// Quarters the parrot's speed where it is within a third of its box's sides of `target`, as BDS's
/// control does before it pushes.
pub(super) fn slow_nearing(body: &mut Body, target: [f32; 3]) {
    if distance(target, body.at) < nearing(body) {
        body.motion = body.motion.map(|c| c * NEARING);
    }
}

/// How near the cell's centre it flies at it slows: a third of its box's three sides.
fn nearing(body: &Body) -> f32 {
    let [width, height] = body.size;
    (width * 2.0 + height) / 3.0
}

/// How near a cell's centre it takes the next: a path's `0.75 - width / 2` (0.5); the takeoff's
/// 1.00125 at 11 ticks needs the first cell kept that long.
fn passes(body: &Body) -> f32 {
    0.75 - body.size[0] / 2.0
}

/// How the mob moves this tick: a parrot pushed this tick flies, else it falls; its
/// `AFFECTED_BY_GRAVITY` is shown again as that changes. Other kinds: `of_kind`.
pub(in crate::creatures) fn physics_showing_gravity(world: &mut World, entity: Entity, kind: &str, of_kind: &'static Physics) -> &'static Physics {
    let now = world.resource::<Tick>().0;
    let Some(wings) = world.get::<Wings>(entity).filter(|_| kind == KIND) else { return of_kind };
    let (flies, shown) = (wings.flew == Some(now), wings.aloft);
    if flies != shown {
        if let Some(mut wings) = world.get_mut::<Wings>(entity) {
            wings.aloft = flies;
        }
        super::super::husbandry::show_flag(world, entity, Flags::AFFECTED_BY_GRAVITY, !flies);
    }
    if flies { &Physics::PARROT } else { &Physics::PARROT_FALLING }
}
