//! What BDS shows of a `/summon`: the summoner's `EntitySpawn` event, to it and those who see it,
//! with the kind's numeric id (research/captures/bds-kill.txt: a chicken 655362, 10 << 16 | 2).

use std::collections::HashMap;
use std::sync::OnceLock;

use acacia_proto::packets::{EntityEvent, EntityEventEventId};
use acacia_proto::types::LegacyEntityType;
use bevy_ecs::prelude::*;

use crate::components::NetId;

/// The low bits BDS sent beside the id.
const SUMMONED: i32 = 2;

/// BDS's numeric id of an entity kind, by the protocol's names for them (`minecraft:armor_stand`: 61).
fn legacy_id(kind: &str) -> Option<i64> {
    static IDS: OnceLock<HashMap<String, i64>> = OnceLock::new();
    let ids = IDS.get_or_init(|| {
        let named = |raw: i64| Some(format!("{:?}", LegacyEntityType::from_raw(raw))).filter(|name| !name.starts_with("Unknown"));
        (0..1024).filter_map(|raw| named(raw).map(|name| (snake(&name), raw))).collect()
    });
    ids.get(kind.strip_prefix("minecraft:").unwrap_or(kind)).copied()
}

/// `ArmorStand` as `armor_stand`.
fn snake(camel: &str) -> String {
    let mut out = String::new();
    for (at, c) in camel.chars().enumerate() {
        if c.is_ascii_uppercase() && at > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// Shows `player` and its viewers that it summoned a `kind`; nothing for a kind with no id.
pub(super) fn show(world: &mut World, player: Entity, kind: &str) {
    let (Some(id), Some(of)) = (world.get::<NetId>(player).map(|id| id.0), legacy_id(kind)) else { return };
    let event = EntityEvent { runtime_entity_id: id, event_id: EntityEventEventId::EntitySpawn, data: ((of as i32) << 16) | SUMMONED, fire_at_position: None };
    crate::vitals::tell(world, player, &event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_take_the_ids_bds_sent() {
        assert_eq!([legacy_id("minecraft:chicken"), legacy_id("minecraft:pig"), legacy_id("minecraft:armor_stand"), legacy_id("minecraft:no_such")], [Some(10), Some(12), Some(61), None]);
    }
}
