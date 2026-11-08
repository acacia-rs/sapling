//! The mobs a hunter takes besides players: those of the families its pack's
//! `nearest_attackable_target` names (`Hunts::preys`), as raiders take villagers and iron golems,
//! zombies villagers, and iron golems monsters. A mob prey is hit as a goat's ram hits one.

use bevy_ecs::prelude::*;
use sapling_api::Effect;
use sapling_data::Hunts;

use super::super::{Creature, Dying, harm, is_alive, is_baby};
use super::{THROW, distance, in_sight};
use crate::body::Body;
use crate::components::{InLayer, Position};
use crate::entities::Kind;

/// A player's box, for one that is the prey.
const PLAYER: [f32; 2] = [0.6, 1.8];

/// Where the prey, a player or a mob, stands.
pub(in crate::creatures) fn feet_of(world: &World, prey: Entity) -> Option<[f32; 3]> {
    world.get::<Position>(prey).map(|at| at.feet).or_else(|| world.get::<Body>(prey).map(|body| body.at))
}

/// The prey's box, across and up.
pub(super) fn size_of(world: &World, prey: Entity) -> [f32; 2] {
    world.get::<Position>(prey).map_or_else(|| world.get::<Body>(prey).map_or(PLAYER, |body| body.size), |_| PLAYER)
}

/// Whether the prey is a mob, not a player.
pub(super) fn is_mob(world: &World, prey: Entity) -> bool {
    world.get::<Creature>(prey).is_some()
}

/// The feet of the mob `prey` if a hunter in `layer` at `from` that sees `sight` far may keep it.
pub(super) fn kept(world: &World, prey: Entity, layer: Entity, from: [f32; 3], sight: f32) -> Option<[f32; 3]> {
    let feet = world.get::<Body>(prey)?.at;
    (is_alive(world, prey) && world.get::<InLayer>(prey)?.0 == layer && distance(from, feet) <= sight).then_some(feet)
}

/// The nearest mob within `sight` that the hunter takes and sees.
pub(super) fn nearest(world: &mut World, hunter: Entity, layer: Entity, body: &Body, sight: f32, hunts: Hunts) -> Option<(Entity, [f32; 3])> {
    hunts.preys?;
    let near: Vec<(Entity, [f32; 3])> = world
        .query_filtered::<(Entity, &InLayer, &Kind, &Body), (With<Creature>, Without<Dying>)>()
        .iter(world)
        .filter(|(mob, on, _, at)| *mob != hunter && on.0 == layer && distance(at.at, body.at) <= sight)
        .filter(|(mob, _, kind, _)| hunts.preys_on(&sapling_data::mob_families(&kind.0), is_baby(world, *mob)))
        .map(|(mob, _, _, at)| (mob, at.at))
        .collect();
    near.into_iter().filter(|(mob, feet)| spotted(world, *mob, body.at, *feet, sight) && in_sight(world, layer, body, *feet)).min_by(|a, b| distance(a.1, body.at).total_cmp(&distance(b.1, body.at)))
}

/// Whether a hunter at `from` that sees `sight` far may take `prey` standing at `feet`: an invisible
/// one only from nearer by its armour (afflictions/invisible.rs), a player measured from its eyes.
// TODO: whether a hunter keeps prey that turns invisible was not drilled; it does here.
pub(in crate::creatures) fn spotted(world: &World, prey: Entity, from: [f32; 3], feet: [f32; 3], sight: f32) -> bool {
    let worn = |world: &World| match world.get::<crate::inventory::Inventory>(prey) {
        Some(inventory) => inventory.worn_names().flatten().count(),
        None => super::super::worn::pieces(world, prey).iter().flatten().filter(|piece| sapling_data::armour(&piece.name).is_some()).count(),
    };
    let unseen = if is_mob(world, prey) { super::super::afflictions::level(world, prey, Effect::Invisibility).is_some() } else { crate::vitals::effects::on(world, prey, Effect::Invisibility).is_some() };
    let eyes = if is_mob(world, prey) { 0.0 } else { sapling_data::EYE_HEIGHT };
    !unseen || distance([feet[0], feet[1] + eyes, feet[2]], from) <= super::super::afflictions::unseen_sight(sight, worn(world))
}

/// The hunter `by`'s hit of `harm` on the mob `prey`, `away` from it: hurt and thrown as a hit throws,
/// startled, and turned on `by` (hurt_by.rs). A mob's harm is not scaled by difficulty.
pub(in crate::creatures) fn hit(world: &mut World, layer: Entity, by: Entity, prey: Entity, harm: f32, away: [f32; 2]) {
    super::hurt_by::hurt(world, prey, by);
    let kept = 1.0 - world.get::<Kind>(prey).map_or(0.0, |kind| sapling_data::mob_traits(&kind.0).resists);
    if let Some(mut thrown) = world.get_mut::<Body>(prey).filter(|_| kept > 0.0) {
        thrown.throw([away[0] * THROW * kept, THROW * kept, away[1] * THROW * kept]);
    }
    harm::wound(world, prey, layer, harm, super::super::Cause::Attack);
    super::super::strolling::startle(world, prey);
}
