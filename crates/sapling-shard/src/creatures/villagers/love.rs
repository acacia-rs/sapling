//! Villager breeding (its pack's native `make_love` and `receive_love`; docs/research/villagers.md,
//! "Breeding"): a grown, awake villager willing by the food in its pantry, at times, finds another
//! grown one near with a free bed near, and both fall in love on one tick; the one that looked walks
//! to the other, both throw love particles, and [`LASTS`] ticks on the walker has a baby at its
//! cell's middle and each eats what made it willing.
// TODO: whether BDS's mate must be willing too is not settled (bds-g11breed-2.txt bred with one fed).

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use acacia_proto::types::MetadataFlags1;
use bevy_ecs::prelude::*;

use super::pantry::Pantry;
use super::{Trader, VILLAGER, beds};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, NetId, Tick};
use crate::entities;

/// Bread, or one of the crops, a villager must hold this many of to be willing (the pack's
/// `shareables` `want_amount` for bread; the Java game's 12 food points for the crops).
const WILLING: [(&str, u32); 4] = [("minecraft:bread", 3), ("minecraft:carrot", 12), ("minecraft:potato", 12), ("minecraft:beetroot", 12)];
/// A willing villager looks for a mate a tick in this many, this far off: BDS's fed pairs that kept
/// within 8 fell in love 19 to 29 s on, those that strolled apart did not (bds-g11love-2.txt,
/// bds-g11love-3.txt, bds-g11share.txt, bds-g11breed-2.txt).
const LOOK_ODDS: u32 = 1000;
const SEEKS: f32 = 8.0;
/// Ticks from falling in love to the baby (bds-g11love-2.txt: `INLOVE` 300 ticks on both).
const LASTS: u64 = 300;
/// The walker stops this near its mate, walking at this speed.
const NEAR: f32 = 1.0;
const WALKS: f32 = 0.5;
/// A tick in this many one in love throws particles.
const PARTICLE_ODDS: u32 = 30;
/// Ticks a parent rests before it may fall in love again.
const RESTS: u64 = 6000;

#[derive(Component)]
pub(super) struct Love {
    mate: Entity,
    ends: u64,
    walks: bool,
}

#[derive(Component)]
pub(super) struct Rests(u64);

/// What `pantry` is willing by, and how many of it breeding eats.
fn willing_by(pantry: &Pantry) -> Option<(&'static str, u32)> {
    WILLING.into_iter().find(|(name, count)| pantry.count(name) >= *count)
}

/// A grown villager, awake, not in love or resting from it.
fn may_love(world: &World, entity: Entity, now: u64) -> bool {
    world.get::<Trader>(entity).is_some_and(|trader| trader.leaves_at.is_none())
        && world.get::<Love>(entity).is_none()
        && world.get::<beds::Lying>(entity).is_none()
        && world.get::<Rests>(entity).is_none_or(|rests| now >= rests.0)
        && !crate::creatures::is_baby(world, entity)
}

fn willing(world: &World, entity: Entity) -> bool {
    world.get::<Pantry>(entity).and_then(willing_by).is_some()
}

fn show_love(world: &mut World, entity: Entity, on: bool) {
    let (Some(id), Some(layer), Some(body)) = (world.get::<NetId>(entity).map(|id| id.0), world.get::<InLayer>(entity).map(|layer| layer.0), world.get::<Body>(entity).copied()) else { return };
    let flags = sapling_data::mob_flags_set(id, entities::mob_flags(world, entity, VILLAGER, MetadataFlags1::INLOVE, on));
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&flags)]);
}

/// A willing villager's look: the nearest other willing one within [`SEEKS`], with a bed free.
fn find_mate(world: &mut World, entity: Entity, layer: Entity, body: &Body, now: u64) -> Option<Entity> {
    let mut villagers = world.query::<(Entity, &Trader, &Body, &InLayer)>();
    let near = |at: [f32; 3]| (0..3).map(|axis| (at[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt();
    let mut found: Vec<(Entity, f32)> = villagers.iter(world).filter(|(other, _, other_body, in_layer)| *other != entity && in_layer.0 == layer && near(other_body.at) <= SEEKS).map(|(other, _, other_body, _)| (other, near(other_body.at))).collect();
    found.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mate = found.into_iter().map(|(other, _)| other).find(|other| may_love(world, *other, now))?;
    beds::one_free(world, layer, body.at).then_some(mate)
}

/// Before its move: a villager in love walks to its mate and has its baby in time; a willing one
/// may find a mate. False for one not in love.
pub(super) fn court(world: &mut World, entity: Entity, layer: Entity, body: &Body) -> bool {
    let now = world.resource::<Tick>().0;
    let Some((mate, ends, walks)) = world.get::<Love>(entity).map(|love| (love.mate, love.ends, love.walks)) else {
        if !may_love(world, entity, now) || !willing(world, entity) || world.get_resource_or_insert_with(Dice::default).roll(LOOK_ODDS) != 0 {
            return false;
        }
        let Some(mate) = find_mate(world, entity, layer, body, now) else { return false };
        world.entity_mut(entity).insert(Love { mate, ends: now + LASTS, walks: true });
        world.entity_mut(mate).insert(Love { mate: entity, ends: now + LASTS, walks: false });
        show_love(world, entity, true);
        show_love(world, mate, true);
        return true;
    };
    let Some(to) = world.get::<Body>(mate).map(|other| other.at).filter(|_| world.get::<Love>(mate).is_some()) else {
        world.entity_mut(entity).remove::<Love>();
        show_love(world, entity, false);
        return false;
    };
    if world.get_resource_or_insert_with(Dice::default).roll(PARTICLE_ODDS) == 0 {
        let Some(id) = world.get::<NetId>(entity).map(|id| id.0) else { return false };
        let particles = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::LoveParticles, data: 0, fire_at_position: None };
        entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), vec![sapling_net::encode(&particles)]);
    }
    if walks && now >= ends {
        breed(world, entity, mate, layer, body);
        return true;
    }
    if walks && (to[0] - body.at[0]).hypot(to[2] - body.at[2]) > NEAR {
        crate::creatures::legs::walk(world, entity, layer, body, [to[0], to[2]], WALKS, NEAR);
    } else {
        crate::creatures::legs::rest(world, entity);
    }
    true
}

/// The walker `parent` and `mate` have their baby at the middle of the walker's cell, heard `born`
/// (bds-g11love-2.txt), eat what made them willing and rest.
fn breed(world: &mut World, parent: Entity, mate: Entity, layer: Entity, body: &Body) {
    let now = world.resource::<Tick>().0;
    for one in [parent, mate] {
        world.entity_mut(one).remove::<Love>().insert(Rests(now + RESTS));
        show_love(world, one, false);
        if let Some(mut pantry) = world.get_mut::<Pantry>(one) {
            if let Some((name, count)) = willing_by(&pantry) {
                pantry.take(name, count);
            }
        }
    }
    let feet = body.at.map(|c| c.floor() + 0.5);
    let feet = [feet[0], body.at[1].floor() + 0.5, feet[2]];
    crate::creatures::spawn_as(world, layer, VILLAGER, feet, sapling_api::SpawnReason::Bred, 0.0, Some("minecraft:entity_born"));
}
