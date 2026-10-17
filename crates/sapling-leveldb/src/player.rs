//! What sapling keeps of a player between visits, under its own key. The fields are named and
//! shaped as in the game's player record (`Inventory` with a `Slot` in each stack, `Armor`,
//! `Offhand`, `SelectedInventorySlot`, `EnderChestInventory`, `PlayerLevel`,
//! `PlayerLevelProgress`, `Attributes` with a `Name` and a `Current` each, `SpawnBlockPositionX` to `Z`
//! and `SpawnX` to `Z`), but the record is not the game's `player_server_<id>`: one of those
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
    /// Health, hunger, saturation and exhaustion; `None` in a record older than they are kept.
    pub vitals: Option<[f32; 4]>,
    /// The head of the bed it rises by and the cell it rises in, in the first world.
    pub bed: Option<[[i32; 3]; 2]>,
    /// Each effect on it: its id, its level from 0, the ticks left of it, and 1 if it shows.
    pub effects: Vec<[i32; 4]>,
}

// TODO: the names the game keeps an effect under, as remembered: no BDS record was read.
const EFFECTS: &str = "ActiveEffects";
const EFFECT: [&str; 4] = ["Id", "Amplifier", "Duration", "ShowParticles"];

fn effect(kept: &Value) -> Option<[i32; 4]> {
    let part = |key: &str| match kept.get(key) {
        Some(Value::Byte(part)) => Some(i32::from(*part)),
        Some(Value::Int(part)) => Some(*part),
        _ => None,
    };
    Some([part(EFFECT[0])?, part(EFFECT[1])?, part(EFFECT[2])?, part(EFFECT[3]).unwrap_or(1)])
}

fn effect_fields([id, amplifier, left, shown]: &[i32; 4]) -> Value {
    Value::Compound(vec![(EFFECT[0].into(), Value::Byte(*id as i8)), (EFFECT[1].into(), Value::Byte(*amplifier as i8)), (EFFECT[2].into(), Value::Int(*left)), (EFFECT[3].into(), Value::Byte(*shown as i8))])
}

const BED: [&str; 3] = ["SpawnBlockPositionX", "SpawnBlockPositionY", "SpawnBlockPositionZ"];
const BEDSIDE: [&str; 3] = ["SpawnX", "SpawnY", "SpawnZ"];

/// The attributes `PlayerData::vitals` are kept as, in its order.
const VITALS: [&str; 4] = ["minecraft:health", "minecraft:player.hunger", "minecraft:player.saturation", "minecraft:player.exhaustion"];

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
        let attribute = |name: &str| list("Attributes").iter().find(|kept| kept.get("Name") == Some(&Value::String(name.into()))).and_then(|kept| kept.get("Current"));
        let current = |name: &str| if let Some(Value::Float(current)) = attribute(name) { Some(*current) } else { None };
        let vitals = if let [Some(health), Some(hunger), Some(saturation), Some(exhaustion)] = VITALS.map(current) { Some([health, hunger, saturation, exhaustion]) } else { None };
        let int = |key: &str| if let Some(Value::Int(value)) = player.get(key) { Some(*value) } else { None };
        let cell = |keys: [&str; 3]| if let [Some(x), Some(y), Some(z)] = keys.map(int) { Some([x, y, z]) } else { None };
        Ok(Self {
            bed: cell(BED).zip(cell(BEDSIDE)).map(|(head, spot)| [head, spot]),
            main: read_slotted(list("Inventory"))?,
            armor: list("Armor").iter().map(ItemStack::parse).collect(),
            offhand: list("Offhand").first().and_then(ItemStack::parse),
            selected,
            ender: read_slotted(list("EnderChestInventory"))?,
            level,
            progress,
            vitals,
            effects: list(EFFECTS).iter().filter_map(effect).collect(),
        })
    }

    /// The record; `registry` names the blocks that block items place.
    pub fn encode(&self, registry: &BlockRegistry) -> Vec<u8> {
        let attribute = |(name, current): (&str, f32)| Value::Compound(vec![("Name".into(), Value::String(name.into())), ("Current".into(), Value::Float(current))]);
        let attributes = self.vitals.iter().flat_map(|vitals| VITALS.into_iter().zip(*vitals)).map(attribute).collect();
        let bed = self.bed.iter().flat_map(|[head, spot]| BED.into_iter().zip(*head).chain(BEDSIDE.into_iter().zip(*spot))).map(|(key, value)| (key.into(), Value::Int(value)));
        let mut fields = vec![
            ("Attributes".into(), Value::List(List { tag: COMPOUND, items: attributes })),
            ("Inventory".into(), slotted(&self.main, registry)),
            ("Armor".into(), stacks(&self.armor, registry)),
            ("Offhand".into(), stacks(std::slice::from_ref(&self.offhand), registry)),
            ("SelectedInventorySlot".into(), Value::Int(self.selected.into())),
            ("EnderChestInventory".into(), slotted(&self.ender, registry)),
            ("PlayerLevel".into(), Value::Int(self.level)),
            ("PlayerLevelProgress".into(), Value::Float(self.progress)),
        ];
        fields.extend(bed);
        if !self.effects.is_empty() {
            fields.push((EFFECTS.into(), Value::List(List { tag: COMPOUND, items: self.effects.iter().map(effect_fields).collect() })));
        }
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
        let player = PlayerData { main, armor, offhand: stack("minecraft:shield", 1, None), selected: 8, ender, level: 17, progress: 0.25, vitals: Some([7.5, 18.0, 0.0, 2.25]), bed: Some([[5, 64, -3], [4, 64, -3]]), effects: vec![[1, 1, 600, 1], [19, 0, 40, 0]] };
        assert_eq!(PlayerData::decode(&player.encode(&registry)).unwrap(), player);

        let empty = PlayerData::decode(&PlayerData::default().encode(&registry)).unwrap();
        assert_eq!((empty.main.len(), empty.ender.len(), empty.offhand, empty.selected, empty.level, empty.vitals, empty.bed), (0, 0, None, 0, 0, None, None));
        assert!(PlayerData::decode(&[1, 2, 3]).is_err());
    }
}
