//! Breeding: a grown one fed its kind's breeding item falls in love, looks for a mate of a kind it
//! breeds with that is in love too, walks to it and, once they have been near a while, a baby is
//! born beside it, with experience; both then rest a while. Mobs that follow food walk after a
//! player holding it.
// TODO: love, the cooldown, the courting distances and the experience are the Java game's numbers
// (docs/research/husbandry.md, "Unverified"); a baby's coat is not its parents'.

use acacia_proto::packets::EntityEventEventId;
use acacia_proto::types::MetadataFlags1 as Flags;
use bevy_ecs::prelude::*;

use super::{Raised, ageing, distance, shown};
use crate::blocks::Dice;
use crate::body::Body;
use crate::components::{InLayer, Position, in_world};
use crate::entities::Kind;
use crate::inventory::Inventory;
use crate::tags::Marks;

/// Ticks a mob stays in love, and rests after breeding.
pub(super) const LOVE: i32 = 600;
const COOLDOWN: i32 = 6000;
/// It looks this far for a mate, every this many ticks, and they breed after this many ticks this near.
const SEEKS: f32 = 8.0;
const LOOKS: u64 = 10;
const COURTS: u32 = 60;
const NEAR: f32 = 3.0;
/// The most experience a birth leaves; it leaves from 1.
const EXPERIENCE: u32 = 7;
/// Kinds whose breeding lays the egg item instead of a baby (the pack's `causes_pregnancy` and
/// `spawn_entity`).
const LAYS_EGG: [(&str, &str); 1] = [("minecraft:sniffer", "minecraft:sniffer_egg")];
/// A horse and a donkey have a mule.
const MIXED: [(&str, &str, &str); 1] = [("minecraft:horse", "minecraft:donkey", "minecraft:mule")];
/// A player this near holding its food is followed.
const TEMPTS: f32 = 10.0;
const TEMPTED_NEAR: f32 = 2.5;

/// Whether the grown mob may fall in love on `item` now.
fn may_love(world: &World, entity: Entity, kind: &str, item: &str) -> bool {
    let (Some(rules), Some(raised)) = (sapling_data::husbandry(kind), world.get::<Raised>(entity)) else { return false };
    let healthy = world.get::<crate::creatures::Health>(entity).is_none_or(|health| health.left() >= health.max());
    // A turtle with an egg does not fall in love (JAVA `Turtle.canFallInLove`).
    rules.breed.contains(&item) && !raised.is_baby() && !super::nesting::is_nesting(world, entity) && raised.kept.in_love == 0 && raised.kept.breed_cooldown == 0 && (!rules.breed_tame || raised.is_tamed()) && (!rules.breed_healthy || healthy)
}

/// The mob is fed `item` by `player`: it falls in love if it may. False where it may not.
pub(super) fn fall_in_love(world: &mut World, entity: Entity, kind: &str, item: &str, player: Entity) -> bool {
    if !may_love(world, entity, kind, item) {
        return false;
    }
    let cause = world.get::<Marks>(player).map_or(0, |marks| marks.unique_id);
    if let Some(mut raised) = world.get_mut::<Raised>(entity) {
        (raised.kept.in_love, raised.kept.love_cause) = (LOVE, cause);
    }
    shown::flag(world, entity, Flags::INLOVE, true);
    shown::event(world, entity, EntityEventEventId::InLoveParticles, 0);
    true
}

/// A tick of the mob's love and its rest after breeding.
pub(super) fn tick(world: &mut World, entity: Entity, _kind: &str, _now: u64) {
    let Some(mut raised) = world.get_mut::<Raised>(entity) else { return };
    raised.kept.breed_cooldown = (raised.kept.breed_cooldown - 1).max(0);
    if raised.kept.in_love == 0 {
        return;
    }
    raised.kept.in_love -= 1;
    if raised.kept.in_love == 0 {
        raised.partner = None;
        shown::flag(world, entity, Flags::INLOVE, false);
    }
}

fn breeds_with(kind: &str, other: &str) -> bool {
    sapling_data::husbandry(kind).is_some_and(|rules| rules.mates.contains(&other))
}

/// The mate in love nearest the mob, within [`SEEKS`].
fn find_mate(world: &mut World, entity: Entity, layer: Entity, kind: &str, at: [f32; 3]) -> Option<Entity> {
    let mut found = world.query::<(Entity, &Raised, &Kind, &Body, &InLayer)>();
    let near = found.iter(world).filter(|(other, raised, other_kind, body, in_layer)| {
        *other != entity && in_layer.0 == layer && raised.kept.in_love > 0 && raised.partner.is_none_or(|(partner, _)| partner == entity) && !raised.is_baby() && breeds_with(kind, &other_kind.0) && distance(at, body.at) <= SEEKS
    });
    near.min_by(|a, b| distance(at, a.3.at).total_cmp(&distance(at, b.3.at))).map(|(other, ..)| other)
}

/// Before the move of a mob in love: it walks to its mate, and they breed once they have been
/// near long enough. False for one not in love or with no mate to be seen.
pub(super) fn court(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let (Some(raised), Some(body)) = (world.get::<Raised>(entity), world.get::<Body>(entity).copied()) else { return false };
    if raised.kept.in_love == 0 || raised.is_baby() {
        return false;
    }
    let kept = raised.partner.filter(|(partner, _)| world.get::<Raised>(*partner).is_some_and(|other| other.kept.in_love > 0) && world.get::<Body>(*partner).is_some_and(|other| distance(body.at, other.at) <= SEEKS));
    let now = world.resource::<crate::components::Tick>().0;
    let partner = match kept {
        Some(partner) => Some(partner),
        None if now.is_multiple_of(LOOKS) => find_mate(world, entity, layer, kind, body.at).map(|mate| (mate, 0)),
        None => None,
    };
    world.get_mut::<Raised>(entity).expect("read above").partner = partner;
    let Some((mate, together)) = partner else { return false };
    let Some(to) = world.get::<Body>(mate).map(|other| other.at) else { return false };
    let close = distance(body.at, to) < NEAR;
    let together = if close { together + 1 } else { together };
    world.get_mut::<Raised>(entity).expect("read above").partner = Some((mate, together));
    let speed = sapling_data::husbandry(kind).map_or(0.25, |rules| rules.walks);
    super::super::legs::walk(world, entity, layer, &body, [to[0], to[2]], speed, 1.0);
    if together >= COURTS {
        breed(world, entity, mate, layer, kind);
    }
    true
}

/// `parent` and `mate` have a baby beside `parent`; both rest, and experience is left.
fn breed(world: &mut World, parent: Entity, mate: Entity, layer: Entity, kind: &str) {
    let mate_kind = world.get::<Kind>(mate).map(|kind| kind.0.clone()).unwrap_or_default();
    let baby_kind = MIXED.iter().find(|(a, b, _)| (*a == kind && *b == mate_kind) || (*b == kind && *a == mate_kind)).map_or(kind, |(_, _, baby)| *baby).to_owned();
    for one in [parent, mate] {
        if let Some(mut raised) = world.get_mut::<Raised>(one) {
            (raised.kept.in_love, raised.kept.breed_cooldown, raised.partner) = (0, COOLDOWN, None);
        }
        shown::flag(world, one, Flags::INLOVE, false);
    }
    let Some(body) = world.get::<Body>(parent).copied() else { return };
    if super::nesting::conceives(world, parent, kind) {
        return leave_experience(world, layer, body.at);
    }
    if let Some((_, egg)) = LAYS_EGG.iter().find(|(mother, _)| *mother == kind) {
        super::products::drop_egg(world, parent, egg);
        return leave_experience(world, layer, body.at);
    }
    let owner = world.get::<Raised>(parent).filter(|raised| raised.is_tamed()).and_then(|raised| raised.kept.owner);
    if let Some(baby) = super::super::spawn_entity(world, layer, &baby_kind, body.at, sapling_api::SpawnReason::Bred) {
        ageing::be_born(world, baby, &baby_kind);
        super::super::properties::inherit(world, baby, [parent, mate], &baby_kind);
        if let Some(owner) = owner {
            super::owners::tame_to(world, baby, &baby_kind, owner);
        }
    }
    leave_experience(world, layer, body.at);
}

fn leave_experience(world: &mut World, layer: Entity, at: [f32; 3]) {
    if super::super::leaves_loot(world, layer) {
        let points = 1 + world.get_resource_or_insert_with(Dice::default).roll(EXPERIENCE) as i32;
        crate::orbs::spawn(world, layer, at, points);
    }
}

/// Before the move of a mob that follows food: it walks to the nearest player near holding some.
pub(super) fn tempted(world: &mut World, entity: Entity, layer: Entity, kind: &str) -> bool {
    let Some(rules) = sapling_data::husbandry(kind).filter(|rules| !rules.tempt.is_empty()) else { return false };
    let Some(body) = world.get::<Body>(entity).copied() else { return false };
    let holds = |player: Entity| world.get::<Inventory>(player).and_then(|inventory| inventory.in_hand().as_ref()).is_some_and(|held| rules.tempt.contains(&held.stack.name.as_str()));
    let near = in_world(world).into_iter().filter(|player| world.get::<InLayer>(*player).is_some_and(|in_layer| in_layer.0 == layer) && holds(*player));
    let feet = near.filter_map(|player| world.get::<Position>(player).map(|position| position.feet)).filter(|feet| distance(body.at, *feet) <= TEMPTS);
    let Some(to) = feet.min_by(|a, b| distance(body.at, *a).total_cmp(&distance(body.at, *b))) else { return false };
    super::super::legs::walk(world, entity, layer, &body, [to[0], to[2]], rules.walks * rules.tempt_speed, TEMPTED_NEAR);
    true
}
