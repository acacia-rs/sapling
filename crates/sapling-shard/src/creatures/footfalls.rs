//! What a mob's feet do to the world: its steps vibrate, and its landing treads farmland.
//!
//! Steps, as BDS's step system (Linux 0x7539970, under gdb in research/captures/bds-f14v2steps-gdb.txt): a
//! mob on the ground counts 3 for each block it walks across and vibrates (`entity_move`, 22) from its feet
//! each time the count passes the next whole one, so every third of a block; BDS's walking zombie at 0.114 a
//! tick vibrated every third tick. The count up a ladder or scaffolding is not ported.
//!
//! Treading, as FarmBlock::transformOnFall (Linux 0xc563f30, called by the fall-distance update 0xb2df080 on
//! the block a mob lands on; bds-f14v2tread*-gdb.txt, docs/research/growth.md): a landing treads farmland as a
//! player's does (`blocks::trample`, the drop less a half as its chance), when the mob's box holds at least
//! 0.512 (width² × height, so no calf or chicken) and `mobGriefing` is on.

use bevy_ecs::prelude::*;

use crate::blocks::GameEvent;
use crate::body::Body;

const LEAST_BULK: f32 = 0.512;
const STRIDES: f32 = 3.0;

/// The count of its strides and the whole one at which it next vibrates.
#[derive(Component, Clone, Copy)]
pub(super) struct Strides {
    walked: f32,
    next: f32,
}

/// A mob's move from `start` to `now`: a step's vibration when its strides pass the next whole one.
pub(super) fn step(world: &mut World, entity: Entity, layer: Entity, start: &Body, now: &Body) {
    let across = (now.at[0] - start.at[0]).hypot(now.at[2] - start.at[2]) * STRIDES;
    if !now.on_ground || now.liquid().is_some() || across <= 0.0 {
        return;
    }
    let mut strides = world.get::<Strides>(entity).copied().unwrap_or(Strides { walked: 0.0, next: 1.0 });
    strides.walked += across;
    let stepped = strides.walked >= strides.next;
    if stepped {
        strides.next = strides.walked.floor() + 1.0;
    }
    world.entity_mut(entity).insert(strides);
    if stepped {
        crate::blocks::vibrate_by(world, layer, now.at, GameEvent::Step, entity);
    }
}

/// A mob's move ended on the ground after dropping `dropped`.
pub(super) fn tread(world: &mut World, layer: Entity, body: &Body, dropped: f32) {
    let [width, height] = body.size;
    if dropped > 0.0 && width * width * height >= LEAST_BULK && super::stomping::griefs(world, layer) {
        crate::blocks::trample(world, layer, body.at, dropped);
    }
}
