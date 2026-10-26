//! A tamed mob of a kind that defends its owner (`behavior.owner_hurt_by_target` and
//! `owner_hurt_target`, a wolf) fights what hurt its owner and what its owner hit, with its
//! tamed bite, until that is dead or gone.
// TODO: the reach, the bite's pace and how far it looks are the hunters' (hunting.rs); its bite
// throws nothing.

use bevy_ecs::prelude::*;

use super::{Raised, distance, shown};
use crate::body::Body;
use crate::components::{InLayer, Position, Tick};
use crate::creatures::{Cause, Health};

/// Mobs this near their owner take up its fight, and give it up beyond this.
const RALLIES: f32 = 16.0;
/// Its bite reaches this much past its box, once in this many ticks.
const REACH: f32 = 0.8;
const EVERY: u64 = 20;
const PLAYER: [f32; 2] = [0.6, 1.8];

/// Where `foe` stands and how wide and tall it is, if it is a player or a mob still alive.
fn foe_at(world: &World, foe: Entity) -> Option<([f32; 3], [f32; 2], Entity)> {
    let layer = world.get::<InLayer>(foe)?.0;
    if let Some(position) = world.get::<Position>(foe) {
        return crate::vitals::is_mortal(world, foe).then_some((position.feet, PLAYER, layer));
    }
    let alive = world.get::<Health>(foe).is_some_and(|health| health.left() > 0.0) && world.get::<crate::creatures::Dying>(foe).is_none();
    world.get::<Body>(foe).filter(|_| alive).map(|body| (body.at, body.size, layer))
}

/// The owner's defenders near it take `foe` for theirs.
fn rally(world: &mut World, owner: Entity, foe: Entity) {
    let Some(unique_id) = world.get::<crate::tags::Marks>(owner).map(|marks| marks.unique_id) else { return };
    let Some(feet) = world.get::<Position>(owner).map(|position| position.feet) else { return };
    let now = world.resource::<Tick>().0;
    let mut defenders = world.query::<(Entity, &mut Raised, &crate::entities::Kind, &Body)>();
    for (entity, mut raised, kind, body) in defenders.iter_mut(world) {
        let defends = sapling_data::husbandry(&kind.0).is_some_and(|rules| rules.defends);
        if defends && entity != foe && raised.is_tamed() && raised.kept.owner == Some(unique_id) && !raised.kept.sitting && distance(body.at, feet) <= RALLIES {
            raised.foe = Some((foe, now));
        }
    }
}

/// Something hurt the player `owner`: its defenders fight `by`, unless `by` is one of them.
pub(crate) fn owner_hurt(world: &mut World, owner: Entity, by: Entity) {
    let own = world.get::<Raised>(by).is_some_and(|raised| raised.is_tamed() && raised.kept.owner == world.get::<crate::tags::Marks>(owner).map(|marks| marks.unique_id));
    if !own {
        rally(world, owner, by);
    }
}

/// The player `owner` hit `struck`: its defenders fight it too, unless it is one of its own.
pub(crate) fn owner_hit(world: &mut World, owner: Entity, struck: Entity) {
    owner_hurt(world, owner, struck);
}

/// Before the move of a mob with a foe: it walks at it and bites it within reach. False for one
/// with none, or whose foe is dead, gone or too far.
pub(super) fn chase(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let (Some((foe, ready)), Some(body)) = (world.get::<Raised>(entity).and_then(|raised| raised.foe), world.get::<Body>(entity).copied()) else { return false };
    let Some((at, size, _)) = foe_at(world, foe).filter(|(at, _, in_layer)| *in_layer == layer && distance(body.at, *at) <= RALLIES) else {
        world.get_mut::<Raised>(entity).expect("read above").foe = None;
        return false;
    };
    let rules = sapling_data::husbandry(kind);
    crate::creatures::legs::walk(world, entity, layer, &body, [at[0], at[2]], rules.map_or(0.3, |rules| rules.walks), 0.0);
    let now = world.resource::<Tick>().0;
    let across = (body.size[0] + size[0]) / 2.0 + REACH;
    let touches = (at[0] - body.at[0]).abs() < across && (at[2] - body.at[2]).abs() < across && at[1] < body.at[1] + body.size[1] && body.at[1] < at[1] + size[1];
    if touches && now >= ready {
        world.get_mut::<Raised>(entity).expect("read above").foe = Some((foe, now + EVERY));
        bite(world, entity, layer, kind, foe, rules.and_then(|rules| rules.tame_attack).unwrap_or(2.0));
    }
    true
}

fn bite(world: &mut World, entity: Entity, layer: Entity, kind: &str, foe: Entity, harm: f32) {
    shown::event(world, entity, acacia_proto::packets::EntityEventEventId::ArmSwing, 0);
    match world.get::<Position>(foe).is_some() {
        true => drop(crate::vitals::bitten(world, foe, harm, kind, [0.0; 3])),
        false => drop(crate::creatures::harm::wound_by(world, foe, layer, harm, Cause::Attack, None)),
    }
}
