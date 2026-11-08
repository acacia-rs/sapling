//! A grown happy ghast's `entity_sensor` (its pack's `adult` group): a player riding no happy ghast within 3
//! across and 1.5 up or down of the point 4.5 over its feet stills it (`become_immobile`: `can_move` 0,
//! `movement` and `flying_speed` 0, its motion stopped, `FlagsExtended` bits 54, 56 and 57); none within 3.5
//! and 2 frees it (`become_mobile`). BDS stilled one the tick after a player was let off onto its top, freed it
//! the tick after the player went, and told each as one SetEntityData of the flags and the property, then an
//! UpdateAttributes of two `minecraft:movement`s (research/captures/bds-as3ghast.txt).
// TODO: a player's position is taken at its eyes, as BDS keeps it; whether the sensor measures that or the
// box was not drilled. The movement attributes carry no slowness modifier (BDS's did under slowness).

use acacia_proto::packets::UpdateAttributes;
use acacia_proto::types::{MetadataDictionaryItem as Item, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataFlags2, PlayerAttributesItem};
use bevy_ecs::prelude::*;
use sapling_api::GameMode;

use super::harness::KIND;
use crate::body::Body;
use crate::components::{InLayer, Mode, NetId, Position, in_world};
use crate::entities::Kind;

/// The extended flags it gains while still.
pub(super) const STILL: i64 = 1 << 54 | 1 << 56 | 1 << 57;
const CAN_MOVE: &str = "minecraft:can_move";
const SENSOR_UP: f32 = 4.5;
/// Across and up or down: within these a player stills it, past these it is freed.
const STILLS: [f32; 2] = [3.0, 1.5];
const FREES: [f32; 2] = [3.5, 2.0];
const EYES: f32 = 1.62;
/// Its `movement` and `flying_speed` free (`adult_mobile`).
const MOVES: f32 = 0.016;

/// A happy ghast a player stands on.
#[derive(Component)]
pub(super) struct Stilled;

pub(super) fn is_stilled(world: &World, mob: Entity) -> bool {
    world.get::<Stilled>(mob).is_some()
}

/// The players' eyes in `layer` its sensor may count: alive, not spectating and riding no happy ghast.
fn sensed(world: &World) -> Vec<(Entity, [f32; 3])> {
    let rides_ghast = |player: Entity| world.get::<super::Riding>(player).is_some_and(|riding| world.get::<Kind>(riding.vehicle).is_some_and(|kind| kind.0 == KIND));
    in_world(world)
        .into_iter()
        .filter(|player| world.get::<Mode>(*player).is_none_or(|mode| mode.0 != GameMode::Spectator) && !rides_ghast(*player) && !crate::vitals::is_dead(world, *player))
        .filter_map(|player| Some((world.get::<InLayer>(player)?.0, world.get::<Position>(player)?.feet)))
        .map(|(layer, [x, y, z])| (layer, [x, y + EYES, z]))
        .collect()
}

/// Each grown happy ghast's sensor this tick: stills or frees those it counts players by.
pub(super) fn sense(world: &mut World) {
    let ghasts: Vec<(Entity, Entity, [f32; 3])> =
        world.query::<(Entity, &Kind, &InLayer, &Body)>().iter(world).filter(|(_, kind, ..)| kind.0 == KIND).map(|(mob, _, layer, body)| (mob, layer.0, body.at)).collect();
    if ghasts.is_empty() {
        return;
    }
    let players = sensed(world);
    for (mob, layer, [x, y, z]) in ghasts {
        if crate::creatures::is_baby(world, mob) || crate::creatures::is_dying(world, mob) {
            continue;
        }
        let within = |[across, up]: [f32; 2]| players.iter().any(|(on, at)| *on == layer && (at[0] - x).abs() <= across && (at[2] - z).abs() <= across && (at[1] - y - SENSOR_UP).abs() <= up);
        match is_stilled(world, mob) {
            false if within(STILLS) => still(world, mob, true),
            true if !within(FREES) => still(world, mob, false),
            _ => {}
        }
    }
}

fn movements(id: u64, speed: f32) -> UpdateAttributes {
    let movement = PlayerAttributesItem { min: 0.0, max: f32::MAX, current: speed, default_min: 0.0, default_max: f32::MAX, default: speed, name: "minecraft:movement".into(), modifiers: Vec::new() };
    UpdateAttributes { runtime_entity_id: id, attributes: vec![movement.clone(), movement], tick: 0 }
}

fn still(world: &mut World, mob: Entity, stilled: bool) {
    match stilled {
        true => {
            world.entity_mut(mob).insert(Stilled);
            if let Some(mut body) = world.get_mut::<Body>(mob) {
                body.motion = [0.0; 3];
            }
        }
        false => {
            world.entity_mut(mob).remove::<Stilled>();
        }
    }
    let flags = Item { key: Key::FlagsExtended, r#type: Type::Long, legacy_type: 7, value: Value::FlagsExtended(MetadataFlags2(super::harness::extended(world, mob))) };
    crate::creatures::set_property(world, mob, CAN_MOVE, i32::from(!stilled), vec![flags.clone()]);
    crate::mobs::reannounce_item(world, mob, flags);
    let Some(id) = world.get::<NetId>(mob).map(|id| id.0) else { return };
    let speed = if stilled { 0.0 } else { MOVES };
    super::tell(world, mob, mob, vec![sapling_net::encode(&movements(id, speed))]);
}
