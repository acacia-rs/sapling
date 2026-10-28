//! Soul Speed: boots with it quicken a player on soul sand and soul soil. BDS adds `0.03 · (1 +
//! 0.35 · level)` to its walking speed as a "Sprinting speed boost" modifier, retold every tick it
//! stands there, and tells the plain speed once it steps off (research/captures/bds-soles.txt,
//! `bds_command_probe soles`: 0.1615 in Soul Speed III).
// TODO: the boots wear as BDS's do on soul sand, and the soul particles, were not seen in the
// capture and are not done.

use bevy_ecs::prelude::*;
use sapling_world::World as Level;

use super::effects::{Paced, tell_pace};
use crate::components::{NetId, Position};
use crate::creatures::holds;

const SOULED: &[&str] = &["minecraft:soul_sand", "minecraft:soul_soil"];
const SPEED: f32 = 0.03;
const PER_LEVEL: f32 = 0.35;

/// What Soul Speed at `level` adds to the walking speed.
fn quickens(level: u8) -> f32 {
    if level == 0 { 0.0 } else { SPEED * (1.0 + PER_LEVEL * f32::from(level)) }
}

/// Whether a player stands on soul sand or soul soil.
pub(super) fn on_soul(level: &Level, position: Position) -> bool {
    let [x, y, z] = position.feet;
    position.on_ground && holds(level, [x.floor() as i32, (y - 0.5).floor() as i32, z.floor() as i32], SOULED)
}

/// A tick of a player's walk, on soul sand in boots with Soul Speed at `level` or off it (`None`).
pub(super) fn tread(world: &mut World, player: Entity, level: Option<u8>) {
    let soles = level.map_or(0.0, quickens);
    let was = world.get::<Paced>(player).map_or(0.0, |paced| paced.soles);
    let Some(id) = world.get::<NetId>(player).map(|id| id.0).filter(|_| soles != 0.0 || was != 0.0) else { return };
    world.entity_mut(player).insert_if_new(Paced::default());
    world.get_mut::<Paced>(player).expect("put above").soles = soles;
    tell_pace(world, player, id);
}

#[cfg(test)]
mod tests {
    #[test]
    fn soul_speed_three_adds_what_bds_told() {
        assert!((super::quickens(3) - 0.0615).abs() < 1e-6);
    }
}
