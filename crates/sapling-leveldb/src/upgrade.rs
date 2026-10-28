//! Old block states made current, as BDS loads an old world: a `{name, states, version}` entry
//! through every upgrade schema from its version on, and a pre-1.13 numeric id or `{name, val}`
//! through its 1.12 state first. The data is pmmp/BedrockBlockUpgradeSchema's (CC0, generated from
//! BDS's own upgrades; `data/upgrade/`): see docs/research/crafting-storage.md "Old storage".

mod schema;

use std::collections::HashMap;
use std::sync::LazyLock;

use acacia_proto::codec::{read_varint, take};
use acacia_proto::nbt::{self, LittleEndian, Value};

pub use schema::States;
use schema::Schema;

macro_rules! schemas {
    ($($file:literal),* $(,)?) => { [$(include_str!(concat!("../data/upgrade/schemas/", $file, ".json"))),*] };
}

/// In version order, as the file names' numbers sort them.
const SCHEMAS: [&str; 37] = schemas![
    "0001_1.9.0_to_1.10.0", "0011_1.10.0_to_1.12.0", "0021_1.12.0_to_1.13.0", "0031_1.13.0_to_1.14.0", "0041_1.14.0_to_1.16.0.57_beta",
    "0051_1.16.0.57_beta_to_1.16.0.59_beta", "0061_1.16.0.59_beta_to_1.16.0.68_beta", "0071_1.16.0_to_1.16.100", "0081_1.16.200_to_1.16.210",
    "0091_1.17.10_to_1.17.30", "0101_1.17.30_to_1.17.40", "0111_1.18.0_to_1.18.10", "0121_1.18.10_to_1.18.20.27_beta", "0131_1.18.20.27_beta_to_1.18.30",
    "0141_1.18.30_to_1.19.0.34_beta", "0151_1.19.0.34_beta_to_1.19.20", "0161_1.19.50_to_1.19.60.26_beta", "0171_1.19.60_to_1.19.70.26_beta",
    "0181_1.19.70_to_1.19.80.24_beta", "0191_1.19.80.24_beta_to_1.20.0.23_beta", "0201_1.20.0.23_beta_to_1.20.10.24_beta",
    "0211_1.20.10.24_beta_to_1.20.20.23_beta", "0221_1.20.20.23_beta_to_1.20.30.22_beta", "0231_1.20.30.22_beta_to_1.20.40.24_beta",
    "0241_1.20.40.24_beta_to_1.20.50.23_beta", "0251_1.20.50.23_beta_to_1.20.60.26_beta", "0261_1.20.60.26_beta_to_1.20.70.24_beta",
    "0271_1.20.70.24_beta_to_1.20.80.24_beta", "0281_1.20.80.24_beta_to_1.21.0.25_beta", "0291_1.21.0.25_beta_to_1.21.20.24_beta",
    "0301_1.21.20.24_beta_to_1.21.30.24_beta", "0311_1.21.30.24_beta_to_1.21.40.25_beta", "0321_1.21.50.29_beta_to_1.21.60.28_beta",
    "0331_1.21.100.23_beta_to_1.21.110.26_beta", "0341_1.26.20_to_1.26.30", "0351_1.26.40_to_1.26.50", "0361_1.26.50_to_1.26.60",
];
const LEGACY_IDS: &str = include_str!("../data/upgrade/block_legacy_id_map.json");
/// Every 1.12 id and meta pair's state: `varint count {varint length, name, varint count {varint meta, LE NBT}}`.
const LEGACY_STATES: &[u8] = include_bytes!("../data/upgrade/id_meta_1.12.0.bin");
/// The first `version` of entries that carry `states`; before it they carry `val` (1.14.0.3).
pub const FIRST_STATES_VERSION: i32 = 17_694_723;

static UPGRADES: LazyLock<Vec<Schema>> = LazyLock::new(|| SCHEMAS.iter().map(|text| Schema::parse(text).expect("an embedded upgrade schema")).collect());
static NAMES: LazyLock<HashMap<u16, String>> = LazyLock::new(|| {
    let ids: HashMap<String, u16> = serde_json::from_str(LEGACY_IDS).expect("the embedded legacy id map");
    ids.into_iter().map(|(name, id)| (id, name)).collect()
});
type Legacy = HashMap<(String, i16), (String, States, i32)>;
static LEGACY: LazyLock<Legacy> = LazyLock::new(|| legacy_states(LEGACY_STATES).expect("the embedded 1.12 states"));

/// A compound's fields as states.
pub fn states_of(value: Option<&Value>) -> States {
    match value {
        Some(Value::Compound(fields)) => fields.iter().map(|(key, value)| (key.to_string(), value.clone())).collect(),
        _ => States::new(),
    }
}

fn legacy_states(mut r: &[u8]) -> Option<Legacy> {
    let r = &mut r;
    let mut out = HashMap::new();
    for _ in 0..read_varint(r).ok()? {
        let length = read_varint(r).ok()? as usize;
        let name = std::str::from_utf8(take(r, length).ok()?).ok()?.to_owned();
        for _ in 0..read_varint(r).ok()? {
            let meta = read_varint(r).ok()? as i16;
            let state = nbt::read::<LittleEndian>(r).ok()?.value;
            let Some(Value::String(current)) = state.get("name") else { return None };
            let version = match state.get("version") {
                Some(Value::Int(version)) => *version,
                _ => 0,
            };
            out.insert((name.clone(), meta), (current.to_string(), states_of(state.get("states")), version));
        }
    }
    Some(out)
}

/// The state `(name, states)` saved at `version`, as the newest schema has it.
pub fn upgraded(name: &str, states: States, version: i32) -> (String, States) {
    UPGRADES.iter().filter(|schema| schema.id >= version).fold((name.to_owned(), states), |(name, states), schema| schema.apply(&name, &states).unwrap_or((name, states)))
}

/// The current state of a pre-1.13 block `name` with `meta`; meta 0's when the meta is unknown.
pub fn from_legacy(name: &str, meta: i16) -> Option<(String, States)> {
    let (current, states, version) = LEGACY.get(&(name.to_owned(), meta)).or_else(|| LEGACY.get(&(name.to_owned(), 0)))?;
    Some(upgraded(current, states.clone(), *version))
}

/// The name of pre-1.13 numeric block id `id`.
pub fn legacy_name(id: u16) -> Option<&'static str> {
    NAMES.get(&id).map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string(text: &str) -> Value {
        Value::String(text.into())
    }

    #[test]
    fn every_schema_parses_in_version_order() {
        assert_eq!(UPGRADES.len(), SCHEMAS.len());
        assert!(UPGRADES.windows(2).all(|pair| pair[0].id <= pair[1].id));
    }

    #[test]
    fn old_states_take_their_current_names_and_properties() {
        // 1.20.70 split wood by type into its own blocks.
        let wood = States::from([("wood_type".to_owned(), string("birch")), ("stripped_bit".to_owned(), Value::Byte(0)), ("pillar_axis".to_owned(), string("y"))]);
        let (name, states) = upgraded("minecraft:wood", wood, 17_959_425);
        assert_eq!((name.as_str(), states.get("pillar_axis")), ("minecraft:birch_wood", Some(&string("y"))));
        assert!(!states.contains_key("wood_type"));
        let current = States::from([("pillar_axis".to_owned(), string("x"))]);
        assert_eq!(upgraded("minecraft:birch_log", current.clone(), 18_168_865), ("minecraft:birch_log".to_owned(), current));
    }

    #[test]
    fn numeric_ids_and_metas_name_current_states() {
        assert_eq!(legacy_name(1), Some("minecraft:stone"));
        let (name, _) = from_legacy("minecraft:stone", 1).expect("granite");
        assert_eq!(name, "minecraft:granite");
        let (name, states) = from_legacy("minecraft:log", 2).expect("a birch log");
        assert_eq!((name.as_str(), states.get("pillar_axis")), ("minecraft:birch_log", Some(&string("y"))));
        assert_eq!(from_legacy("minecraft:stone", 99).map(|(name, _)| name), Some("minecraft:stone".to_owned()), "an unknown meta is meta 0's");
    }
}
