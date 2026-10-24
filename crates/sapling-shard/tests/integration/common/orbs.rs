use acacia_proto::RawPacket;
use acacia_proto::packets::AddEntity;
use acacia_proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Value, MetadataDictionaryItemValueDefault as Plain};

use super::all;

/// What each experience orb added in `sent` is worth.
pub fn worths(sent: &[RawPacket]) -> Vec<i32> {
    let worth = |orb: &AddEntity| orb.metadata.iter().find(|item| item.key == Key::ExperienceValue).and_then(|item| if let Value::Default(Plain::Int(value)) = item.value { Some(value) } else { None });
    all::<AddEntity>(sent).iter().filter(|added| added.entity_type == "minecraft:xp_orb").filter_map(worth).collect()
}
