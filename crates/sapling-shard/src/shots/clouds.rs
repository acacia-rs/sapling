//! The cloud a lingering potion leaves where it breaks (`spawn_aoe_cloud` in the pack: 30 seconds,
//! BDS's ticks of it 600, radius 3, 0.5 less for each one given, the same one again 40 ticks on),
//! run as every cloud is (clouds.rs).
// TODO: shrinking over its life (radius / life a tick) is the Java game's; where BDS sets RadiusPerTick was not found.

use bevy_ecs::prelude::*;

use crate::clouds::{self, Spec};

const TICKS: u64 = 600;
const RADIUS: f32 = 3.0;

/// A cloud of the potion of data value `potion` at `at`.
pub(crate) fn spawn(world: &mut World, layer: Entity, at: [f32; 3], potion: i16) -> Option<Entity> {
    let colour = sapling_data::shots::potion_colour(potion);
    let spec = Spec { potion, colour, radius: RADIUS, per_tick: -RADIUS / TICKS as f32, on_use: -0.5, ticks: TICKS, again: 40 };
    clouds::spawn(world, layer, at, 0.0, spec, clouds::KIND)
}
