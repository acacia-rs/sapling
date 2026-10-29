//! The cloud a lingering potion leaves where it breaks, as BDS's (research/captures/bds-clouds.txt):
//! 600 ticks, radius 3 growing by half a block over its life, 0.5 less for each one given, the
//! same one again 40 ticks on, particle 35. Run as every cloud is (clouds.rs).

use bevy_ecs::prelude::*;

use crate::clouds::{self, Spec};

const TICKS: u64 = 600;
const RADIUS: f32 = 3.0;
const GROWTH: f32 = 0.5;
const PARTICLE: i32 = 35;

/// A cloud of the potion of data value `potion` in the middle of the block at `at` (bds-clouds.txt).
pub(crate) fn spawn(world: &mut World, layer: Entity, at: [f32; 3], potion: i16) -> Option<Entity> {
    let at = at.map(|c| c.floor() + 0.5);
    let colour = sapling_data::shots::potion_colour(potion);
    let spec = Spec { potion, colour, particle: PARTICLE, radius: RADIUS, per_tick: GROWTH / TICKS as f32, on_use: -0.5, ticks: TICKS, again: 40 };
    clouds::spawn(world, layer, at, 0.0, spec, clouds::KIND)
}
