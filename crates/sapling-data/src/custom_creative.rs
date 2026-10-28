//! The recorded ItemRegistry and CreativeContent with the custom items in: their registry
//! entries after the recorded ones, and the listed ones in the creative inventory after BDS's,
//! each in its category's last unnamed group or the named group it asks for.

use std::sync::LazyLock;

use acacia_proto::packets::{CreativeContent, CreativeContentItemsItem, ItemRegistry};
use acacia_proto::types::{ItemExtraDataWithoutBlockingTick as Extra, ItemExtraDataWithoutBlockingTickHasNbt as HasNbt, ItemLegacy, ItemLegacyExtra};
use sapling_api::CreativeCategory;

use crate::custom_items::{self, Registered};
use crate::items::CreativeItem;
use crate::templates::{CREATIVE_CONTENT, ITEM_REGISTRY, decode};

static BASE: LazyLock<CreativeContent> = LazyLock::new(|| {
    crate::set_shield_item_id();
    decode(CREATIVE_CONTENT)
});
static LAST_ENTRY: LazyLock<u32> = LazyLock::new(|| BASE.items.iter().map(|item| item.entry_id).max().unwrap_or(0));

/// The custom items the creative inventory lists, each with its entry id.
fn listed() -> impl Iterator<Item = (u32, std::sync::Arc<Registered>)> {
    let listed = custom_items::all().into_iter().filter(|item| item.category != CreativeCategory::None);
    (*LAST_ENTRY + 1..).zip(listed)
}

/// What the custom creative entry `entry_id` gives.
pub(crate) fn entry(entry_id: u32) -> Option<CreativeItem> {
    let (_, item) = listed().find(|(id, _)| *id == entry_id)?;
    Some(CreativeItem { name: item.name, damage: 0, block: item.block, tag: None })
}

pub(crate) fn item_registry() -> ItemRegistry {
    let mut registry: ItemRegistry = decode(ITEM_REGISTRY);
    registry.itemstates.extend(custom_items::all().iter().map(|item| item.entry.clone()));
    registry
}

pub(crate) fn creative_content() -> CreativeContent {
    let mut content = BASE.clone();
    for (entry_id, item) in listed() {
        let category = i64::from(item.category.id());
        let named = content.groups.iter().position(|group| !item.group.is_empty() && group.name == item.group);
        let plain = || content.groups.iter().rposition(|group| group.category.to_raw() == category && group.name.is_empty());
        let Some(group) = named.or_else(plain) else { continue };
        let extra = Extra { has_nbt: HasNbt::False, nbt: None, can_place_on: Vec::new(), can_destroy: Vec::new() };
        let stack = ItemLegacy { network_id: i32::from(item.id), count: 1, metadata: 0, block_runtime_id: item.block.unwrap_or(0) as i32, extra: ItemLegacyExtra::Default(Some(extra)) };
        content.items.push(CreativeContentItemsItem { entry_id, item: stack, group_index: group as u32 });
    }
    content
}
