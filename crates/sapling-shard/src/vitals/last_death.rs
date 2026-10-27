//! Where a player last died, which its recovery compass points to: told as BDS tells it, in the
//! data a death shrinks the body by, and kept with its items as BDS keeps it
//! (research/captures/bds-anchor.txt; `DeathPositionX` and the rest, BDSRE: Player load 1402af220).

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain, Vec3i};
use bevy_ecs::prelude::*;
use sapling_data::EYE_HEIGHT;

/// The cell of a player's eyes as it died, and the dimension it died in.
#[derive(Component, Clone, Copy, PartialEq, Debug)]
pub(crate) struct LastDeath {
    pub(crate) at: [i32; 3],
    pub(crate) dimension: i32,
}

impl LastDeath {
    /// A death with the feet at `feet`: BDS names the cell its eyes are in.
    pub(crate) fn at_feet([x, y, z]: [f32; 3], dimension: i32) -> Self {
        Self { at: [x, y + EYE_HEIGHT, z].map(|c| c.floor() as i32), dimension }
    }
}

fn item(key: Key, r#type: Type, legacy_type: u8, value: Plain) -> MetadataDictionaryItem {
    MetadataDictionaryItem { key, r#type, legacy_type, value: Value::Default(value) }
}

fn place([x, y, z]: [i32; 3]) -> MetadataDictionaryItem {
    item(Key::PlayerLastDeathPosition, Type::Vec3i, 6, Plain::Vec3i(Vec3i { x, y, z }))
}

fn dimension(dimension: i32) -> MetadataDictionaryItem {
    item(Key::PlayerLastDeathDimension, Type::Int, 2, Plain::Int(dimension))
}

fn has_died() -> MetadataDictionaryItem {
    item(Key::PlayerHasDied, Type::Byte, 0, Plain::Byte(1))
}

/// What a death tells the player and those who see it: the body shrunk, then of `now` what differs
/// from `before`, and that it has died if it never had (BDS sends only the fields that changed).
pub(crate) fn told(id: u64, before: Option<LastDeath>, now: LastDeath, small: [f32; 2], tick: u64) -> SetEntityData {
    let size = [(Key::BoundingboxWidth, small[0]), (Key::BoundingboxHeight, small[1])].map(|(key, size)| item(key, Type::Float, 3, Plain::Float(size)));
    let mut metadata = size.to_vec();
    if before.is_none_or(|before| before.at != now.at) {
        metadata.push(place(now.at));
    }
    if before.is_none_or(|before| before.dimension != now.dimension) && !(before.is_none() && now.dimension == 0) {
        metadata.push(dimension(now.dimension));
    }
    if before.is_none() {
        metadata.push(has_died());
    }
    SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick }
}

/// What tells a joining player where it last died, kept from an earlier visit.
pub(crate) fn greeting(id: u64, kept: LastDeath, tick: u64) -> SetEntityData {
    SetEntityData { runtime_entity_id: id, metadata: vec![place(kept.at), dimension(kept.dimension), has_died()], properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(told: &SetEntityData) -> Vec<Key> {
        told.metadata.iter().map(|item| item.key).collect()
    }

    #[test]
    fn a_death_tells_what_changed_as_bds_told_three_in_a_row() {
        let small = [0.2, 0.2];
        let nether = LastDeath::at_feet([-1.5, 100.0, 2.5], 1);
        assert_eq!(nether.at, [-2, 101, 2]);
        let size = [Key::BoundingboxWidth, Key::BoundingboxHeight];
        let first = told(1, None, nether, small, 0);
        assert_eq!(keys(&first), [&size[..], &[Key::PlayerLastDeathPosition, Key::PlayerLastDeathDimension, Key::PlayerHasDied]].concat());
        let moved = LastDeath::at_feet([0.5, 100.0, 0.5], 1);
        assert_eq!(keys(&told(1, Some(nether), moved, small, 0)), [&size[..], &[Key::PlayerLastDeathPosition]].concat());
        assert_eq!(keys(&told(1, Some(moved), moved, small, 0)), size);
        let home = LastDeath::at_feet([-7.5, -60.0, 0.5], 0);
        assert_eq!((home.at, keys(&told(1, Some(moved), home, small, 0))), ([-8, -59, 0], [&size[..], &[Key::PlayerLastDeathPosition, Key::PlayerLastDeathDimension]].concat()));
    }
}
