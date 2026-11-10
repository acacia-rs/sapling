//! An unarmoured wither's swoop to a spot near its prey, as BDS's `WitherRandomAttackPosGoal` (Linux vtable
//! 0xeecb048; canUse 0x54fca50, start 0x54fcbf0, stop 0x54fcda0) and `WitherBoss`'s volley count
//! (0xbbbd900): [`REST`] ticks after a volley's blue skull it takes a way at random (`RandomPos::getPos`
//! within 10 across, of which only the way counts) and flies to the spot 10 across from its prey along
//! that way, at the middle of the block its feet are in. Under gdb on ab1wither2 (bds-ah1wither-gdb1.txt):
//! goal speed 1, `xz_dist` 10, `y_dist` 7, rest 40, fire rate 20, the spot 10.0 off the cow on each start.
//! Its push is its air speed times 15 along its path, here fitted (bds-ae1wither2.txt, second swoop, to
//! 0.2 of a block over 35 ticks): [`THRUST`] a tick after drag, coming up to strength by a tenth a tick.
//! The goal ends a tick after the path is done, fitted as two ticks after it comes within [`REACHED`]
//! across or passes the spot (BDS's stopped 2.4 to 3.5 past the spot). Its middle head shoots again on its fire rate's beat
//! once [`AFTER`] ticks have passed (the stop's +0x4fc).

use bevy_ecs::prelude::*;

use super::Wither;
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{Layer, Tick};
use sapling_world::ChunkPos;

pub(super) const REST: u64 = 40;
pub(super) const AFTER: u64 = 20;
const OFF_PREY: f32 = 10.0;
const WAYS: i32 = 10;
const THRUST: f32 = 0.1764;
const PACE: f32 = 0.1;
const REACHED: f32 = 1.0;
const LAST_TICKS: u8 = 2;
/// A swoop that has not come to its spot by then ends (BDS's ran 1.5 to 2 s; its path would fail).
const GIVES_UP: u32 = 100;

#[derive(Default)]
pub(super) struct Swoop {
    /// The tick it is due, and the tick and pace of the volley's blue skull that set it.
    due: Option<(u64, u64, u64)>,
    way: Option<Way>,
}

struct Way {
    spot: [f32; 3],
    /// The way across to the spot at the start: once past the spot along it, the swoop ends.
    from: [f32; 2],
    ticks: u32,
    pace: f32,
    /// Ticks left once it has come near the spot.
    last: Option<u8>,
    blue: u64,
    every: u64,
}

impl Swoop {
    /// A volley's blue skull went at `now`, its middle head shooting every `every` ticks.
    pub(super) fn after_volley(&mut self, now: u64, every: u64) {
        self.due = Some((now + REST, now, every));
    }
}

/// The spot `OFF_PREY` across from `prey` along a way drawn as `RandomPos::getPos` draws its offset, at the middle of `body`'s block.
fn spot(dice: &mut Dice, body: &Body, prey: [f32; 3]) -> [f32; 3] {
    let (x, z) = loop {
        let (x, z) = (dice.roll(2 * WAYS as u32 + 1) as i32 - WAYS, dice.roll(2 * WAYS as u32 + 1) as i32 - WAYS);
        if x != 0 || z != 0 {
            break (x as f32, z as f32);
        }
    };
    let long = x.hypot(z);
    [prey[0] + x / long * OFF_PREY, body.at[1].floor() + 0.5, prey[2] + z / long * OFF_PREY]
}

/// Before its move: a tick of its swoop, or the start of one that is due; false when it is not swooping.
/// At the swoop's end the middle head's next shot is set on its beat.
pub(super) fn steer(world: &mut World, wither: Entity, layer: Entity, body: &Body, prey: Option<[f32; 3]>) -> bool {
    let now = world.resource::<Tick>().0;
    let Some(mut swoop) = world.get_mut::<Wither>(wither).map(|mut kept| std::mem::take(&mut kept.swoop)) else { return false };
    let mut ended = None;
    if swoop.way.is_none()
        && let (Some((due, blue, every)), Some(prey)) = (swoop.due, prey)
        && now >= due
    {
        let spot = spot(&mut world.get_resource_or_insert_with(Dice::default), body, prey);
        let from = [spot[0] - body.at[0], spot[2] - body.at[2]];
        // BDS's path to a spot in no loaded chunk fails, and the goal stops at once.
        let open = world.get::<Layer>(layer).is_some_and(|layer| layer.world.is_loaded(ChunkPos::of_block(spot[0].floor() as i32, spot[2].floor() as i32)));
        swoop.due = None;
        if open {
            swoop.way = Some(Way { spot, from, ticks: 0, pace: 0.0, last: None, blue, every });
        } else {
            ended = Some((blue, every));
        }
    }
    if let Some(way) = swoop.way.as_mut() {
        let across = [way.spot[0] - body.at[0], way.spot[2] - body.at[2]];
        let long = across[0].hypot(across[1]).max(1e-4);
        way.ticks += 1;
        let passed = across[0] * way.from[0] + across[1] * way.from[1] <= 0.0;
        if way.last.is_none() && (long < REACHED || passed || way.ticks >= GIVES_UP) {
            way.last = Some(LAST_TICKS);
        }
        if let Some(last) = way.last.as_mut() {
            *last -= 1;
        }
        way.pace = (way.pace + PACE).min(1.0);
        let push = THRUST * way.pace;
        if let Some(mut moving) = world.get_mut::<Body>(wither) {
            moving.motion[1] = way.spot[1] - body.at[1];
            moving.fly([across[0] / long * push, 0.0, across[1] / long * push]);
        }
        if way.last == Some(0) {
            ended = Some((way.blue, way.every));
        }
    }
    let swooping = swoop.way.is_some();
    if ended.is_some() {
        swoop.way = None;
    }
    if let Some(mut kept) = world.get_mut::<Wither>(wither) {
        kept.swoop = swoop;
        if let Some((blue, every)) = ended {
            let after = now + 1 + AFTER;
            kept.heads.resume(blue + (after.saturating_sub(blue)).div_ceil(every) * every);
        }
    }
    swooping
}
