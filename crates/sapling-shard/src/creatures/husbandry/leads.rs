//! Leads (`minecraft:leashable`): a player's lead ties a mob to it; the mob is pulled along past
//! [`SOFT`], dragged past [`HARD`], and the lead snaps past [`BREAKS`], dropping. A mob is kept on
//! its lead by its holder's `UniqueID` (`LeasherID`); a holder not found again in [`GRACE`] ticks
//! lets it go, as a player who left does.
// TODO: the distances and the drag are the Java game's; BDS's leashable says 6/10 soft/hard but not how hard it pulls.

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use bevy_ecs::prelude::*;
use sapling_world::ItemStack;

use super::{Raised, distance, keep_for_good, shown};
use crate::body::Body;
use crate::components::{InLayer, Position};
use crate::tags::{Marks, UniqueId};

pub(super) const LEAD: &str = "minecraft:lead";
const SOFT: f32 = 4.0;
const HARD: f32 = 6.0;
/// BDS kept a slowed cow's lead with its holder 11 blocks off and snapped it at 11.99 (bds-uuknot.txt).
pub(super) const BREAKS: f32 = 11.0;
/// A player holder's middle, whence a snapping lead is heard (bds-uuknot.txt).
const HOLDER_MIDDLE: f32 = 0.9;
const GRACE: u64 = 100;
/// How hard a lead past [`HARD`] drags, per block past it.
const DRAG: f32 = 0.08;

/// Where the holder `holder` is: a player's feet, or a knot's.
fn holder_at(world: &World, holder: Entity) -> Option<([f32; 3], Entity)> {
    let layer = world.get::<InLayer>(holder)?.0;
    world.get::<Position>(holder).map(|position| position.feet).or_else(|| world.get::<Body>(holder).map(|body| body.at)).map(|at| (at, layer))
}

/// The player, knot or wandering trader kept under `unique_id`.
fn find_holder(world: &mut World, unique_id: i64) -> Option<Entity> {
    shown::player_of(world, unique_id).or_else(|| world.query_filtered::<(Entity, &UniqueId), With<Body>>().iter(world).find(|(_, id)| id.0 == unique_id).map(|(entity, _)| entity))
}

/// The holder's `UniqueID`: a player's, or a kept entity's.
pub(super) fn unique_id_of(world: &World, holder: Entity) -> Option<i64> {
    world.get::<Marks>(holder).map(|marks| marks.unique_id).or_else(|| world.get::<UniqueId>(holder).map(|id| id.0))
}

/// `holder` holds the mob's lead now.
pub(in crate::creatures) fn tie(world: &mut World, entity: Entity, holder: Entity) {
    let Some(unique_id) = unique_id_of(world, holder) else { return };
    if let Some(mut raised) = world.get_mut::<Raised>(entity) {
        (raised.kept.leasher, raised.holder, raised.looked_since) = (Some(unique_id), Some(holder), 0);
    }
    keep_for_good(world, entity);
    shown::links(world, entity);
}

/// A player's lead goes on a mob of a kind that takes one and has none. False where it does not.
pub(super) fn leash(world: &mut World, entity: Entity, kind: &str, player: Entity) -> bool {
    let free = world.get::<Raised>(entity).is_some_and(|raised| raised.kept.leasher.is_none());
    if !free || !sapling_data::husbandry(kind).is_some_and(|rules| rules.leashable) {
        return false;
    }
    tie(world, entity, player);
    shown::sound_unattributed(world, entity, "lead.leash", 0.5);
    true
}

/// The mob's lead comes off and drops by it, heard as `sound`.
pub(super) fn untie(world: &mut World, entity: Entity, sound: &str) {
    if let_go(world, entity) {
        shown::sound(world, entity, sound);
    }
}

/// The mob's lead comes off and drops by it; false when it had none.
fn let_go(world: &mut World, entity: Entity) -> bool {
    if !unleash(world, entity, true) {
        return false;
    }
    drop_lead(world, entity);
    true
}

/// A lead drops from the mob's middle.
fn drop_lead(world: &mut World, entity: Entity) {
    if let Some((layer, _, _, body)) = shown::of(world, entity) {
        let lead = ItemStack { name: LEAD.to_owned(), count: 1, damage: 0, block: None, tag: None };
        crate::drops::toss(world, layer, lead, [body.at[0], body.at[1] + body.size[1] / 2.0, body.at[2]]);
    }
}

/// The mob's lead comes off, its viewers told `RemoveLeash` (`dropped` 1 when the lead drops, as
/// BDS's; bds-k2trader.txt) before its flags; false when it had none.
fn unleash(world: &mut World, entity: Entity, dropped: bool) -> bool {
    let Some(mut raised) = world.get_mut::<Raised>(entity).filter(|raised| raised.kept.leasher.is_some()) else { return false };
    (raised.kept.leasher, raised.holder, raised.looked_since) = (None, None, 0);
    if let Some(id) = world.get::<crate::components::NetId>(entity).map(|id| id.0) {
        let event = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::RemoveLeash, data: i32::from(dropped), fire_at_position: None };
        shown::tell(world, entity, vec![sapling_net::encode(&event)]);
    }
    shown::links(world, entity);
    true
}

/// The mob's lead comes off and goes with no drop and no sound: its holder went (a wandering trader
/// despawning takes its llamas with it, bds-k2trader.txt).
pub(in crate::creatures) fn release(world: &mut World, entity: Entity) {
    unleash(world, entity, false);
}

/// The lead pulled too far snaps, heard from its holder at `holder` (bds-uuknot.txt).
fn snap(world: &mut World, entity: Entity, holder: [f32; 3]) {
    if let_go(world, entity) {
        heard_from_holder(world, entity, "lead.break", holder);
    }
}

/// `name` heard as no one's from the middle of a player holder whose feet are at `holder`, by the
/// viewers of the mob `entity` (bds-uuknot.txt).
pub(super) fn heard_from_holder(world: &mut World, entity: Entity, name: &str, [x, y, z]: [f32; 3]) {
    let position = acacia_proto::types::Vec3f { x, y: y + HOLDER_MIDDLE, z };
    shown::tell(world, entity, vec![shown::unattributed(name, position)]);
}

/// A wandering trader's llama (`llama_wandering_trader`): once off its lead it tames itself, owned
/// by no one, as BDS's did a tick after their trader died (bds-k2trader.txt).
#[derive(Component)]
pub(crate) struct TamedWhenLoose;

/// A held mob whose holder died, with the holder's middle then: BDS's let go the tick after the
/// death and dropped the lead, `lead.break` heard there, the tick after that (bds-k2trader.txt).
#[derive(Component, Clone)]
struct Orphaned(acacia_proto::types::Vec3f);

/// A tick of the mob's lead: its kept holder is found again, or given up on; one too far snaps it;
/// a dying holder's lets go ([`Orphaned`]).
pub(super) fn hold(world: &mut World, entity: Entity, now: u64) {
    if let Some(Orphaned(middle)) = world.get::<Orphaned>(entity).cloned() {
        if !unleash(world, entity, true) {
            world.entity_mut(entity).remove::<Orphaned>();
            drop_lead(world, entity);
            shown::tell(world, entity, vec![shown::unattributed("lead.break", middle)]);
        }
        return;
    }
    let Some(raised) = world.get::<Raised>(entity) else { return };
    if raised.kept.leasher.is_none() && world.get::<TamedWhenLoose>(entity).is_some() {
        world.entity_mut(entity).remove::<TamedWhenLoose>();
        if let Some(kind) = world.get::<crate::entities::Kind>(entity).map(|kind| kind.0.clone()) {
            super::owners::tame_by_upgrade(world, entity, &kind);
        }
        return;
    }
    let (Some(leasher), holder, since) = (raised.kept.leasher, raised.holder, raised.looked_since) else { return };
    if let Some(body) = holder.filter(|holder| crate::creatures::is_dying(world, *holder)).and_then(|dying| world.get::<Body>(dying)) {
        let middle = acacia_proto::types::Vec3f { x: body.at[0], y: body.at[1] + body.size[1] / 2.0, z: body.at[2] };
        world.entity_mut(entity).insert(Orphaned(middle));
        return;
    }
    let here = world.get::<InLayer>(entity).map(|layer| layer.0).zip(world.get::<Body>(entity).map(|body| body.at));
    match holder.and_then(|holder| holder_at(world, holder)) {
        Some((at, layer)) if here.is_some_and(|(mine, feet)| mine == layer && distance(feet, at) <= BREAKS) => {}
        Some((at, layer)) if here.is_some_and(|(mine, _)| mine == layer) => snap(world, entity, at),
        Some(_) => untie(world, entity, "lead.break"),
        None => match find_holder(world, leasher) {
            Some(found) => tie(world, entity, found),
            None if since == 0 => world.get_mut::<Raised>(entity).expect("read above").looked_since = now,
            None if now - since > GRACE => untie(world, entity, "lead.break"),
            None => {}
        },
    }
}

/// Before the mob's move: past [`SOFT`] it walks to its holder, past [`HARD`] it is dragged too.
pub(super) fn pull(world: &mut World, entity: Entity, layer: Entity) -> bool {
    let Some(holder) = world.get::<Raised>(entity).and_then(|raised| raised.holder) else { return false };
    let (Some((at, _)), Some(body)) = (holder_at(world, holder), world.get::<Body>(entity).copied()) else { return false };
    let far = distance(body.at, at);
    if far <= SOFT {
        return false;
    }
    let kind = world.get::<crate::entities::Kind>(entity).map(|kind| kind.0.clone()).unwrap_or_default();
    let walks = sapling_data::husbandry(&kind).map_or(0.25, |rules| rules.walks);
    crate::creatures::legs::walk(world, entity, layer, &body, [at[0], at[2]], walks, SOFT / 2.0);
    if far > HARD {
        let pulled = (far - HARD) * DRAG / far;
        if let Some(mut body) = world.get_mut::<Body>(entity) {
            for axis in [0, 2] {
                body.motion[axis] += (at[axis] - body.at[axis]) * pulled;
            }
        }
    }
    true
}
