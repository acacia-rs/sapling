//! A witch's thrown potions (docs/research/mob-kinds.md, "The witch"): which it throws, by its
//! definition's `shooter` list, and what one does where it breaks, to players and mobs alike, a
//! player's thrown splash potion too (shots/throws.rs).

use acacia_proto::packets::{LevelEvent, LevelEventEvent};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;

use super::{Creature, Dying};
use crate::blocks::Dice;
use crate::body::Body;
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

/// The potion of data value `data` breaks at `at`, flying from `from`, on the player or mob `struck` if it
/// met one: that one gets it in full, others within reach a share by how far off they are.
pub(super) fn burst(world: &mut World, layer: Entity, at: [f32; 3], from: [f32; 3], data: i16, struck: Option<Entity>) {
    let [x, y, z] = at;
    let splash = LevelEvent { event: LevelEventEvent::ParticleSplash, position: Vec3f { x, y, z }, data: sapling_data::shots::potion_colour(data) };
    let cell = at.map(|c| c.floor() as i32);
    crate::view::to_viewers(world, layer, cell, &[sapling_net::encode(&splash)]);
    late_sounds::play_later(world, 1, layer, cell, super::arrows::sound("glass", at, ""));
    // BDS shares by how far the break is from the nearest point of each box (bds-clouds2.txt).
    let share = |body: &Body, who: Entity| match struck == Some(who) {
        true => 1.0,
        false => {
            let (low, high) = body.bounds();
            1.0 - (0..3).map(|axis| (low[axis] - at[axis]).max(at[axis] - high[axis]).max(0.0).powi(2)).sum::<f32>().sqrt() / REACH
        }
    };
    for player in in_world(world) {
        let Some(feet) = world.get::<Position>(player).map(|position| position.feet).filter(|_| world.get::<InLayer>(player).is_some_and(|on| on.0 == layer)) else { continue };
        let share = share(&Body::new(feet, [0.0; 3], vitals::SIZE), player);
        if share > 0.0 {
            vitals::splashed(world, player, data, share, from);
        }
    }
    let mobs: Vec<(Entity, f32)> = world
        .query_filtered::<(Entity, &InLayer, &Body), (With<Creature>, Without<Dying>)>()
        .iter(world)
        .filter(|(_, on, _)| on.0 == layer)
        .map(|(mob, _, body)| (mob, share(body, mob)))
        .filter(|(_, share)| *share > 0.0)
        .collect();
    for (mob, share) in mobs {
        super::afflictions::splashed(world, mob, data, share);
    }
}
