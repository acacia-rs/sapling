//! A witch's thrown potions (docs/research/mob-kinds.md, "The witch"): which it throws, by its
//! definition's `shooter` list, and what one does where it breaks.
// TODO: a mob splashed is given nothing, the witch drinks nothing, and the colours are of the four
// potions it throws at players alone.

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use crate::blocks::Dice;
use crate::components::{InLayer, Position, in_world};
use crate::{late_sounds, vitals};

const SLOWNESS: i16 = 17;
const HARMING: i16 = 23;
const POISON: i16 = 25;
const WEAKNESS: i16 = 34;
/// Slowness from this far off, poison while its prey has this much health, and weakness, one
/// time in four, from this near.
const SLOWS_FROM: f32 = 8.0;
const POISONS_ABOVE: f32 = 8.0;
const WEAKENS_WITHIN: f32 = 3.0;
const WEAKENS: f32 = 0.25;
/// A potion reaches those within this of where it broke.
const REACH: f32 = 4.0;
/// The colours of its splash, as BDS sent them (research/captures/bds-witch.txt) but weakness's,
/// which is the Java game's.
const COLOURS: [(i16, u32); 4] = [(SLOWNESS, 0xFF8B_AFE0), (HARMING, 0xFFA9_656A), (POISON, 0xFF87_A363), (WEAKNESS, 0xFF48_4D48)];

/// The potion a witch `far` from its prey throws at it.
pub(super) fn choose(world: &mut World, prey: Entity, far: f32) -> i16 {
    if far >= SLOWS_FROM && !vitals::is_under(world, prey, "slowness") {
        return SLOWNESS;
    }
    if vitals::health_left(world, prey) >= POISONS_ABOVE && !vitals::is_under(world, prey, "poison") {
        return POISON;
    }
    let lucky = world.get_resource_or_insert_with(Dice::default).fraction() < WEAKENS;
    match far <= WEAKENS_WITHIN && lucky && !vitals::is_under(world, prey, "weakness") {
        true => WEAKNESS,
        false => HARMING,
    }
}

/// The potion of data value `data` breaks at `at`, flying from `from`, on the player `struck` if it
/// met one: that one gets it in full, others within reach a share by how far off they are.
pub(super) fn burst(world: &mut World, layer: Entity, at: [f32; 3], from: [f32; 3], data: i16, struck: Option<Entity>) {
    let colour = COLOURS.iter().find(|(potion, _)| *potion == data).map_or(COLOURS[1].1, |(_, colour)| *colour);
    let [x, y, z] = at;
    let splash = LevelEvent { event: LevelEventEvent::ParticleSplash, position: Vec3f { x, y, z }, data: colour as i32 };
    let cell = at.map(|c| c.floor() as i32);
    crate::view::to_viewers(world, layer, cell, &[sapling_net::encode(&splash)]);
    late_sounds::play_later(world, 1, layer, cell, super::arrows::sound("glass", at, ""));
    for player in in_world(world) {
        let Some(feet) = world.get::<Position>(player).map(|position| position.feet).filter(|_| world.get::<InLayer>(player).is_some_and(|on| on.0 == layer)) else { continue };
        let far = (0..3).map(|axis| (feet[axis] - at[axis]).powi(2)).sum::<f32>().sqrt();
        let share = if struck == Some(player) { 1.0 } else { 1.0 - far / REACH };
        if share > 0.0 {
            vitals::splashed(world, player, data, share, from);
        }
    }
}
