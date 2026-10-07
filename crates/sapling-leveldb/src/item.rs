//! An item stack as the game stores it: `{Name, Count, Damage, Block, tag}`.

use acacia_proto::nbt::{Str, Value};
use acacia_world::BlockRegistry;

use crate::states::{block_of_hash, network_hash};

/// A stored item stack: an item entity's `Item`, or a slot of a mob's `Armor`, `Mainhand`,
/// `Offhand`, or of a player's inventory.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemStack {
    /// Namespaced, e.g. `minecraft:diamond`.
    pub name: String,
    pub count: u8,
    /// Wear, or what tells apart the kinds of an old item.
    pub damage: i16,
    /// The network hash of the block the item places, when it is one.
    pub block: Option<u32>,
    /// The stack's own NBT (`tag`): enchantments, a custom name and the like.
    pub tag: Option<Value>,
}

impl ItemStack {
    pub fn parse(item: &Value) -> Option<Self> {
        // An empty slot is a stack without a name.
        let Some(Value::String(name)) = item.get("Name").filter(|name| !matches!(name, Value::String(name) if name.is_empty())) else { return None };
        let count = if let Some(Value::Byte(count)) = item.get("Count") { *count as u8 } else { 1 };
        let damage = if let Some(Value::Short(damage)) = item.get("Damage") { *damage } else { 0 };
        Some(Self { name: name.to_string(), count, damage, block: item.get("Block").and_then(network_hash), tag: item.get("tag").cloned() })
    }

    /// The stack's fields; a block the registry cannot name is left out.
    pub fn fields(&self, registry: &BlockRegistry) -> Vec<(Str, Value)> {
        let mut fields = vec![("Name".into(), Value::String(self.name.as_str().into())), ("Count".into(), Value::Byte(self.count as i8)), ("Damage".into(), Value::Short(self.damage))];
        fields.extend(self.block.and_then(|hash| block_of_hash(registry, hash)).map(|block| ("Block".into(), block)));
        fields.extend(self.tag.clone().map(|tag| ("tag".into(), tag)));
        fields
    }

    /// The fields of a slot that holds nothing.
    pub(crate) fn no_fields() -> Vec<(Str, Value)> {
        vec![("Name".into(), Value::String(Str::default())), ("Count".into(), Value::Byte(0)), ("Damage".into(), Value::Short(0))]
    }
}
