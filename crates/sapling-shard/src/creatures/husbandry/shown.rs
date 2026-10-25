//! What a raised mob's viewers are told: its flags, events (hearts, a taming's smoke or hearts, a
//! sheep's grazing), sounds, growing up, wool, name, and who owns it and holds its lead.

use acacia_proto::packets::{EntityEvent, EntityEventEventId, LevelSoundEvent};
use acacia_proto::types::{MetadataFlags1 as Flags, Vec3f};
use bevy_ecs::prelude::*;
use bytes::Bytes;

use super::Raised;
use crate::body::Body;
use crate::components::{InLayer, NetId, in_world};
use crate::entities::{self, Kind};
use crate::sync::Told;
use crate::tags::Marks;

/// A mob's layer, runtime id, kind and body.
pub(super) fn of(world: &World, entity: Entity) -> Option<(Entity, u64, String, Body)> {
    Some((world.get::<InLayer>(entity)?.0, world.get::<NetId>(entity)?.0, world.get::<Kind>(entity)?.0.clone(), *world.get::<Body>(entity)?))
}

/// Sends `packets` to the mob's viewers.
pub(super) fn tell(world: &mut World, entity: Entity, packets: Vec<Bytes>) {
    let Some((layer, _, _, body)) = of(world, entity) else { return };
    entities::tell_viewers(world, entity, layer, body.at.map(|c| c.floor() as i32), packets);
}

/// The whole word of the mob's flags with `flag` set or cleared.
pub(super) fn flags_with(world: &mut World, entity: Entity, flag: Flags, on: bool) -> Flags {
    let kind = world.get::<Kind>(entity).map(|kind| kind.0.clone()).unwrap_or_default();
    if let Some(mut told) = world.get_mut::<Told>(entity).filter(|_| !on) {
        told.forget_announced(flag);
    }
    entities::mob_flags(world, entity, &kind, flag, on)
}

/// Sets or clears `flag` and tells the mob's viewers.
pub(super) fn flag(world: &mut World, entity: Entity, flag: Flags, on: bool) {
    let Some(id) = world.get::<NetId>(entity).map(|id| id.0) else { return };
    let now = flags_with(world, entity, flag, on);
    tell(world, entity, vec![sapling_net::encode(&sapling_data::mob_flags_set(id, now))]);
}

pub(super) fn event(world: &mut World, entity: Entity, event_id: EntityEventEventId, data: i32) {
    let Some(id) = world.get::<NetId>(entity).map(|id| id.0) else { return };
    tell(world, entity, vec![sapling_net::encode(&EntityEvent { runtime_entity_id: id, event_id, data, fire_at_position: None })]);
}

/// The mob's sound `name`, from the middle of its box.
pub(super) fn sound(world: &mut World, entity: Entity, name: &str) {
    let Some((_, id, kind, body)) = of(world, entity) else { return };
    let is_baby = world.get::<Raised>(entity).is_some_and(Raised::is_baby);
    let position = Vec3f { x: body.at[0], y: body.at[1] + body.size[1] / 2.0, z: body.at[2] };
    let sound = LevelSoundEvent { sound_id: name.into(), position, extra_data: -1, entity_type: kind, is_baby_mob: is_baby, is_global: false, entity_unique_id: id as i64, fire_at_position: None };
    tell(world, entity, vec![sapling_net::encode(&sound)]);
}

/// The mob grew up or was born: its flags, scale and box.
pub(super) fn age(world: &mut World, entity: Entity) {
    let Some((_, id, kind, _)) = of(world, entity) else { return };
    let baby = world.get::<Raised>(entity).is_some_and(Raised::is_baby);
    let Some((size, scale)) = sapling_data::mob_look(&kind, baby) else { return };
    let now = flags_with(world, entity, Flags::BABY, baby);
    tell(world, entity, vec![sapling_net::encode(&sapling_data::mob_age_set(id, now, scale, size))]);
}

/// The mob's wool changed colour, or came off or back.
pub(super) fn wool(world: &mut World, entity: Entity) {
    let Some((id, raised)) = world.get::<NetId>(entity).map(|id| id.0).zip(world.get::<Raised>(entity).map(|raised| (raised.kept.sheared, raised.kept.colour))) else { return };
    let now = flags_with(world, entity, Flags::SHEARED, raised.0);
    tell(world, entity, vec![sapling_net::encode(&sapling_data::mob_colour_set(id, now, raised.1))]);
}

pub(super) fn name(world: &mut World, entity: Entity) {
    let Some((id, name)) = world.get::<NetId>(entity).map(|id| id.0).zip(world.get::<Raised>(entity).map(|raised| raised.kept.name.clone())) else { return };
    tell(world, entity, vec![sapling_net::encode(&sapling_data::mob_name_set(id, &name))]);
}

/// The player whose `UniqueID` is `unique_id`, if it is in the world.
pub(super) fn player_of(world: &World, unique_id: i64) -> Option<Entity> {
    in_world(world).into_iter().find(|player| world.get::<Marks>(*player).is_some_and(|marks| marks.unique_id == unique_id))
}

/// Who owns the mob and holds its lead, by runtime id, for its viewers; `None` where neither is anyone.
pub(crate) fn links_packet(world: &World, entity: Entity, id: u64, flags: Flags) -> Option<Bytes> {
    let raised = world.get::<Raised>(entity)?;
    let runtime = |of: Option<Entity>| of.and_then(|of| world.get::<NetId>(of)).map_or(-1, |id| id.0 as i64);
    let owner = runtime(raised.kept.owner.and_then(|owner| player_of(world, owner)));
    let holder = runtime(raised.holder);
    (owner != -1 || holder != -1).then(|| sapling_net::encode(&sapling_data::mob_links_set(id, flags, owner, holder)))
}

/// Tells the mob's viewers who owns it and holds its lead now, with its `LEASHED` flag.
pub(super) fn links(world: &mut World, entity: Entity) {
    let Some(id) = world.get::<NetId>(entity).map(|id| id.0) else { return };
    let leashed = world.get::<Raised>(entity).is_some_and(|raised| raised.kept.leasher.is_some());
    let now = flags_with(world, entity, Flags::LEASHED, leashed);
    let packet = links_packet(world, entity, id, now).unwrap_or_else(|| sapling_net::encode(&sapling_data::mob_links_set(id, now, -1, -1)));
    tell(world, entity, vec![packet]);
}
