//! Disk palette entries (`{name, states, version}` in little-endian NBT) to runtime ids.
//!
//! Entries saved by an older game version need the block-state upgrader (docs/DESIGN.md open
//! items). Until it exists: a known block with unknown states becomes that block's first state,
//! an unknown block becomes [`PLACEHOLDER`]; each is logged once.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use acacia_proto::nbt::{self, LittleEndian, Nbt, Value};
use acacia_world::{BlockRegistry, BlockState};
use bytes::BytesMut;

use crate::Error;

/// The block vanilla itself shows for blocks it does not know.
pub const PLACEHOLDER: &str = "minecraft:info_update";
/// The `version` of written entries (1.21.60.33); tests/world.rs holds it to what BDS saves.
const STATE_VERSION: i32 = 18_168_865;

pub struct BlockStates {
    registry: Arc<BlockRegistry>,
    by_name: HashMap<&'static str, Vec<u32>>,
    placeholder: u32,
    /// Raw entry bytes to runtime id: a world repeats a few hundred entries in every palette.
    resolved: Mutex<HashMap<Box<[u8]>, u32>>,
    /// Runtime id to the entry [`BlockStates::write`] made for it.
    written: Mutex<HashMap<u32, Arc<[u8]>>>,
    reported: Mutex<HashSet<String>>,
}

impl BlockStates {
    pub fn new(registry: Arc<BlockRegistry>) -> Self {
        let mut by_name: HashMap<&'static str, Vec<u32>> = HashMap::new();
        for id in 0..registry.len() as u32 {
            by_name.entry(registry.get(id).expect("an id below the length").name).or_default().push(id);
        }
        let placeholder = by_name.get(PLACEHOLDER).map_or(registry.air_id(), |ids| ids[0]);
        Self { registry, by_name, placeholder, resolved: Mutex::default(), written: Mutex::default(), reported: Mutex::default() }
    }

    pub fn registry(&self) -> &Arc<BlockRegistry> {
        &self.registry
    }

    pub fn placeholder(&self) -> u32 {
        self.placeholder
    }

    /// Reads one palette entry off `r` and resolves it.
    pub fn read(&self, r: &mut &[u8]) -> Result<u32, Error> {
        let start = *r;
        nbt::skip::<LittleEndian>(r)?;
        let raw = &start[..start.len() - r.len()];
        if let Some(&id) = self.resolved.lock().expect("resolver lock").get(raw) {
            return Ok(id);
        }
        let entry = nbt::read::<LittleEndian>(&mut &raw[..])?.value;
        let id = self.resolve(&entry);
        self.resolved.lock().expect("resolver lock").insert(raw.into(), id);
        Ok(id)
    }

    /// The runtime id of a `{name, states}` compound.
    pub fn resolve(&self, entry: &Value) -> u32 {
        let Some(Value::String(name)) = entry.get("name") else {
            self.report("an entry without a name", "");
            return self.placeholder;
        };
        let Some(ids) = self.by_name.get(name.as_str()) else {
            self.report("unknown block", name);
            return self.placeholder;
        };
        let properties = entry.get("states").map(properties).unwrap_or_default();
        let exact = ids.iter().find(|&&id| self.registry.get(id).is_some_and(|s| s.properties == properties));
        *exact.unwrap_or_else(|| {
            self.report("unknown states, using the block's first state", &format!("{name}[{properties}]"));
            &ids[0]
        })
    }

    /// Appends the palette entry of runtime id `id`; an error when [`typed_states`] cannot type it.
    pub fn write(&self, id: u32, out: &mut Vec<u8>) -> Result<(), Error> {
        if let Some(entry) = self.written.lock().expect("writer lock").get(&id) {
            out.extend_from_slice(entry);
            return Ok(());
        }
        let state = self.registry.get(id).ok_or_else(|| Error::State(format!("runtime id {id}")))?;
        let states = typed_states(state).ok_or_else(|| Error::State(format!("{}[{}]", state.name, state.properties)))?;
        let entry: Arc<[u8]> = compound(vec![name_field(state), ("states".into(), states), ("version".into(), Value::Int(STATE_VERSION))]).into();
        out.extend_from_slice(&entry);
        self.written.lock().expect("writer lock").insert(id, entry);
        Ok(())
    }

    fn report(&self, what: &str, block: &str) {
        if self.reported.lock().expect("report lock").insert(block.to_string()) {
            tracing::warn!(block, "{what}");
        }
    }
}

fn name_field(state: &BlockState) -> (String, Value) {
    ("name".into(), Value::String(state.name.into()))
}

fn compound(fields: Vec<(String, Value)>) -> Vec<u8> {
    let mut out = BytesMut::new();
    nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: Value::Compound(fields) });
    out.to_vec()
}

fn fnv1a_32(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5, |hash: u32, &byte| (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193))
}

/// The network hash of a stored `{name, states}` compound, whatever else it holds.
pub(crate) fn network_hash(block: &Value) -> Option<u32> {
    let (name, states) = (block.get("name")?.clone(), block.get("states")?.clone());
    Some(fnv1a_32(&compound(vec![("name".into(), name), ("states".into(), states)])))
}

/// The `{name, states, version}` compound of the state a network hash names, as an item stack
/// stores the block it places.
pub(crate) fn block_of_hash(registry: &BlockRegistry, hash: u32) -> Option<Value> {
    let state = registry.get(registry.runtime_id_from_hash(hash)?)?;
    Some(Value::Compound(vec![name_field(state), ("states".into(), typed_states(state)?), ("version".into(), Value::Int(STATE_VERSION))]))
}

/// A state's values with their NBT types. The registry keeps them as text, where a number may be
/// a byte or an int; the network hash is over the typed `{name, states}`, so the typing that
/// reproduces it is the right one. `None` when none does; a state without numbers needs no hash
/// (`minecraft:unknown` and custom blocks have none to match).
fn typed_states(state: &BlockState) -> Option<Value> {
    let pairs: Vec<(&str, &str)> = state.properties.split(',').filter_map(|pair| pair.split_once('=')).collect();
    let numbers = pairs.iter().filter(|(_, value)| value.parse::<i32>().is_ok()).count();
    if numbers == 0 {
        return Some(Value::Compound(pairs.iter().map(|(key, text)| (key.to_string(), Value::String(text.to_string()))).collect()));
    }
    (0..1u32 << numbers).find_map(|ints| {
        let mut number = 0;
        let mut typed = Vec::with_capacity(pairs.len());
        for (key, text) in &pairs {
            let value = match text.parse::<i32>() {
                Err(_) => Value::String(text.to_string()),
                Ok(int) => {
                    number += 1;
                    if ints >> (number - 1) & 1 == 1 { Value::Int(int) } else { Value::Byte(i8::try_from(int).ok()?) }
                }
            };
            typed.push((key.to_string(), value));
        }
        let states = Value::Compound(typed);
        (fnv1a_32(&compound(vec![name_field(state), ("states".into(), states.clone())])) == state.network_hash).then_some(states)
    })
}

/// The registry's spelling of a state compound: `key=value` sorted by key, booleans as 0/1.
fn properties(states: &Value) -> String {
    let Value::Compound(entries) = states else { return String::new() };
    let text = |value: &Value| match value {
        Value::Byte(v) => v.to_string(),
        Value::Short(v) => v.to_string(),
        Value::Int(v) => v.to_string(),
        Value::String(v) => v.clone(),
        other => format!("{other:?}"),
    };
    let mut pairs: Vec<String> = entries.iter().map(|(key, value)| format!("{key}={}", text(value))).collect();
    pairs.sort_unstable();
    pairs.join(",")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn entry(name: &str, states: &[(&str, Value)]) -> Vec<u8> {
        let states = states.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
        compound(vec![
            ("name".to_string(), Value::String(name.into())),
            ("states".to_string(), Value::Compound(states)),
            ("version".to_string(), Value::Int(STATE_VERSION)),
        ])
    }

    #[test]
    fn every_vanilla_state_is_written_and_read_back_as_itself() {
        let states = BlockStates::new(BlockRegistry::vanilla_arc());
        for id in 0..states.registry().len() as u32 {
            let mut entry = Vec::new();
            states.write(id, &mut entry).unwrap_or_else(|error| panic!("{error}"));
            assert_eq!(states.read(&mut &entry[..]).unwrap(), id, "{:?}", states.registry().get(id));
        }
        assert!(states.reported.lock().unwrap().is_empty(), "nothing fell back to a placeholder");
    }

    #[test]
    fn entries_resolve_by_name_and_states() {
        let states = BlockStates::new(BlockRegistry::vanilla_arc());
        let registry = states.registry().clone();
        let read = |bytes: Vec<u8>| {
            let mut r = &bytes[..];
            let id = states.read(&mut r).unwrap();
            assert!(r.is_empty(), "the whole entry is consumed");
            id
        };
        assert_eq!(read(entry("minecraft:air", &[])), registry.air_id());
        assert_eq!(read(entry("minecraft:stone", &[])), registry.find("minecraft:stone", "").unwrap());

        let (id, state) = registry.states_of("minecraft:oak_log").last().unwrap();
        let axis = state.property("pillar_axis").unwrap();
        assert_eq!(read(entry("minecraft:oak_log", &[("pillar_axis", Value::String(axis.into()))])), id);
        // The second read is the cached one.
        assert_eq!(read(entry("minecraft:oak_log", &[("pillar_axis", Value::String(axis.into()))])), id);
    }

    #[test]
    fn what_does_not_resolve_stays_visible() {
        let states = BlockStates::new(BlockRegistry::vanilla_arc());
        let registry = states.registry();
        assert_eq!(registry.get(states.placeholder()).unwrap().name, PLACEHOLDER);
        let read = |bytes: Vec<u8>| states.read(&mut &bytes[..]).unwrap();
        assert_eq!(read(entry("mod:gizmo", &[])), states.placeholder());
        let first_log = registry.states_of("minecraft:oak_log").next().unwrap().0;
        assert_eq!(read(entry("minecraft:oak_log", &[("old_axis", Value::Int(2))])), first_log);
        assert!(states.read(&mut &entry("minecraft:stone", &[])[..5]).is_err());
    }
}
