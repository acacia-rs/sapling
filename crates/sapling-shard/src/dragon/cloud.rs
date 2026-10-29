//! The dragon's breath clouds (research/dragon-extras-decompile.md 4.3-4.5,
//! research/captures/bds-dragon-survival.txt): clouds of harming (clouds.rs), one where a fireball
//! meets something, radius 6 for 2400 ticks, and one by its head as it flames, radius 5 for 200.
//! Neither grows nor shrinks. Half an instant harm is 3 (the undead are mended), a player thrown
//! as by a magic hit from the cloud; the fireball's spares one it harmed for 20 ticks.
// BDS's pulse stops at its owner (extras 5.4), but these clouds have none (OwnerEid -1 as captured).

use bevy_ecs::prelude::*;

use crate::clouds::{self, Spec};
pub(crate) use crate::clouds::{item, with};

const HARMING: i16 = 23;
/// The particle BDS named in its breath's clouds.
const BREATH: i32 = 49;

/// Where a fireball meets something.
pub(super) const FIREBALL: Spec = Spec { potion: HARMING, colour: -5_675_670, particle: BREATH, radius: 6.0, per_tick: -0.0, on_use: 0.0, ticks: 2400, again: 20 };
/// By the flaming dragon's head.
pub(super) const FLAMES: Spec = Spec { potion: HARMING, colour: 14_418_159, particle: BREATH, radius: 5.0, per_tick: -0.0, on_use: 0.0, ticks: 200, again: 0 };

/// A cloud of `breath` at `at`, turned `yaw`.
pub(super) fn spawn(world: &mut World, layer: Entity, at: [f32; 3], yaw: f32, breath: Spec) -> Option<Entity> {
    clouds::spawn(world, layer, at, yaw, breath, super::DRAGON)
}
