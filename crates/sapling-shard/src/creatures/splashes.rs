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
use crate::vitals;

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
/// How far round each box the share is measured from: BDS's three watchers' shares were 1 - d/4
/// to within a tick for d the gap to their boxes grown by this.
const GROWN: f32 = 0.05;

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

/// The share of a potion broken at `at` for the box `(low, high)`: by how far the break is from
/// the box's nearest point, the box grown by [`GROWN`] (research/captures/bds-splashshare.txt).
fn share_of((low, high): ([f32; 3], [f32; 3]), at: [f32; 3]) -> f32 {
    1.0 - (0..3).map(|axis| (low[axis] - GROWN - at[axis]).max(at[axis] - high[axis] - GROWN).max(0.0).powi(2)).sum::<f32>().sqrt() / REACH
}

/// The potion of data value `data` breaks at `at`, flying from `from`, on the player or mob `struck` if it
/// met one: that one gets it in full, others within reach a share by how far off they are. What they
/// show of it waits for the next tick's sync, the potion going then too (flight.rs).
pub(super) fn burst(world: &mut World, layer: Entity, at: [f32; 3], from: [f32; 3], data: i16, struck: Option<Entity>) {
    let [x, y, z] = at;
    let splash = LevelEvent { event: LevelEventEvent::ParticleSplash, position: Vec3f { x, y, z }, data: sapling_data::shots::potion_colour(data) };
    let cell = at.map(|c| c.floor() as i32);
    let shown: Vec<_> = std::iter::once(sapling_net::encode(&splash)).chain(super::landing::cracks("minecraft:splash_potion", super::landing::POTION_CRACKS, at)).collect();
    crate::view::to_viewers(world, layer, cell, &shown);
    crate::entities::synced_next_tick(world, |world| give(world, layer, at, from, data, struck));
}

/// The potion's effects on those it reaches.
fn give(world: &mut World, layer: Entity, at: [f32; 3], from: [f32; 3], data: i16, struck: Option<Entity>) {
    let share = |body: &Body, who: Entity| match struck == Some(who) {
        true => 1.0,
        false => share_of(body.bounds(), at),
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

#[cfg(test)]
#[test]
fn bds_watchers_got_their_share_of_900_ticks() {
    // research/captures/bds-splashshare.txt: the break and each watcher's feet, and the ticks BDS gave.
    let at = [3.15, -25.7998, 4.5];
    for (feet, ticks) in [([3.5, -26.0, 6.6], 506), ([5.6, -26.0, 4.5], 428), ([3.5, -26.0, 3.5], 754)] {
        let bounds = ([feet[0] - 0.3, feet[1], feet[2] - 0.3], [feet[0] + 0.3, feet[1] + 1.8, feet[2] + 0.3]);
        assert_eq!((900.0 * share_of(bounds, at) + 0.5) as u32, ticks, "{feet:?}");
    }
}

#[cfg(test)]
#[test]
fn bds_watchers_were_harmed_and_healed_by_their_share() {
    // research/captures/bds-t2esplash.txt: harming and healing broken on Steve, the watcher off at each spot.
    let at = [3.15, -25.7998, 4.5];
    for (feet, harmed, healed) in [([3.5, -26.0, 6.6], 3.0, 2.0), ([5.6, -26.0, 4.5], 3.0, 2.0), ([3.5, -26.0, 3.5], 5.0, 3.0)] {
        let share = share_of(Body::new(feet, [0.0; 3], vitals::SIZE).bounds(), at);
        let strength = |base| vitals::shared_strength(base, 0, share);
        assert_eq!((strength(vitals::effects::HARMS), strength(vitals::effects::HEALS)), (harmed, healed), "{feet:?}");
    }
}
