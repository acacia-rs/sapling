//! Item stacks on the wire and the tables behind them: the network ids the bundled ItemRegistry
//! gives item names, and the creative inventory's entries (`CreativeContent.bin`).

use std::collections::HashMap;
use std::sync::LazyLock;

use acacia_proto::nbt::{self, Nbt};
use acacia_proto::packets::{CreativeContent, CreativeContentItemsItem, ItemRegistry};
use acacia_proto::types::{
    ItemExtraDataWithBlockingTick as ShieldExtra, ItemExtraDataWithBlockingTickHasNbt as ShieldHasNbt, ItemExtraDataWithBlockingTickNbt as ShieldNbt,
    ItemExtraDataWithoutBlockingTick as Extra, ItemExtraDataWithoutBlockingTickHasNbt as HasNbt, ItemExtraDataWithoutBlockingTickNbt as ExtraNbt, ItemLegacy,
    ItemLegacyExtra, ItemV4, ItemV4Extra,
};

use crate::templates::{CREATIVE_CONTENT, ITEM_REGISTRY, decode};

/// The one item whose extra data carries a blocking tick (acacia-proto docs/proto.md).
pub(crate) const SHIELD: &str = "minecraft:shield";
const NBT_VERSION: u8 = 1;
/// Items a slot holds one of, and sixteen of, by the end of their name.
// TODO: per-item data; the registry a client is sent does not carry stack sizes.
const SINGLE: &[&str] = &[
    "_sword", "_pickaxe", "_axe", "_shovel", "_hoe", "_helmet", "_chestplate", "_leggings", "_boots", "_bucket", "potion", ":bow", "crossbow", "shield", "elytra",
    "_boat", "minecart", "enchanted_book", "totem_of_undying", ":bed", "saddle", "shears", "flint_and_steel", "fishing_rod", "trident", ":mace",
    "_horse_armor", "writable_book", "written_book", "spyglass", "shulker_box", "_stew", "_soup", "cake", "_on_a_stick", ":brush", "bundle",
];
const SIXTEEN: &[&str] = &["ender_pearl", "snowball", ":egg", "_sign", ":bucket", "banner", "honey_bottle", "armor_stand", "ender_eye"];

pub(crate) static NETWORK_IDS: LazyLock<HashMap<String, i16>> =
    LazyLock::new(|| decode::<ItemRegistry>(ITEM_REGISTRY).itemstates.into_iter().map(|item| (item.name, item.runtime_id)).collect());
static NAMES: LazyLock<HashMap<i16, &'static str>> = LazyLock::new(|| NETWORK_IDS.iter().map(|(name, id)| (*id, name.as_str())).collect());
static CREATIVE: LazyLock<HashMap<u32, CreativeItem>> = LazyLock::new(|| {
    set_shield_item_id();
    let entry = |entry: CreativeContentItemsItem| Some((entry.entry_id, described(entry.item)?));
    decode::<CreativeContent>(CREATIVE_CONTENT).items.into_iter().filter_map(entry).collect()
});

/// An item of a recorded list, by name; `None` for one the registry does not list.
pub(crate) fn described(item: ItemLegacy) -> Option<CreativeItem> {
    let tag = match item.extra {
        ItemLegacyExtra::Default(extra) => extra.and_then(|extra| extra.nbt).map(|nbt| nbt.nbt.value),
        ItemLegacyExtra::ShieldItemID(extra) => extra.and_then(|extra| extra.nbt).map(|nbt| nbt.nbt.value),
    };
    let name = *NAMES.get(&i16::try_from(item.network_id).ok()?)?;
    let block = Some(item.block_runtime_id as u32).filter(|hash| *hash != 0);
    Some(CreativeItem { name, damage: item.metadata as i16, block, tag })
}

/// An item stack as a world stores it.
#[derive(Clone, Copy)]
pub struct Stack<'a> {
    /// Namespaced, e.g. `minecraft:diamond`.
    pub name: &'a str,
    pub count: u8,
    /// Wear, or what tells apart the kinds of an old item.
    pub damage: i16,
    /// The network hash of the block the item places, when it is one.
    pub block: Option<u32>,
    /// The stack's own NBT.
    // TODO: BDS adds `Damage` to the tag of an item that wears; a stored stack without a tag goes without.
    pub tag: Option<&'a nbt::Value>,
}

/// One entry of the creative inventory, or what a recipe makes.
#[derive(Debug, Clone, PartialEq)]
pub struct CreativeItem {
    pub name: &'static str,
    pub damage: i16,
    pub block: Option<u32>,
    pub tag: Option<nbt::Value>,
}

/// Tells acacia-proto which item is the shield, as a client does on ItemRegistry: item stacks a
/// client sends cannot be read before. The id is one per process.
pub fn set_shield_item_id() {
    acacia_proto::manual::set_shield_item_id(NETWORK_IDS[SHIELD].into());
}

/// Whether the bundled ItemRegistry lists the item, so that a client can be shown it.
pub fn is_item(name: &str) -> bool {
    NETWORK_IDS.contains_key(name)
}

/// The name of the item a stack on the wire names by `network_id`.
pub fn item_name(network_id: i32) -> Option<&'static str> {
    NAMES.get(&i16::try_from(network_id).ok()?).copied()
}

/// `name` as the ItemRegistry spells it, kept for the whole run; `None` for an unlisted item.
pub fn static_item_name(name: &str) -> Option<&'static str> {
    NETWORK_IDS.get_key_value(name).map(|(name, _)| name.as_str())
}

/// What a client names by `entry_id` when it takes an item from the creative inventory.
pub fn creative_item(entry_id: u32) -> Option<&'static CreativeItem> {
    CREATIVE.get(&entry_id)
}

/// How many of an item one slot holds.
pub fn max_stack(name: &str) -> u8 {
    let ends = |list: &[&str]| list.iter().any(|end| name.ends_with(end));
    // A disc's name ends with which one it is; BDS gave two as two (bds-item-names.txt).
    match (ends(SIXTEEN), ends(SINGLE) || name.contains(":music_disc_")) {
        (true, _) => 16,
        (false, true) => 1,
        (false, false) => 64,
    }
}

fn extra(name: &str, tag: Option<&nbt::Value>) -> ItemV4Extra {
    let shown = tag.map(shown_tag);
    let tag = shown.as_deref();
    // Never the empty extra a reader takes for "none": BDS always writes one, tag or not.
    let nbt = || Nbt { name: String::new(), value: tag.cloned().unwrap_or(nbt::Value::Compound(Vec::new())) };
    if name == SHIELD {
        let has_nbt = if tag.is_some() { ShieldHasNbt::True } else { ShieldHasNbt::False };
        let nbt = tag.map(|_| ShieldNbt { version: NBT_VERSION, nbt: nbt() });
        return ItemV4Extra::ShieldItemID(Some(ShieldExtra { has_nbt, nbt, can_place_on: Vec::new(), can_destroy: Vec::new(), blocking_tick: 0 }));
    }
    let has_nbt = if tag.is_some() { HasNbt::True } else { HasNbt::False };
    let nbt = tag.map(|_| ExtraNbt { version: NBT_VERSION, nbt: nbt() });
    ItemV4Extra::Default(Some(Extra { has_nbt, nbt, can_place_on: Vec::new(), can_destroy: Vec::new() }))
}

/// A tag as clients see it: a bundle's contents stay on the server, which shows them as a
/// container of their own (research/captures/bds-bundles.txt).
fn shown_tag(tag: &nbt::Value) -> std::borrow::Cow<'_, nbt::Value> {
    match tag {
        nbt::Value::Compound(fields) if fields.iter().any(|(key, _)| key == STORAGE_CONTENT) => {
            std::borrow::Cow::Owned(nbt::Value::Compound(fields.iter().filter(|(key, _)| key != STORAGE_CONTENT).cloned().collect()))
        }
        other => std::borrow::Cow::Borrowed(other),
    }
}

/// Where a bundle's tag keeps what it holds, as BDS saves it.
pub const STORAGE_CONTENT: &str = "storage_item_component_content";

/// An empty slot.
pub fn no_item() -> ItemV4 {
    ItemV4 { network_id: 0, count: 0, metadata: 0, has_stack_id: false, stack_id: None, block_runtime_id: 0, extra: ItemV4Extra::Default(None) }
}

/// A stack on the wire, with the id a client names it by in its inventory requests (`None` where
/// a packet carries stacks without one). `None` for an item the registry does not list.
pub fn item(stack: &Stack<'_>, stack_id: Option<i32>) -> Option<ItemV4> {
    Some(ItemV4 {
        network_id: *NETWORK_IDS.get(stack.name)?,
        count: stack.count.into(),
        metadata: stack.damage.max(0) as u32,
        has_stack_id: stack_id.is_some(),
        stack_id,
        block_runtime_id: stack.block.unwrap_or(0),
        extra: extra(stack.name, stack.tag),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_creative_inventory_names_items_the_registry_lists() {
        assert!(CREATIVE.len() > 1000, "{} creative entries", CREATIVE.len());
        let named = |name: &str| CREATIVE.values().filter(|item| item.name == name).collect::<Vec<_>>();
        assert!(named("minecraft:stone")[0].block.is_some(), "a block item names the block it places");
        assert!(named("minecraft:diamond")[0].block.is_none());
        assert!(named("minecraft:enchanted_book").iter().all(|book| book.tag.is_some()) && named("minecraft:enchanted_book").len() > 10);
        assert!(creative_item(0).is_none() || creative_item(u32::MAX).is_none());
    }

    #[test]
    fn a_slot_holds_as_many_as_the_item_stacks_to() {
        let sizes = ["minecraft:stone", "minecraft:diamond_sword", "minecraft:ender_pearl", "minecraft:bucket", "minecraft:water_bucket", "minecraft:oak_sign"].map(max_stack);
        assert_eq!(sizes, [64, 1, 16, 16, 1, 16]);
        assert!(is_item("minecraft:stone") && !is_item("sapling:no_such_item"));
        assert_eq!(item(&Stack { name: "minecraft:stone", count: 3, damage: 0, block: None, tag: None }, Some(7)).unwrap().stack_id, Some(7));
    }
}
