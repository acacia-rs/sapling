//! What trial spawners and vaults tell and give out: their sounds, their level events and the
//! items they eject.

use acacia_proto::packets::{LevelEvent, LevelEventEvent, LevelSoundEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, Item};

use super::dice;
use crate::components::Layer;

fn vec([x, y, z]: [f32; 3]) -> Vec3f {
    Vec3f { x, y, z }
}

/// The sound `name` at `at`, heard by those who see `pos`.
pub(super) fn sound(world: &mut World, layer: Entity, pos: BlockPos, name: &str, at: [f32; 3], data: i32) {
    let sound = LevelSoundEvent { sound_id: name.into(), position: vec(at), extra_data: data, entity_type: String::new(), is_baby_mob: false, is_global: false, entity_unique_id: -1, fire_at_position: None };
    let told = sapling_net::encode(&sound);
    crate::blocks::to_viewers(world, layer, pos, &[told]);
}

/// The level event `event` at `at`, seen by those who see `pos`.
pub(super) fn event(world: &mut World, layer: Entity, pos: BlockPos, event: LevelEventEvent, at: [f32; 3], data: i32) {
    let told = sapling_net::encode(&LevelEvent { event, position: vec(at), data });
    crate::blocks::to_viewers(world, layer, pos, &[told]);
}

/// Rolls the loot table `table` (its path in the packs) with the spawner blocks' dice.
pub(super) fn roll(world: &mut World, table: &str) -> Vec<(String, u32)> {
    let mut dice = dice(world);
    sapling_data::table_loot(table, &mut || dice.fraction())
}

/// A normal number (mean 0, deviation 1) by the spawner blocks' dice (Box-Muller).
pub(super) fn gaussian(world: &mut World) -> f32 {
    let mut dice = dice(world);
    let (a, b) = (dice.fraction().max(f32::MIN_POSITIVE), dice.fraction());
    (-2.0 * a.ln()).sqrt() * (std::f32::consts::TAU * b).cos()
}

/// Lets `count` of `item` go from `at` at `motion`; nothing for an item no client knows.
pub(super) fn item(world: &mut World, layer: Entity, (name, count): (String, u32), at: [f32; 3], motion: [f32; 3]) {
    let Some(stack) = world.get::<Layer>(layer).and_then(|level| crate::inventory::stored(&Item::new(name, count.min(64) as u8), level.world.registry())) else { return };
    crate::drops::launch(world, layer, stack, at, motion);
}
