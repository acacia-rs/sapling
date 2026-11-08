//! A bee with pollen grows crops it flies over (`grows_crop`: 10 charges, 0.03 a tick), as BDS's
//! bees going home over wheat grew the wheat a block under them (research/captures/bds-g14bee.txt).
// The Java game's bee also looks two blocks under; one is all BDS's showed.

use bevy_ecs::prelude::*;

use crate::blocks::Dice;
use crate::body::Body;

const CHARGES: u8 = 10;
const CHANCE: f32 = 0.03;

/// A tick of a bee with pollen that has grown `grown` crops since it took it: it may grow the
/// crop under it. The crops grown now.
pub(super) fn grow(world: &mut World, layer: Entity, body: &Body, grown: u8) -> u8 {
    if grown >= CHARGES || world.get_resource_or_insert_with(Dice::default).fraction() >= CHANCE {
        return grown;
    }
    let [x, y, z] = body.at.map(|c| c.floor() as i32);
    grown + u8::from(crate::blocks::pollinate(world, layer, [x, y - 1, z]))
}
