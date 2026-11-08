//! What tells viewers of a raised mob's look changing: a baby's box and scale as BDS announced one
//! (`data/Mobs.bin`), growing up, a name tag, and its owner and lead holder.

use acacia_proto::packets::SetEntityData;
use acacia_proto::types::{
    EntityProperties, MetadataDictionaryItem as Item, MetadataDictionaryItemKey as Key, MetadataDictionaryItemType as Type, MetadataDictionaryItemValue as Value,
    MetadataDictionaryItemValueDefault as Plain, MetadataFlags1,
};

use crate::entity::recorded;

fn float(identifier: &str, baby: bool, key: Key) -> Option<f32> {
    recorded(identifier, baby)?.metadata.iter().find(|item| item.key == key).and_then(|item| if let Value::Default(Plain::Float(value)) = item.value { Some(value) } else { None })
}

/// The box (width, height) and scale a baby or grown mob of `identifier` was announced with; the
/// box it collides with is the one scaled. `None` for a kind with no such recording.
/// A baby of a kind recorded only grown has the box and scale its pack's born groups give (a
/// baby villager's 0.98 by 1.96 at 0.5, bds-g11share.txt).
pub fn mob_look(identifier: &str, baby: bool) -> Option<([f32; 2], f32)> {
    let recorded = || Some(([float(identifier, baby, Key::BoundingboxWidth)?, float(identifier, baby, Key::BoundingboxHeight)?], float(identifier, baby, Key::Scale).unwrap_or(1.0)));
    recorded().or_else(|| crate::husbandry(identifier).filter(|_| baby).and_then(|raised| raised.baby_look))
}

/// The `minecraft:movement` a baby or grown mob of `identifier` was announced with.
pub fn mob_pace(identifier: &str, baby: bool) -> Option<f32> {
    recorded(identifier, baby)?.attributes.iter().find(|attribute| attribute.name == "minecraft:movement").map(|attribute| attribute.value)
}

fn set(id: u64, metadata: Vec<Item>) -> SetEntityData {
    SetEntityData { runtime_entity_id: id, metadata, properties: EntityProperties { ints: Vec::new(), floats: Vec::new() }, tick: 0 }
}

fn flags(flags: MetadataFlags1) -> Item {
    Item { key: Key::Flags, r#type: Type::Long, legacy_type: 7, value: Value::Flags(flags) }
}

fn plain(key: Key, value: Plain) -> Item {
    let (r#type, legacy_type) = match value {
        Plain::Byte(_) => (Type::Byte, 0),
        Plain::Float(_) => (Type::Float, 3),
        Plain::String(_) => (Type::String, 4),
        _ => (Type::Long, 7),
    };
    Item { key, r#type, legacy_type, value: Value::Default(value) }
}

/// The mob `id`, whose flags are now `now` (`BABY` among them or not), is shown at `scale`, with a box of `size` when that changed: BDS told a sheep's
/// growing up without its unchanged box, a turtle's with it (research/captures/bds-bc1event.txt).
pub fn mob_age_set(id: u64, now: MetadataFlags1, scale: f32, size: Option<[f32; 2]>) -> SetEntityData {
    let mut items = vec![flags(now), plain(Key::Scale, Plain::Float(scale))];
    if let Some([width, height]) = size {
        items.extend([plain(Key::BoundingboxWidth, Plain::Float(width)), plain(Key::BoundingboxHeight, Plain::Float(height))]);
    }
    set(id, items)
}

/// The mob `id` is named `name`.
pub fn mob_name_set(id: u64, name: &str) -> SetEntityData {
    set(id, vec![plain(Key::Nametag, Plain::String(name.to_owned()))])
}

/// The mob `id`, whose flags are now `now`, belongs to the entity `owner` and is held on a lead by
/// `holder`, each a runtime id or -1 for none.
pub fn mob_links_set(id: u64, now: MetadataFlags1, owner: i64, holder: i64) -> SetEntityData {
    set(id, vec![flags(now), plain(Key::OwnerEid, Plain::Long(owner)), plain(Key::LeadHolderEid, Plain::Long(holder))])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_calf_is_smaller_than_a_cow_and_grows_into_its_box() {
        let (calf, cow) = (mob_look("minecraft:cow", true).unwrap(), mob_look("minecraft:cow", false).unwrap());
        // BDS announces a calf with the cow's box, scaled.
        assert_eq!((calf, cow), (([0.9, 1.3], 0.5), ([0.9, 1.3], 1.0)));
        assert_eq!(Some(cow.0), crate::mob_size("minecraft:cow"));
        let grown = mob_age_set(5, MetadataFlags1(0), cow.1, None);
        assert_eq!((grown.runtime_entity_id, grown.metadata.len()), (5, 2));
        assert_eq!(mob_age_set(5, MetadataFlags1(0), cow.1, Some(cow.0)).metadata.len(), 4);
        assert!(mob_look("sapling:nothing_recorded", true).is_none());
        assert_eq!(mob_look("minecraft:villager_v2", true), Some(([0.98, 1.96], 0.5)));
    }
}
