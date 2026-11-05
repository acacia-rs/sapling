//! An entity moved into another world: it goes from the screens of its old world and is announced
//! in the new one under a new runtime id, keeping its unique id, as BDS shows a mob sent by
//! `/execute in … run tp` (research/captures/bds-u1inherit.txt).

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, AddItemEntity, MobArmorEquipment, MobEquipment, SetEntityData};
use acacia_proto::types::Vec3f;
use bevy_ecs::prelude::*;
use bytes::Bytes;
use sapling_world::ChunkPos;

use super::{Mob, MobGrid};
use crate::components::{InLayer, NetId, Players};

/// Moves the entity into the world `layer` at `feet` under a new runtime id, which it returns.
pub(crate) fn rehome(world: &mut World, mob: Entity, layer: Entity, feet: [f32; 3]) -> Option<u64> {
    let home = ChunkPos::of_block(feet[0].floor() as i32, feet[2].floor() as i32);
    let (old, id, from) = world.query::<(&Mob, &NetId, &InLayer)>().get(world, mob).ok().map(|(mob, id, layer)| (mob.home, id.0, layer.0))?;
    let new_id = world.resource::<Players>().next_net_id;
    world.resource_mut::<Players>().next_net_id += 1;
    world.entity_mut(mob).insert((InLayer(layer), NetId(new_id)));
    if let Some(mut shown) = world.get_mut::<Mob>(mob) {
        shown.home = home;
        shown.announcement = shown.announcement.iter().map(|packet| renumbered(packet, Some(new_id), Some(feet))).collect();
        shown.after = shown.after.iter().map(|packet| renumbered(packet, Some(new_id), None)).collect();
    }
    let mut grid = world.resource_mut::<MobGrid>();
    grid.grid.remove(from, old, &(mob, id));
    grid.grid.insert(layer, home, (mob, new_id));
    grid.changes += 1;
    Some(new_id)
}

/// An entity's or item's announcement with its place moved to `feet`; others as they are.
pub(super) fn placed(first: &Bytes, feet: [f32; 3]) -> Bytes {
    renumbered(first, None, Some(feet))
}

/// A packet of an entity's announcement under the runtime id `id` where given, its place moved to `feet`.
fn renumbered(packet: &Bytes, id: Option<u64>, feet: Option<[f32; 3]>) -> Bytes {
    let Ok(raw) = RawPacket::parse(packet.clone()) else { return packet.clone() };
    let at = |was: Vec3f| feet.map_or(was, |[x, y, z]| Vec3f { x, y, z });
    if let Ok(added) = raw.decode::<AddEntity>() {
        return sapling_net::encode(&AddEntity { runtime_id: id.unwrap_or(added.runtime_id), position: at(added.position), ..added });
    }
    if let Ok(added) = raw.decode::<AddItemEntity>() {
        return sapling_net::encode(&AddItemEntity { runtime_entity_id: id.unwrap_or(added.runtime_entity_id), position: at(added.position), ..added });
    }
    let Some(id) = id else { return packet.clone() };
    if let Ok(data) = raw.decode::<SetEntityData>() {
        return sapling_net::encode(&SetEntityData { runtime_entity_id: id, ..data });
    }
    if let Ok(held) = raw.decode::<MobEquipment>() {
        return sapling_net::encode(&MobEquipment { runtime_entity_id: id, ..held });
    }
    match raw.decode::<MobArmorEquipment>() {
        Ok(worn) => sapling_net::encode(&MobArmorEquipment { runtime_entity_id: id, ..worn }),
        Err(_) => packet.clone(),
    }
}
