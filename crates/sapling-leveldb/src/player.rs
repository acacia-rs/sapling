//! What sapling keeps of a player between visits, under its own key. The fields are named and
//! shaped as in the game's player record (`Inventory` with a `Slot` in each stack, `Armor`,
//! `Offhand`, `SelectedInventorySlot`, `EnderChestInventory`, `PlayerLevel`,
//! `PlayerLevelProgress`), but the record is not the game's `player_server_<id>`: one of those
//! holding only items is not a player the game can load (docs/DESIGN.md "Inventories").

use acacia_proto::nbt::{self, List, LittleEndian, Nbt, Value};
use acacia_world::BlockRegistry;
use bytes::BytesMut;

use crate::{Error, ItemStack};

const KEY: &str = "sapling_player_";
const COMPOUND: u8 = 10;

/// A player's items and experience as they are kept.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlayerData {
    /// Hotbar first; a slot past the end is empty.
    pub main: Vec<Option<ItemStack>>,
    /// Helmet, chestplate, leggings, boots.
    pub armor: Vec<Option<ItemStack>>,
    pub offhand: Option<ItemStack>,
    /// The hotbar slot in the hand.
    pub selected: u8,
    /// What its ender chests hold; a slot past the end is empty.
    pub ender: Vec<Option<ItemStack>>,
    pub level: i32,
    /// How far into the next level, 0 to 1.
    pub progress: f32,
}

/// The database key of the player with this id (its UUID as text).
pub(crate) fn key(id: &str) -> Vec<u8> {
    [KEY, id].concat().into_bytes()
}

fn stacks(slots: &[Option<ItemStack>], registry: &BlockRegistry) -> Value {
    let stack = |slot: &Option<ItemStack>| Value::Compound(slot.as_ref().map_or_else(ItemStack::no_fields, |stack| stack.fields(registry)));
    Value::List(List { tag: COMPOUND, items: slots.iter().map(stack).collect() })
}

/// Stacks each with the `Slot` it lies in; empty slots are left out.
fn slotted(slots: &[Option<ItemStack>], registry: &BlockRegistry) -> Value {
    let held = |(slot, stack): (usize, &Option<ItemStack>)| {
        let mut fields = stack.as_ref()?.fields(registry);
        fields.push(("Slot".into(), Value::Byte(slot as i8)));
        Some(Value::Compound(fields))
    };
    Value::List(List { tag: COMPOUND, items: slots.iter().enumerate().filter_map(held).collect() })
}

fn read_slotted(stored: &[Value]) -> Result<Vec<Option<ItemStack>>, Error> {
    let mut slots = Vec::new();
    for stored in stored {
        let (Some(Value::Byte(slot)), Some(stack)) = (stored.get("Slot"), ItemStack::parse(stored)) else { continue };
        let slot = usize::try_from(*slot).map_err(|_| Error::Player("a negative slot"))?;
        slots.resize(slots.len().max(slot + 1), None);
        slots[slot] = Some(stack);
    }
    Ok(slots)
}

impl PlayerData {
    pub fn decode(record: &[u8]) -> Result<Self, Error> {
        let player = nbt::read::<LittleEndian>(&mut &record[..])?.value;
        let list = |key: &str| if let Some(Value::List(list)) = player.get(key) { &list.items[..] } else { &[] };
        let selected = if let Some(Value::Int(selected)) = player.get("SelectedInventorySlot") { *selected as u8 } else { 0 };
        let level = if let Some(Value::Int(level)) = player.get("PlayerLevel") { *level } else { 0 };
        let progress = if let Some(Value::Float(progress)) = player.get("PlayerLevelProgress") { *progress } else { 0.0 };
        Ok(Self {
            main: read_slotted(list("Inventory"))?,
            armor: list("Armor").iter().map(ItemStack::parse).collect(),
            offhand: list("Offhand").first().and_then(ItemStack::parse),
            selected,
            ender: read_slotted(list("EnderChestInventory"))?,
            level,
            progress,
        })
    }

    /// The record; `registry` names the blocks that block items place.
    pub fn encode(&self, registry: &BlockRegistry) -> Vec<u8> {
        let fields = vec![
            ("Inventory".into(), slotted(&self.main, registry)),
            ("Armor".into(), stacks(&self.armor, registry)),
            ("Offhand".into(), stacks(std::slice::from_ref(&self.offhand), registry)),
            ("SelectedInventorySlot".into(), Value::Int(self.selected.into())),
            ("EnderChestInventory".into(), slotted(&self.ender, registry)),
            ("PlayerLevel".into(), Value::Int(self.level)),
            ("PlayerLevelProgress".into(), Value::Float(self.progress)),
        ];
        let mut out = BytesMut::new();
        nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: Value::Compound(fields) });
        out.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_player_reads_back_as_it_was_written() {
        let registry = BlockRegistry::vanilla();
        let stone = registry.network_hash(registry.states_of("minecraft:stone").next().unwrap().0);
        let stack = |name: &str, count, block| Some(ItemStack { name: name.into(), count, damage: 0, block, tag: None });
        let sword = ItemStack { tag: Some(Value::Compound(vec![("Damage".into(), Value::Int(3))])), ..stack("minecraft:iron_sword", 1, None).unwrap() };
        let mut main = vec![None; 36];
        (main[0], main[8], main[35]) = (stack("minecraft:stone", 64, Some(stone)), Some(sword), stack("minecraft:diamond", 5, None));
        let ender = vec![None, None, stack("minecraft:emerald", 12, None)];
        let armor = vec![stack("minecraft:iron_helmet", 1, None), None, None, None];
        let player = PlayerData { main, armor, offhand: stack("minecraft:shield", 1, None), selected: 8, ender, level: 17, progress: 0.25 };
        assert_eq!(PlayerData::decode(&player.encode(&registry)).unwrap(), player);

        let empty = PlayerData::decode(&PlayerData::default().encode(&registry)).unwrap();
        assert_eq!((empty.main.len(), empty.ender.len(), empty.offhand, empty.selected, empty.level), (0, 0, None, 0, 0));
        assert!(PlayerData::decode(&[1, 2, 3]).is_err());
    }
}
