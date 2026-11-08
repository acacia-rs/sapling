//! A mob landing on farmland treads it to dirt as a player does (`blocks::trample`, the drop less a half as
//! its chance), when its box holds at least 0.512 (width² × height, so no baby or chicken) and `mobGriefing`
//! is on: FarmBlock::transformOnFall (Linux 0xc563f30), called by the fall-distance update 0xb2df080 on the
//! block the mob lands on, read under gdb in research/captures/bds-f14v2tread*.txt (docs/research/growth.md).

use bevy_ecs::prelude::*;

use crate::body::Body;

const LEAST_BULK: f32 = 0.512;

/// A mob's move ended on the ground after dropping `dropped`.
pub(super) fn tread(world: &mut World, layer: Entity, body: &Body, dropped: f32) {
    let [width, height] = body.size;
    if dropped > 0.0 && width * width * height >= LEAST_BULK && super::stomping::griefs(world, layer) {
        crate::blocks::trample(world, layer, body.at, dropped);
    }
}
