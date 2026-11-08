//! Every attribute of a player in one UpdateAttributes, in BDS's order, as BDS told the player
//! and its viewers when it started eating (research/captures/bds-t2eitems.txt); and all its data.

use acacia_proto::packets::{SetEntityData, UpdateAttributes};
use acacia_proto::types::{MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataFlags2, PlayerAttributesItem};
use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::{MAX, Vitals, boost, effects};
use crate::components::{Identity, NetId, Tick};
use crate::movement::Motion;

/// The ones no state of the shard's moves: name, least, most and value (bds-t2eitems.txt).
const STILL: [(&str, f32, f32, f32); 6] = [
    ("minecraft:underwater_movement", 0.0, f32::MAX, 0.02),
    ("minecraft:lava_movement", 0.0, f32::MAX, 0.02),
    ("minecraft:luck", -1024.0, 1024.0, 0.0),
    ("minecraft:friction_modifier", 0.0, 256.0, 1.0),
    ("minecraft:bounciness", 0.0, 1.0, 0.0),
    ("minecraft:air_drag_modifier", 0.0, 256.0, 1.0),
];
const ORDER: [&str; 14] = [
    "minecraft:health", "minecraft:movement", "minecraft:underwater_movement", "minecraft:lava_movement", "minecraft:absorption", "minecraft:luck", "minecraft:friction_modifier",
    "minecraft:bounciness", "minecraft:air_drag_modifier", "minecraft:player.hunger", "minecraft:player.saturation", "minecraft:player.exhaustion", "minecraft:player.level",
    "minecraft:player.experience",
];

/// The player's every attribute now; `None` for one without vitals.
pub(crate) fn all_attributes(world: &World, player: Entity) -> Option<UpdateAttributes> {
    let (id, tick, vitals) = (world.get::<NetId>(player)?, world.resource::<Tick>().0, world.get::<Vitals>(player)?);
    let still = STILL.map(|(name, min, max, value)| PlayerAttributesItem { min, max, current: value, default_min: min, default_max: max, default: value, name: name.into(), modifiers: Vec::new() });
    let mut known: Vec<PlayerAttributesItem> = boost::boosted_health(id.0, vitals.health.left(), MAX, effects::level(world, player, Effect::HealthBoost)).attributes;
    known.push(effects::walking(world, player, id.0));
    known.extend(effects::cushion(id.0, vitals.health.cushion(), tick).attributes);
    known.extend(vitals.hunger.told(id.0, tick).attributes);
    if let Some(inventory) = world.get::<crate::inventory::Inventory>(player) {
        known.extend(crate::experience::attributes(id, &inventory.experience, true, tick).attributes);
    }
    known.extend(still);
    let attributes = ORDER.iter().filter_map(|name| known.iter().find(|known| known.name == *name).cloned()).collect();
    Some(UpdateAttributes { runtime_entity_id: id.0, attributes, tick })
}

/// The player's whole data now, as BDS told it and its viewers at a throw (research/captures/
/// bds-bg2player.txt, bds-small.txt), stamped with its input's tick.
pub(crate) fn whole_data(world: &World, player: Entity) -> Option<SetEntityData> {
    let (id, motion) = (world.get::<NetId>(player)?.0, world.get::<Motion>(player)?);
    let name = world.get::<Identity>(player).map_or("", |identity| identity.0.name.as_str());
    let mut data = crate::movement::whole_packet(id, name, motion.pose, motion.input_tick());
    live_data(id, world.get::<effects::Effects>(player), world.get::<super::Guard>(player), &mut data.metadata);
    Some(data)
}

/// Writes what the player `id` shows now of its effects and its shield into its recorded `metadata`.
pub(crate) fn live_data(id: u64, effects: Option<&effects::Effects>, guard: Option<&super::Guard>, metadata: &mut [MetadataDictionaryItem]) {
    for item in metadata {
        match item.key {
            Key::VisibleMobEffects => *item = effects::announced(id, effects),
            Key::FlagsExtended => item.value = Value::FlagsExtended(MetadataFlags2(sapling_data::player_flags_extended().0 | super::shown_bits(guard))),
            _ => {}
        }
    }
}
