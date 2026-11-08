//! Telling players what an entity does: only those it is shown to (mobs.rs) hear of it, as BDS
//! sent a mob hidden past 4 chunks nothing but a teleport's move (research/captures/bds-farmobs.txt,
//! bds-tickcircle*.txt), and what it does in the tick it appeared waits for its announcement.

use acacia_proto::packets::{MoveEntityDelta, SetEntityData};
use bevy_ecs::prelude::*;
use bytes::Bytes;

use super::{Kind, is_new};
use crate::components::{Outgoing, Position, Tick};
use crate::mobs::{Mob, SeesMobs};
use crate::sync::{Told, pace};
use crate::view;

/// What entities did in the tick they appeared, and what was heard about them, for their viewers
/// once they are told of them: by world, cell and the entity it is about.
#[derive(Resource, Default)]
pub(crate) struct AfterAnnouncing(Vec<(Entity, [i32; 3], Option<Entity>, Vec<Bytes>)>);

/// Has `packets` sent to the viewers of `cell` once this tick's new entities are announced to them.
pub(crate) fn after_announcing(world: &mut World, layer: Entity, cell: [i32; 3], packets: Vec<Bytes>) {
    world.get_resource_or_insert_with(AfterAnnouncing::default).0.push((layer, cell, None, packets));
}

/// [`after_announcing`] for packets about `entity`: only the viewers it is shown to get them.
pub(crate) fn after_announcing_of(world: &mut World, entity: Entity, layer: Entity, cell: [i32; 3], packets: Vec<Bytes>) {
    world.get_resource_or_insert_with(AfterAnnouncing::default).0.push((layer, cell, Some(entity), packets));
}

/// What is heard as entities appear, told right after the tick's announcements, before their first
/// data: a shot's sound and `disappeared` (research/captures/bds-r1shots.txt).
#[derive(Resource, Default)]
struct OnAnnouncing(Vec<(Entity, [i32; 3], Vec<Bytes>)>);

/// Has `packets` sent to the viewers of `cell` as soon as this tick's new entities are announced.
pub(crate) fn on_announcing(world: &mut World, layer: Entity, cell: [i32; 3], packets: Vec<Bytes>) {
    world.get_resource_or_insert_with(OnAnnouncing::default).0.push((layer, cell, packets));
}

/// What players were told while a command ran, held back until after its answer: BDS sends an evented
/// mob's SetEntityData as its tick ends (research/captures/bds-bc1event.txt).
#[derive(Resource, Default)]
struct AfterAnswer(Vec<(Entity, Vec<Bytes>)>);

/// What a breaking potion has told at the sync, held to the next tick's: BDS's potion breaks after its
/// tick's actor-data sync, so the marks of what it gave show a tick after their `MobEffect`
/// (research/captures/bds-bg2player.txt, bds-bf1potion.txt). The tick it was told in.
#[derive(Resource, Default)]
struct NextSync {
    on: bool,
    held: Vec<(u64, Entity, Vec<Bytes>)>,
}

/// Runs `act`, holding what it tells at the sync to the next tick's.
pub(crate) fn synced_next_tick<R>(world: &mut World, act: impl FnOnce(&mut World) -> R) -> R {
    world.get_resource_or_insert_with(NextSync::default).on = true;
    let done = act(world);
    world.resource_mut::<NextSync>().on = false;
    done
}

/// The tick whose sync tells what is told now: the next while [`synced_next_tick`] runs.
pub(crate) fn sync_tick(world: &World) -> u64 {
    let deferred = world.get_resource::<NextSync>().is_some_and(|next| next.on);
    world.resource::<Tick>().0 + u64::from(deferred)
}

/// Runs `act`, holding what it tells players until this tick's held moves are sent.
pub(crate) fn told_after_answer<R>(world: &mut World, act: impl FnOnce(&mut World) -> R) -> R {
    let before: Vec<(Entity, usize)> = world.query::<(Entity, &Outgoing)>().iter(world).map(|(player, out)| (player, out.0.len())).collect();
    let done = act(world);
    let mut held = Vec::new();
    for (player, len) in before {
        if let Some(mut out) = world.get_mut::<Outgoing>(player).filter(|out| out.0.len() > len) {
            held.push((player, last_data(out.0.split_off(len))));
        }
    }
    let now = world.resource::<Tick>().0;
    match world.get_resource_mut::<NextSync>().filter(|next| next.on) {
        Some(mut next) => next.held.extend(held.into_iter().map(|(player, packets)| (now, player, packets))),
        None => world.get_resource_or_insert_with(AfterAnswer::default).0.extend(held),
    }
    done
}

/// The held packets told before this tick, in front of this tick's.
fn due_from_last_tick(world: &mut World) -> Vec<(Entity, Vec<Bytes>)> {
    let now = world.resource::<Tick>().0;
    let Some(mut next) = world.get_resource_mut::<NextSync>() else { return Vec::new() };
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut next.held).into_iter().partition(|(when, ..)| *when < now);
    next.held = waiting;
    due.into_iter().map(|(_, player, packets)| (player, packets)).collect()
}

/// `packets` less each SetEntityData whose every item a later one of the same entity sets again: BDS
/// sent a player's raid omen swap as one VisibleMobEffects (bds-bc1event.txt).
fn last_data(packets: Vec<Bytes>) -> Vec<Bytes> {
    let data: Vec<Option<SetEntityData>> = packets.iter().map(|packet| acacia_proto::RawPacket::parse(packet.clone()).ok().and_then(|raw| raw.decode::<SetEntityData>().ok())).collect();
    let set_later = |index: usize, item: &acacia_proto::types::MetadataDictionaryItem, id: u64| {
        data[index + 1..].iter().flatten().any(|later| later.runtime_entity_id == id && later.metadata.iter().any(|again| again.key == item.key))
    };
    let stale = |index: usize| data[index].as_ref().is_some_and(|told| told.properties.ints.is_empty() && told.properties.floats.is_empty() && !told.metadata.is_empty() && told.metadata.iter().all(|item| set_later(index, item, told.runtime_entity_id)));
    packets.into_iter().enumerate().filter(|(index, _)| !stale(*index)).map(|(_, packet)| packet).collect()
}

/// Sends what is heard as new entities appear, their first data and what was held back for the
/// tick's announcements; runs after them (mobs.rs).
pub(crate) fn send_held_moves(world: &mut World) {
    let mut held: Vec<(Entity, Vec<Bytes>)> = Vec::new();
    let now_told = world.get_resource_mut::<AfterAnswer>().map(|mut held| std::mem::take(&mut held.0)).unwrap_or_default();
    for (player, packets) in due_from_last_tick(world).into_iter().chain(now_told) {
        match held.iter_mut().find(|(known, _)| *known == player) {
            Some((_, all)) => all.extend(packets),
            None => held.push((player, packets)),
        }
    }
    for (player, packets) in held {
        let about: Vec<(Option<u64>, Bytes)> = packets.into_iter().map(|packet| (about(&packet), packet)).collect();
        let packets = super::slots::in_sync_order(world, about);
        if let Some(mut out) = world.get_mut::<Outgoing>(player) {
            out.0.extend(packets);
        }
    }
    let heard = world.get_resource_mut::<OnAnnouncing>().map(|mut heard| std::mem::take(&mut heard.0)).unwrap_or_default();
    for (layer, cell, packets) in heard {
        view::to_viewers(world, layer, cell, &packets);
    }
    super::first_data::send(world);
    let Some(mut held) = world.get_resource_mut::<AfterAnnouncing>().map(|mut held| std::mem::take(&mut held.0)) else { return };
    keep_last_flags(&mut held);
    for (layer, cell, about, packets) in held {
        match about {
            None => view::to_viewers(world, layer, cell, &packets),
            Some(entity) => {
                let packets = unannounced(world, entity, packets);
                let packets = crate::world_notices::heard(world, layer, &packets).into_owned();
                for viewer in shown_to(world, entity, layer, cell) {
                    if let Some(mut out) = world.get_mut::<Outgoing>(viewer) {
                        out.0.extend_from_slice(&packets);
                    }
                }
            }
        }
    }
}

/// The runtime id of the actor a held packet is about.
fn about(packet: &Bytes) -> Option<u64> {
    let raw = acacia_proto::RawPacket::parse(packet.clone()).ok()?;
    let sound = |raw: &acacia_proto::RawPacket| raw.decode::<acacia_proto::packets::LevelSoundEvent>().ok().and_then(|sound| u64::try_from(sound.entity_unique_id).ok());
    raw.decode::<SetEntityData>().ok().map(|data| data.runtime_entity_id).or_else(|| sound(&raw))
}

/// `packets` about `entity` less what its announcement this tick already showed (mobs/folded.rs).
fn unannounced(world: &World, entity: Entity, packets: Vec<Bytes>) -> Vec<Bytes> {
    let Some(shown) = crate::mobs::announced_linked(world, entity).filter(|_| is_new(world, entity)) else { return packets };
    packets.into_iter().filter_map(|packet| crate::mobs::unannounced(&shown, packet)).collect()
}

/// Leaves each new entity's last flag word of the tick and drops the ones before it: BDS told a
/// zombie's first BREATHING and fire in one SetEntityData (bds-firstdata.txt). Each word is whole.
fn keep_last_flags(held: &mut [(Entity, [i32; 3], Option<Entity>, Vec<Bytes>)]) {
    let mut told = std::collections::HashSet::new();
    for (_, _, about, packets) in held.iter_mut().rev() {
        let Some(entity) = *about else { continue };
        let mut kept: Vec<Bytes> = packets.drain(..).rev().filter(|packet| !is_flags_only(packet) || told.insert(entity)).collect();
        kept.reverse();
        *packets = kept;
    }
}

fn is_flags_only(packet: &Bytes) -> bool {
    let data = acacia_proto::RawPacket::parse(packet.clone()).ok().and_then(|raw| raw.decode::<SetEntityData>().ok());
    data.is_some_and(|data| matches!(&data.metadata[..], [item] if item.key == acacia_proto::types::MetadataDictionaryItemKey::Flags))
}

/// Hands `packets` to every player whose client has `entity`.
pub(super) fn to_seers(world: &mut World, entity: Entity, packets: &[Bytes]) {
    for (sees, mut out) in world.query::<(&SeesMobs, &mut Outgoing)>().iter_mut(world) {
        if sees.has(entity) {
            out.0.extend_from_slice(packets);
        }
    }
}

/// Whether `viewer`'s client has `entity`; a player (a rider told of) is no mob and always is.
fn sees(world: &World, viewer: Entity, entity: Entity) -> bool {
    world.get::<Mob>(entity).is_none() || world.get::<SeesMobs>(viewer).is_some_and(|sees| sees.has(entity))
}

/// The viewers of `cell` that `entity` is shown to.
fn shown_to(world: &mut World, entity: Entity, layer: Entity, cell: [i32; 3]) -> Vec<Entity> {
    view::viewers(world, layer, cell).into_iter().filter(|viewer| sees(world, *viewer, entity)).collect()
}

/// Sends `packets` about the entity to the viewers of `cell` it is shown to, after its
/// announcement when it appeared this tick.
pub(crate) fn tell_viewers(world: &mut World, entity: Entity, layer: Entity, cell: [i32; 3], packets: Vec<Bytes>) {
    if packets.is_empty() {
        return;
    }
    if is_new(world, entity) {
        if let Some(mut told) = world.get_mut::<Told>(entity) {
            told.settle(false);
        }
        after_announcing_of(world, entity, layer, cell, packets);
    } else {
        to_viewers_paced(world, entity, layer, cell, &packets);
    }
}

/// A player's eyes above its feet, whence its distance to a move is measured.
const EYES: f32 = 1.62;

/// Hands `packets` to the viewers of `cell` the entity is shown to, each told of its move as often
/// as its kind lets one so far away (sync/pace.rs), with that in the move's `ticks`. A teleport's
/// move goes to every viewer of `cell`, shown it or not, as BDS's `/tp` of a hidden mob did.
fn to_viewers_paced(world: &mut World, entity: Entity, layer: Entity, cell: [i32; 3], packets: &[Bytes]) {
    let packets = &*crate::world_notices::heard(world, layer, packets);
    let pending = world.get::<Told>(entity).and_then(|told| told.pending().cloned());
    let limits = world.get::<Kind>(entity).and_then(|kind| pace::limits(&kind.0));
    let now = world.resource::<Tick>().0;
    let mut versions: Vec<(u64, Bytes)> = pending.iter().map(|pending| (0, pending.bytes.clone())).collect();
    let mut none = true;
    for viewer in view::viewers(world, layer, cell) {
        let Some(pending) = pending.as_ref() else {
            if sees(world, viewer, entity)
                && let Some(mut out) = world.get_mut::<Outgoing>(viewer)
            {
                out.0.extend_from_slice(packets);
            }
            continue;
        };
        if !sees(world, viewer, entity) {
            if pending.delta.force_move
                && let Some(mut out) = world.get_mut::<Outgoing>(viewer)
            {
                out.0.push(pending.bytes.clone());
                none = false;
            }
            continue;
        }
        let eye = world.get::<Position>(viewer).map_or([f32::MAX; 3], |position| [position.feet[0], position.feet[1] + EYES, position.feet[2]]);
        let ticks = world.get_mut::<Told>(entity).and_then(|mut told| told.told_to(viewer, eye, now, limits));
        let delta = ticks.map(|ticks| match versions.iter().find(|(known, _)| *known == ticks) {
            Some((_, bytes)) => bytes.clone(),
            None => {
                let bytes = sapling_net::encode(&MoveEntityDelta { ticks, ..pending.delta.clone() });
                versions.push((ticks, bytes.clone()));
                bytes
            }
        });
        none &= delta.is_none();
        let told: Vec<Bytes> = packets.iter().filter_map(|packet| if *packet == pending.bytes { delta.clone() } else { Some(packet.clone()) }).collect();
        if let Some(mut out) = world.get_mut::<Outgoing>(viewer) {
            out.0.extend(told);
        }
    }
    if pending.is_some()
        && let Some(mut told) = world.get_mut::<Told>(entity)
    {
        told.settle(none);
    }
}
