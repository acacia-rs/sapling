//! Where a player's shot leaves from and how fast it goes: from its eyes, 0.1 under them for the
//! kinds whose pack has `offset: [0, -0.1, 0]` (arrows, tridents, snowballs; potions, eggs, pearls
//! and bottles leave from the eyes themselves, bds-f14cthrow.txt), along its look, turned up by an `angle_offset`.
//! Each part of the look is moved by a Gaussian draw times 0.0075 times the uncertainty before
//! the power multiplies it; the player's own motion is not added (BDS 1424020c0; bds-t2ebow.txt:
//! arrows loosed walking east left at 2.99 and 3.02, as standing).

use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;

use crate::blocks::Dice;
use crate::components::Position;

pub(super) const BELOW_EYES: f32 = 0.1;
const SPREAD: f32 = 0.0075;

/// The way a player at `position` looks, turned `tilt` degrees further up, as a unit vector.
pub(super) fn look(position: &Position, tilt: f32) -> [f32; 3] {
    let (yaw, pitch) = (position.yaw.to_radians(), position.pitch.to_radians());
    let raised = (position.pitch + tilt).to_radians();
    let along = [-yaw.sin() * pitch.cos(), -raised.sin(), yaw.cos() * pitch.cos()];
    let long = along.iter().map(|c| c * c).sum::<f32>().sqrt().max(1e-4);
    along.map(|c| c / long)
}

/// Where a shot leaves a player at `position`, `below` its eyes, and its motion at `power` along its
/// look turned `tilt` up, each part off by `uncertainty` times the spread.
pub(super) fn from_eyes(world: &mut World, position: &Position, below: f32, power: f32, tilt: f32, uncertainty: f32) -> ([f32; 3], [f32; 3]) {
    let feet = position.feet;
    let from = [feet[0], feet[1] + EYE_HEIGHT - below, feet[2]];
    let along = look(position, tilt);
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let motion = along.map(|c| (c + dice.gaussian() * SPREAD * uncertainty) * power);
    (from, motion)
}

/// `motion` turned `degrees` about the up axis (a crossbow's side shots).
pub(super) fn turned(motion: [f32; 3], degrees: f32) -> [f32; 3] {
    let (sin, cos) = degrees.to_radians().sin_cos();
    [motion[0] * cos - motion[2] * sin, motion[1], motion[0] * sin + motion[2] * cos]
}
