//! Items in a player's inventory, as plugins name them.

/// A stack of one item. What a stack holds beyond this (enchantments, a custom name) stays with
/// it in the inventory but does not show here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Namespaced, e.g. `minecraft:diamond`.
    pub name: String,
    pub count: u8,
    /// Wear, or what tells apart the kinds of an old item.
    pub damage: i16,
}

impl Item {
    pub fn new(name: impl Into<String>, count: u8) -> Self {
        Self { name: name.into(), count, damage: 0 }
    }
}

/// A place in a player's inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Slot {
    /// 0 to 35; the first nine are the hotbar.
    Main(u8),
    /// 0 helmet, 1 chestplate, 2 leggings, 3 boots.
    Armor(u8),
    Offhand,
}

/// A player throwing a stack out of their inventory. Cancelled, the stack stays where it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropEvent {
    pub(crate) player: crate::Player,
    item: Item,
    pub(crate) cancelled: bool,
}

impl DropEvent {
    pub fn new(player: crate::Player, item: Item) -> Self {
        Self { player, item, cancelled: false }
    }

    /// What is thrown: the item and how many of it.
    pub fn item(&self) -> &Item {
        &self.item
    }
}
