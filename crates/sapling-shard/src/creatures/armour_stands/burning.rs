//! An armour stand in fire or lava, as BDS showed one (research/captures/bds-n3breath.txt): it
//! flinches (`HurtAnimation` -1) on its first tick there and every 10 ticks after, and is shown
//! alight from the tick after; it loses nothing and never dies of it.
// TODO: when BDS puts out a stand taken from the fire was not drilled; here it goes out at once.

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use acacia_proto::types::MetadataFlags1 as Flags;
use bevy_ecs::prelude::*;

use super::STAND;
use crate::body::Body;
use crate::components::{InLayer, Layer, NetId};
use crate::entities::{self, Kind};

const FIERY: &[&str] = &["minecraft:lava", "minecraft:flowing_lava", "minecraft:fire", "minecraft:soul_fire"];
const EVERY: u32 = 10;
/// The data of each flinch.
const UNNAMED_CAUSE: i32 = -1;

/// A stand in fire: how many ticks it has been there.
#[derive(Component)]
pub(super) struct Burning(u32);

pub(super) fn tick(world: &mut World) {
    let stands: Vec<(Entity, u64, Entity, Body)> = world.query::<(Entity, &Kind, &NetId, &InLayer, &Body)>().iter(world).filter(|(_, kind, ..)| kind.0 == STAND).map(|(stand, _, id, layer, body)| (stand, id.0, layer.0, *body)).collect();
    for (stand, id, layer, body) in stands {
        let fiery = world.get::<Layer>(layer).is_some_and(|level| crate::creatures::touches(&level.world, &body, FIERY));
        let ticks = world.get::<Burning>(stand).map(|burning| burning.0);
        let cell = body.at.map(|c| c.floor() as i32);
        let mut told = Vec::new();
        match (fiery, ticks) {
            (false, None) => continue,
            (false, Some(_)) => {
                world.entity_mut(stand).remove::<Burning>();
                told.push(flags(world, stand, id, false));
            }
            (true, ticks) => {
                let ticks = ticks.map_or(0, |ticks| ticks + 1);
                world.entity_mut(stand).insert(Burning(ticks));
                if ticks % EVERY == 0 {
                    told.push(sapling_net::encode(&EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::HurtAnimation, data: UNNAMED_CAUSE, fire_at_position: None }));
                }
                if ticks == 1 {
                    told.push(flags(world, stand, id, true));
                }
            }
        }
        if !told.is_empty() {
            entities::tell_viewers(world, stand, layer, cell, told);
        }
    }
}

fn flags(world: &mut World, stand: Entity, id: u64, on: bool) -> bytes::Bytes {
    let word = entities::mob_flags(world, stand, STAND, Flags::ONFIRE, on);
    sapling_net::encode(&sapling_data::mob_flags_set(id, word))
}
