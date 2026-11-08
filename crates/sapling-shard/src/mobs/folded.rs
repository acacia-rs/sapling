//! A mob's announcement carries what it is: BDS's `AddEntity` holds the values a new mob was given before
//! it was added (a zombie villager's profession, a baby's scale and box, a jockey's seat), and only what
//! changed in its first tick follows in a `SetEntityData` (research/captures/bds-ar2look.txt,
//! bds-f3convert.txt). The values sapling's code tells after building an announcement are folded into it.

use acacia_proto::RawPacket;
use acacia_proto::packets::{AddEntity, SetEntityData};
use acacia_proto::types::{EntityProperties, MetadataDictionaryItem, MetadataDictionaryItemKey as Key};
use bytes::Bytes;

fn none() -> EntityProperties {
    EntityProperties { ints: Vec::new(), floats: Vec::new() }
}

fn added(packet: &Bytes) -> Option<AddEntity> {
    RawPacket::parse(packet.clone()).ok()?.decode::<AddEntity>().ok()
}

fn data(packet: &Bytes) -> Option<SetEntityData> {
    RawPacket::parse(packet.clone()).ok()?.decode::<SetEntityData>().ok()
}

/// `announcement` with each `SetEntityData` of its own entity after the `AddEntity` put into it: items the
/// `AddEntity` has take its place; the rest stay in one `SetEntityData`, where the first of them was.
pub(super) fn fold(mut announcement: Vec<Bytes>) -> Vec<Bytes> {
    let Some(mut entity) = announcement.first().and_then(added) else { return announcement };
    let mut left: Vec<MetadataDictionaryItem> = Vec::new();
    let mut first = None;
    let mut index = 1;
    while index < announcement.len() {
        let Some(told) = data(&announcement[index]).filter(|told| told.runtime_entity_id == entity.runtime_id && told.properties.ints.is_empty() && told.properties.floats.is_empty()) else {
            index += 1;
            continue;
        };
        announcement.remove(index);
        first.get_or_insert(index);
        for item in told.metadata {
            match entity.metadata.iter_mut().find(|old| old.key == item.key) {
                Some(old) => *old = item,
                None => match left.iter_mut().find(|old| old.key == item.key) {
                    Some(old) => *old = item,
                    None => left.push(item),
                },
            }
        }
    }
    let Some(first) = first else { return announcement };
    announcement[0] = sapling_net::encode(&entity);
    if !left.is_empty() {
        let rest = SetEntityData { runtime_entity_id: entity.runtime_id, metadata: left, properties: none(), tick: 0 };
        announcement.insert(first, sapling_net::encode(&rest));
    }
    announcement
}

/// What of `packet`, told about a mob in the tick it appeared, its announcement does not already show:
/// a `SetEntityData`'s items equal to its `AddEntity`'s are left out (all of it when none is left), as BDS
/// sends no unchanged item after a new mob's `AddEntity`. Its flag word is kept (sync.rs tells it whole).
pub(crate) fn unannounced(announcement: &[Bytes], packet: Bytes) -> Option<Bytes> {
    let Some(mut told) = data(&packet) else { return Some(packet) };
    let Some(entity) = announcement.first().and_then(added).filter(|entity| entity.runtime_id == told.runtime_entity_id) else { return Some(packet) };
    let before = told.metadata.len();
    told.metadata.retain(|item| item.key == Key::Flags || !entity.metadata.contains(item));
    match told.metadata.len() {
        0 if told.properties.ints.is_empty() && told.properties.floats.is_empty() => None,
        kept if kept == before => Some(packet),
        _ => Some(sapling_net::encode(&told)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use acacia_proto::types::{MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};

    fn int(key: Key, value: i32) -> MetadataDictionaryItem {
        MetadataDictionaryItem { key, r#type: Type::Int, legacy_type: 2, value: Value::Default(Plain::Int(value)) }
    }

    fn told(id: u64, metadata: Vec<MetadataDictionaryItem>) -> Bytes {
        sapling_net::encode(&SetEntityData { runtime_entity_id: id, metadata, properties: none(), tick: 0 })
    }

    fn villager() -> Bytes {
        let options = sapling_data::AddEntityOptions {
            id: 7,
            identifier: "minecraft:villager_v2",
            feet: [0.5; 3],
            yaw: 0.0,
            pitch: 0.0,
            variant: 0,
            mark_variant: 0,
            color: 0,
            baby: false,
            flags: acacia_proto::types::MetadataFlags1(0),
            health: None,
            name: "",
        };
        sapling_net::encode(&sapling_data::add_entity(&options))
    }

    #[test]
    fn told_items_go_into_the_add_entity_and_unknown_ones_stay() {
        let folded = fold(vec![villager(), told(7, vec![int(Key::Variant, 8), int(Key::SkinId, 0)]), told(7, vec![int(Key::Variant, 3)]), told(9, vec![int(Key::Variant, 1)])]);
        assert_eq!(folded.len(), 2);
        let entity = added(&folded[0]).unwrap();
        assert!(entity.metadata.contains(&int(Key::Variant, 3)));
        assert!(entity.metadata.contains(&int(Key::SkinId, 0)));
        assert_eq!(data(&folded[1]).unwrap().runtime_entity_id, 9);
    }

    #[test]
    fn what_the_announcement_shows_is_not_told_again() {
        let folded = fold(vec![villager(), told(7, vec![int(Key::Variant, 8)])]);
        assert!(unannounced(&folded, told(7, vec![int(Key::Variant, 8)])).is_none());
        let changed = unannounced(&folded, told(7, vec![int(Key::Variant, 8), int(Key::SkinId, 4)])).unwrap();
        assert_eq!(data(&changed).unwrap().metadata, vec![int(Key::SkinId, 4)]);
    }
}
