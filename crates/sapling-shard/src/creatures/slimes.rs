//! A slime gets about by hopping, as BDS's (`minecraft:movement.jump`, its pack's `jump_delay` of
//! 0.5 to 1.5 s; research/captures/bds-kinds2.txt): on the ground it waits out its delay, then
//! jumps 0.42 up and ahead the way it faces, and keeps going ahead in the air; now and then it
//! turns another way. Its pace is by its size, a small one's 0.3, a medium's 0.4, a large's 0.6.
// TODO: an aggressive slime (one with a target, delay 0.16 to 0.5 s) hops at it; magma cubes.

use bevy_ecs::prelude::*;

use crate::body::Body;

const KIND: &str = "minecraft:slime";
/// Ticks on the ground between hops, least and most.
const DELAY: [u32; 2] = [10, 30];
/// Of its pace, what a hop throws it ahead and what it adds in the air each tick after
/// (bds-kinds2.txt: a medium slime's 0.0856 a tick at the jump, about 0.082 after).
const HOP: f32 = 0.214;
const AIR: f32 = 0.0185;
/// Ticks between turns, least and most.
const TURNS: [u32; 2] = [40, 100];

/// A slime's hopping, with a random state of its own: drawing on the shard's every tick would
/// shift every other draw.
#[derive(Component, Clone, Copy)]
pub(super) struct Hops {
    wait: u32,
    yaw: f32,
    turn_in: u32,
    state: u64,
}

impl Hops {
    /// A number from `low` to `high`, both kept.
    fn roll(&mut self, [low, high]: [u32; 2]) -> u32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        low + (self.state % u64::from(high - low + 1)) as u32
    }
}

/// A new slime's hopping, if `kind` is one.
pub(super) fn arm(world: &mut World, entity: Entity, kind: &str) {
    if kind == KIND {
        world.entity_mut(entity).insert(Hops { wait: DELAY[0], yaw: 0.0, turn_in: 0, state: entity.to_bits() | 1 });
    }
}

/// Its pace by its box: a slime's width is its size's.
fn pace(body: &Body) -> f32 {
    match body.size[0] {
        width if width < 0.8 => 0.3,
        width if width < 1.5 => 0.4,
        _ => 0.6,
    }
}

/// Before its move: a slime turns, waits or hops.
pub(super) fn tick(world: &mut World, entity: Entity, start: &Body) {
    let Some(mut hops) = world.get::<Hops>(entity).copied() else { return };
    let leap = start.on_ground && hops.wait == 0;
    if hops.turn_in == 0 {
        hops.yaw = hops.roll([0, 359]) as f32;
        hops.turn_in = hops.roll(TURNS);
    }
    if start.on_ground {
        hops.wait = if leap { hops.roll(DELAY) } else { hops.wait - 1 };
    }
    hops.turn_in -= 1;
    let (sin, cos) = hops.yaw.to_radians().sin_cos();
    let ahead = if leap { HOP } else if start.on_ground { 0.0 } else { AIR } * pace(start);
    if let Some(mut kept) = world.get_mut::<Hops>(entity) {
        *kept = hops;
    }
    if let Some(mut body) = world.get_mut::<Body>(entity) {
        // BDS's comes to a stop where it lands, and moves again with its next hop.
        if start.on_ground && !leap {
            (body.motion[0], body.motion[2]) = (0.0, 0.0);
        }
        body.walk([-sin * ahead, cos * ahead], leap);
    }
}
