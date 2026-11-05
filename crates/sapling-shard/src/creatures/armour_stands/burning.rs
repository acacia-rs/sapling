//! An armour stand in fire or lava, as BDS showed one (research/captures/bds-n3breath.txt,
//! bds-e1stand.txt, bds-o3stand.txt, bds-o3stand2.txt): it flinches (`HurtAnimation` -1) and loses
//! 0.05 health on its first tick there and every 10 ticks after, and is shown alight from the tick
//! after. Taken out, it burns on 160 ticks: at each 20th but the last it flinches and loses 1, its
//! health told the tick after; at none it is gone, unflinched, that tick after, dropping its gear.
//! What it wears wards off each loss as a mob's armour does.

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use acacia_proto::types::MetadataFlags1 as Flags;
use bevy_ecs::prelude::*;

use super::hits::drop_gear;
use super::{HEAD, STAND, gear_of};
use crate::body::Body;
use crate::creatures::health::armour_stops;
use crate::components::{InLayer, Layer, NetId};
use crate::entities::{self, Kind};
use crate::mobs;

const FIERY: &[&str] = &["minecraft:lava", "minecraft:flowing_lava", "minecraft:fire", "minecraft:soul_fire"];
const EVERY: u32 = 10;
const SCORCHES: f32 = 0.05;
const BURNS_EVERY: u32 = 20;
const BURNS: f32 = 1.0;
const BURNS_FOR: u32 = 160;
const HEALTH: f32 = 6.0;
/// The data of each flinch.
const UNNAMED_CAUSE: i32 = -1;

/// A stand alight: the ticks it has been in fire (none once out), the ticks it burns on once out,
/// and its health.
#[derive(Component)]
pub(super) struct Burning {
    inside: Option<u32>,
    left: u32,
    health: f32,
    lit: bool,
}

pub(super) fn tick(world: &mut World) {
    let stands: Vec<(Entity, u64, Entity, Body)> = world.query::<(Entity, &Kind, &NetId, &InLayer, &Body)>().iter(world).filter(|(_, kind, ..)| kind.0 == STAND).map(|(stand, _, id, layer, body)| (stand, id.0, layer.0, *body)).collect();
    for (stand, id, layer, body) in stands {
        let fiery = world.get::<Layer>(layer).is_some_and(|level| crate::creatures::touches(&level.world, &body, FIERY));
        let Some(mut burning) = world.entity_mut(stand).take::<Burning>().or(fiery.then_some(Burning { inside: None, left: 0, health: HEALTH, lit: false })) else { continue };
        let mut told = Vec::new();
        let armour = armour(world, stand);
        let taken = |by: f32| by * (1.0 - armour_stops(by, armour));
        let hurt = |told: &mut Vec<_>, burning: &mut Burning, by: f32| {
            told.push(sapling_net::encode(&EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::HurtAnimation, data: UNNAMED_CAUSE, fire_at_position: None }));
            burning.health -= taken(by);
        };
        let health = |burning: &Burning| sapling_net::encode(&sapling_data::mob_health_update(id, burning.health.max(0.0), HEALTH));
        if fiery {
            let inside = burning.inside.map_or(0, |ticks| ticks + 1);
            if inside % EVERY == 0 {
                hurt(&mut told, &mut burning, SCORCHES);
                told.push(health(&burning));
            }
            if inside == 1 && !burning.lit {
                burning.lit = true;
                told.push(flags(world, stand, id, true));
            }
            (burning.inside, burning.left) = (Some(inside), BURNS_FOR);
        } else if burning.left > 0 {
            burning.inside = None;
            burning.left -= 1;
            let out = BURNS_FOR - burning.left;
            match out % BURNS_EVERY {
                0 if burning.left == 0 => {}
                0 if burning.health > taken(BURNS) => hurt(&mut told, &mut burning, BURNS),
                0 => burning.health -= taken(BURNS),
                1 if out > 1 && burning.health <= 0.0 => {
                    burn_up(world, stand, layer, body.at);
                    continue;
                }
                1 if out > 1 => told.push(health(&burning)),
                _ => {}
            }
            if burning.left == 0 && burning.lit {
                burning.lit = false;
                told.push(flags(world, stand, id, false));
            }
        }
        if burning.health <= 0.0 && fiery {
            burn_up(world, stand, layer, body.at);
            continue;
        }
        world.entity_mut(stand).insert(burning);
        if !told.is_empty() {
            entities::tell_viewers(world, stand, layer, body.at.map(|c| c.floor() as i32), told);
        }
    }
}

/// The points and toughness of what it wears, which a burn goes through as a mob's hit does.
fn armour(world: &World, stand: Entity) -> [f32; 2] {
    gear_of(world, stand).slots[HEAD..].iter().flatten().filter_map(|piece| sapling_data::armour(&piece.name)).fold([0.0; 2], |[points, toughness], [more, tougher]| [points + more, toughness + tougher])
}

/// Gone, its gear dropped and not itself (research/captures/bds-o3stand2.txt).
fn burn_up(world: &mut World, stand: Entity, layer: Entity, at: [f32; 3]) {
    drop_gear(world, stand, layer, at);
    mobs::remove(world, stand);
}

fn flags(world: &mut World, stand: Entity, id: u64, on: bool) -> bytes::Bytes {
    let word = entities::mob_flags(world, stand, STAND, Flags::ONFIRE, on);
    sapling_net::encode(&sapling_data::mob_flags_set(id, word))
}
