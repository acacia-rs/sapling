//! What a world keeps of a player between visits: BDS's `player_server_` compound (keys.rs for
//! where it lies), with BDS's field names and shapes (BDSRE: Player save 1402b2d30, Actor save
//! 140d40ec0, effects 141c57070). Sapling writes the fields it knows over the stored record and
//! keeps the rest as they were.

mod keys;
pub(crate) mod parts;
#[cfg(test)]
mod tests;

use acacia_proto::nbt::{self, LittleEndian, List, Nbt, Str, Value};
use acacia_world::BlockRegistry;
use bytes::BytesMut;

use crate::{Error, ItemStack};
pub(crate) use keys::{load, store};
use parts::{COMPOUND, effect, effect_fields, floats, read_slotted, remove, set, slotted, stacks, with_attributes};

/// A player's items, state and place as they are kept.
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
    /// The head of the bed (or the respawn anchor) it rises by and the cell it rises in.
    pub bed: Option<[[i32; 3]; 2]>,
    /// The dimension `bed` is in (0 overworld, 1 nether).
    pub spawn_dimension: i32,
    /// The cell of its eyes when it last died, and the dimension; `None` for one never dead.
    pub last_death: Option<([i32; 3], i32)>,
    /// Each effect on it: its id, its level from 0, the ticks left of it, and 1 if it shows.
    pub effects: Vec<[i32; 4]>,
    /// The dimension it was in (0 overworld, 1 nether, 2 end), and where its feet were there.
    pub dimension: i32,
    pub position: Option<[f32; 3]>,
    /// Yaw and pitch, in degrees.
    pub rotation: Option<[f32; 2]>,
    /// BDS's game type: 0 survival, 1 creative, 2 adventure, 6 spectator (5 is the world's).
    pub game_mode: Option<i32>,
    /// What its enchanting table offers are drawn from; `None` in a record without one.
    pub enchant_seed: Option<i32>,
    /// What `/tag` put on it.
    pub tags: Vec<String>,
    /// Its actor id (`UniqueID`), by which the world's scoreboard knows it.
    pub unique_id: Option<i64>,
    /// Ticks since it last slept, which phantoms come by (`TimeSinceRest`, an int: BDSRE 141bf7240).
    pub rested: Option<i32>,
    /// Its recipe book; `None` in a record without one.
    pub recipes: Option<RecipeRecord>,
}

/// BDS's `recipe_unlocking` compound (BDSRE: UnlockedRecipesServerComponent save 141172dc0).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RecipeRecord {
    /// `unlocked_recipes`: recipe ids.
    pub unlocked: Vec<String>,
    /// `used_contexts`: the unlocking contexts already met, a bit each.
    pub used_contexts: i32,
}

/// BDS keeps an actor's `Pos` where its position is, a player's eyes: this far above the feet.
const EYE_HEIGHT: f32 = 1.62001;
const DIMENSION: &str = "DimensionId";
const EFFECTS: &str = "ActiveEffects";
const GAME_MODE: &str = "PlayerGameMode";
const ENCHANT_SEED: &str = "EnchantmentSeed";
const UNIQUE_ID: &str = "UniqueID";
const RESTED: &str = "TimeSinceRest";
const RECIPES: [&str; 3] = ["recipe_unlocking", "used_contexts", "unlocked_recipes"];
const STRING: u8 = 8;
const BED: [&str; 3] = ["SpawnBlockPositionX", "SpawnBlockPositionY", "SpawnBlockPositionZ"];
const BEDSIDE: [&str; 3] = ["SpawnX", "SpawnY", "SpawnZ"];
const SPAWN_DIMENSION: &str = "SpawnDimension";
/// Where it last died (BDSRE: Player load 1402af220 reads them only all together).
const DEATH: [&str; 3] = ["DeathPositionX", "DeathPositionY", "DeathPositionZ"];
const DEATH_DIMENSION: &str = "DeathDimension";
const HAS_DIED: &str = "HasDiedBefore";
/// What BDS keeps in the spawn fields of a player with no spawn point of its own (BDSRE: Player
/// load 1402af220 sets these when the fields are missing).
const NO_SPAWN: i32 = i32::MIN;
const NO_SPAWN_DIMENSION: i32 = 3;

/// The attributes `PlayerData::vitals` are kept as, in its order.
const VITALS: [&str; 4] = ["minecraft:health", "minecraft:player.hunger", "minecraft:player.saturation", "minecraft:player.exhaustion"];

fn compound(record: &[u8]) -> Result<Vec<(Str, Value)>, Error> {
    match nbt::read::<LittleEndian>(&mut &record[..])?.value {
        Value::Compound(fields) => Ok(fields),
        _ => Err(Error::Player("a record that is not a compound")),
    }
}

fn recipe_book(kept: &Value) -> RecipeRecord {
    let unlocked = match kept.get(RECIPES[2]) {
        Some(Value::List(list)) => list.items.iter().filter_map(|id| if let Value::String(id) = id { Some(id.to_string()) } else { None }).collect(),
        _ => Vec::new(),
    };
    RecipeRecord { unlocked, used_contexts: if let Some(Value::Int(used)) = kept.get(RECIPES[1]) { *used } else { 0 } }
}

impl PlayerData {
    /// Reads a `player_server_` record.
    pub fn decode(record: &[u8]) -> Result<Self, Error> {
        Self::read(Value::Compound(compound(record)?), EYE_HEIGHT)
    }

    /// Reads a record sapling kept under `sapling_player_`, whose `Pos` is at the feet.
    pub(crate) fn decode_legacy(record: &[u8]) -> Result<Self, Error> {
        Self::read(Value::Compound(compound(record)?), 0.0)
    }

    fn read(player: Value, eyes: f32) -> Result<Self, Error> {
        let list = |key: &str| if let Some(Value::List(list)) = player.get(key) { &list.items[..] } else { &[] };
        let attribute = |name: &str| list("Attributes").iter().find(|kept| kept.get("Name") == Some(&Value::String(name.into()))).and_then(|kept| kept.get("Current"));
        let current = |name: &str| if let Some(Value::Float(current)) = attribute(name) { Some(*current) } else { None };
        let vitals = if let [Some(health), Some(hunger), Some(saturation), Some(exhaustion)] = VITALS.map(current) { Some([health, hunger, saturation, exhaustion]) } else { None };
        let int = |key: &str| if let Some(Value::Int(value)) = player.get(key) { Some(*value) } else { None };
        let float = |key: &str| if let Some(Value::Float(value)) = player.get(key) { Some(*value) } else { None };
        let cell = |keys: [&str; 3]| if let [Some(x), Some(y), Some(z)] = keys.map(int) { Some([x, y, z]).filter(|cell| !cell.contains(&NO_SPAWN)) } else { None };
        let spawn_dimension = int(SPAWN_DIMENSION).unwrap_or(0);
        let died = matches!(player.get(HAS_DIED), Some(Value::Byte(1..)));
        Ok(Self {
            bed: cell(BED).zip(cell(BEDSIDE)).filter(|_| matches!(spawn_dimension, 0 | 1)).map(|(head, spot)| [head, spot]),
            spawn_dimension: if spawn_dimension == 1 { 1 } else { 0 },
            last_death: cell(DEATH).zip(int(DEATH_DIMENSION)).filter(|_| died),
            main: read_slotted(list("Inventory"))?,
            armor: list("Armor").iter().map(ItemStack::parse).collect(),
            offhand: list("Offhand").first().and_then(ItemStack::parse),
            selected: int("SelectedInventorySlot").unwrap_or(0) as u8,
            ender: read_slotted(list("EnderChestInventory"))?,
            level: int("PlayerLevel").unwrap_or(0),
            progress: float("PlayerLevelProgress").unwrap_or(0.0),
            vitals,
            effects: list(EFFECTS).iter().filter_map(effect).collect(),
            dimension: int(DIMENSION).unwrap_or(0),
            position: match list("Pos") {
                [Value::Float(x), Value::Float(y), Value::Float(z)] => Some([*x, *y - eyes, *z]),
                _ => None,
            },
            rotation: match list("Rotation") {
                [Value::Float(yaw), Value::Float(pitch)] => Some([*yaw, *pitch]),
                _ => None,
            },
            game_mode: int(GAME_MODE),
            enchant_seed: int(ENCHANT_SEED),
            tags: crate::actor_tags::read_tags(&player),
            unique_id: if let Some(Value::Long(id)) = player.get(UNIQUE_ID) { Some(*id) } else { None },
            rested: int(RESTED),
            recipes: player.get(RECIPES[0]).map(recipe_book),
        })
    }

    /// A record of this player alone; `registry` names the blocks that block items place.
    pub fn encode(&self, registry: &BlockRegistry) -> Vec<u8> {
        self.encode_over(registry, None).expect("no record to read")
    }

    /// `previous`, a stored record, with this player's fields written over it.
    pub(crate) fn encode_over(&self, registry: &BlockRegistry, previous: Option<&[u8]>) -> Result<Vec<u8>, Error> {
        let mut fields = previous.map(compound).transpose()?.unwrap_or_default();
        let stored_attributes = fields.iter().find(|(name, _)| name == "Attributes").map(|(_, attributes)| attributes);
        let attributes = with_attributes(stored_attributes, self.vitals.iter().flat_map(|vitals| VITALS.into_iter().zip(*vitals)));
        set(&mut fields, "Attributes", attributes);
        set(&mut fields, "Inventory", slotted(&self.main, registry));
        set(&mut fields, "Armor", stacks(&self.armor, registry));
        set(&mut fields, "Offhand", stacks(std::slice::from_ref(&self.offhand), registry));
        set(&mut fields, "SelectedInventorySlot", Value::Int(self.selected.into()));
        set(&mut fields, "EnderChestInventory", slotted(&self.ender, registry));
        set(&mut fields, "PlayerLevel", Value::Int(self.level));
        set(&mut fields, "PlayerLevelProgress", Value::Float(self.progress));
        set(&mut fields, DIMENSION, Value::Int(self.dimension));
        // A player with no place kept starts at the world's spawn, not at an older place.
        match self.position {
            Some([x, y, z]) => set(&mut fields, "Pos", floats(&[x, y + EYE_HEIGHT, z])),
            None => remove(&mut fields, "Pos"),
        }
        match self.rotation {
            Some(rotation) => set(&mut fields, "Rotation", floats(&rotation)),
            None => remove(&mut fields, "Rotation"),
        }
        if let Some(mode) = self.game_mode {
            set(&mut fields, GAME_MODE, Value::Int(mode));
        }
        if let Some(seed) = self.enchant_seed {
            set(&mut fields, ENCHANT_SEED, Value::Int(seed));
        }
        if let Some(id) = self.unique_id {
            set(&mut fields, UNIQUE_ID, Value::Long(id));
        }
        if let Some(rested) = self.rested {
            set(&mut fields, RESTED, Value::Int(rested));
        }
        if let Some(book) = &self.recipes {
            let unlocked = Value::List(List { tag: STRING, items: book.unlocked.iter().map(|id| Value::String(id.as_str().into())).collect() });
            set(&mut fields, RECIPES[0], Value::Compound(vec![(RECIPES[1].into(), Value::Int(book.used_contexts)), (RECIPES[2].into(), unlocked)]));
        }
        crate::actor_tags::write_tags(&mut fields, &self.tags);
        match self.effects.is_empty() {
            true => remove(&mut fields, EFFECTS),
            false => set(&mut fields, EFFECTS, Value::List(List { tag: COMPOUND, items: self.effects.iter().map(effect_fields).collect() })),
        }
        let [head, spot] = self.bed.unwrap_or([[NO_SPAWN; 3]; 2]);
        for (key, value) in BED.into_iter().zip(head).chain(BEDSIDE.into_iter().zip(spot)) {
            set(&mut fields, key, Value::Int(value));
        }
        set(&mut fields, SPAWN_DIMENSION, Value::Int(if self.bed.is_some() { self.spawn_dimension } else { NO_SPAWN_DIMENSION }));
        if let Some((at, dimension)) = self.last_death {
            for (key, value) in DEATH.into_iter().zip(at) {
                set(&mut fields, key, Value::Int(value));
            }
            set(&mut fields, DEATH_DIMENSION, Value::Int(dimension));
            set(&mut fields, HAS_DIED, Value::Byte(1));
        }
        let mut out = BytesMut::new();
        nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: Value::Compound(fields) });
        Ok(out.to_vec())
    }
}
