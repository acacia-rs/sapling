//! Items put on a mob by `/replaceitem entity`: its armour and hands, a mount's saddle, body
//! armour or carpet, and its inventory.

use crate::{EntityId, Item, ShardCtx};

/// Where on a mob an item goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobSlot {
    /// Head 0 to feet 3.
    Armor(u8),
    MainHand,
    OffHand,
    /// A horse's, donkey's or mule's saddle.
    Saddle,
    /// A horse's armour or a llama's carpet (`slot.armor`).
    Body,
    /// A slot of the chest a donkey, mule or llama carries.
    Chest(usize),
    /// A slot of a mount's whole inventory, its tack first.
    Inventory(usize),
}

/// Why an item did not go on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MobSlotRefusal {
    /// The mob has no such slot (`commands.generic.exception`).
    NoSlot,
    /// The slot is past the last of `size`.
    BadSlot { size: usize },
}

impl ShardCtx<'_> {
    /// Puts `item` (`None`: nothing) in a mob's slot as `/replaceitem entity` does, shown and
    /// heard as BDS's were (research/captures/bds-fixreplace.txt).
    pub fn set_mob_item(&mut self, id: EntityId, slot: MobSlot, item: Option<&Item>) -> Result<(), MobSlotRefusal> {
        self.backend.set_mob_item(id, slot, item)
    }
}
