//! What tells viewers of a mob's health and of its being on fire, in BDS's shapes
//! (research/captures/bds-burn.txt), and which kinds fire does nothing to (`data/fire-immune.txt`,
//! README "Fire immune").

use acacia_proto::packets::{SetEntityData, UpdateAttributes};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataFlags1, PlayerAttributesItem};

use crate::entity::mob_flags;

const HEALTH: &str = "minecraft:health";

/// Whether fire and lava leave a mob of `identifier` alone.
pub fn is_fire_immune(identifier: &str) -> bool {
    include_str!("../data/fire-immune.txt").lines().any(|immune| immune == identifier)
}

/// The mob `id` has `left` of `max` health.
pub fn mob_health_update(id: u64, left: f32, max: f32) -> UpdateAttributes {
    let health = PlayerAttributesItem { min: 0.0, max, current: left, default_min: 0.0, default_max: max, default: max, name: HEALTH.to_owned(), modifiers: Vec::new() };
    UpdateAttributes { runtime_entity_id: id, attributes: vec![health], tick: 0 }
}

/// The flags of the mob `id`, a grown one of `identifier`, on fire or not.
pub fn mob_fire_flags(id: u64, identifier: &str, on_fire: bool) -> SetEntityData {
    let flags = MetadataFlags1(mob_flags(identifier).0 | if on_fire { MetadataFlags1::ONFIRE.0 } else { 0 });
    let item = MetadataDictionaryItem { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(flags) };
    SetEntityData { runtime_entity_id: id, metadata: vec![item], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_strider_is_fire_immune_and_a_pig_is_not() {
        assert!(is_fire_immune("minecraft:strider") && !is_fire_immune("minecraft:pig"));
    }

    #[test]
    fn a_burning_pig_keeps_its_own_flags() {
        let flags = |on_fire| match mob_fire_flags(7, "minecraft:pig", on_fire).metadata[0].value {
            Value::Flags(MetadataFlags1(flags)) => flags,
            _ => unreachable!(),
        };
        assert_eq!(flags(true), flags(false) | 1);
        assert_ne!(flags(false), 0);
        assert_eq!(mob_health_update(7, 3.0, 10.0).attributes[0].current, 3.0);
    }
}
