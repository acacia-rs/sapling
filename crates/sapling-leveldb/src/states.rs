//! Disk palette entries (`{name, states, version}` in little-endian NBT) to runtime ids.
//!
//! An entry that names no current state goes through the upgrader (upgrade.rs) first; only an
//! exact miss is upgraded, since BDS saves today's states under an older `version`. What still
//! does not resolve: a known block becomes its first state, an unknown block [`PLACEHOLDER`];
//! each is logged once.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use acacia_proto::nbt::{self, LittleEndian, Nbt, Str, Value};
use acacia_world::{BlockRegistry, BlockState};
use bytes::BytesMut;

use crate::Error;
use crate::custom_states;
use crate::upgrade::{self, States};

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
    /// Pre-1.13 numeric id and meta to runtime id.
    legacy: Mutex<HashMap<(u8, u8), u32>>,
    reported: Mutex<HashSet<String>>,
}

impl BlockStates {
    pub fn new(registry: Arc<BlockRegistry>) -> Self {
        let mut by_name: HashMap<&'static str, Vec<u32>> = HashMap::new();
        for id in 0..registry.len() as u32 {
            by_name.entry(registry.get(id).expect("an id below the length").name).or_default().push(id);
        }
        let placeholder = by_name.get(PLACEHOLDER).map_or(registry.air_id(), |ids| ids[0]);
        Self { registry, by_name, placeholder, resolved: Mutex::default(), written: Mutex::default(), legacy: Mutex::default(), reported: Mutex::default() }
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

    /// The runtime id of a `{name, states, version}` compound, or of a pre-1.13 `{name, val}`.
    pub fn resolve(&self, entry: &Value) -> u32 {
        let Some(Value::String(name)) = entry.get("name") else {
            self.report("an entry without a name", "");
            return self.placeholder;
        };
        let version = match entry.get("version") {
            Some(Value::Int(version)) => *version,
            _ => 0,
        };
        let states = upgrade::states_of(entry.get("states"));
        let meta = match entry.get("val") {
            Some(Value::Short(meta)) if version < upgrade::FIRST_STATES_VERSION => Some(*meta),
            _ => None,
        };
        if let Some(id) = meta.is_none().then(|| self.exact(name, &states)).flatten() {
            return id;
        }
        let upgraded = match meta {
            Some(meta) => upgrade::from_legacy(name, meta),
            None => Some(upgrade::upgraded(name, states.clone(), version)),
        };
        let (name, states) = upgraded.unwrap_or((name.to_string(), states));
        self.exact(&name, &states).unwrap_or_else(|| self.nearest(&name, &states))
    }

    /// The runtime id of a pre-1.13 numeric block id and its meta, as legacy sub-chunks store them.
    pub fn resolve_legacy(&self, id: u8, meta: u8) -> u32 {
        if let Some(&resolved) = self.legacy.lock().expect("legacy lock").get(&(id, meta)) {
            return resolved;
        }
        let state = upgrade::legacy_name(id.into()).and_then(|name| upgrade::from_legacy(name, meta.into()));
        let resolved = match state {
            Some((name, states)) => self.exact(&name, &states).unwrap_or_else(|| self.nearest(&name, &states)),
            None => {
                self.report("unknown numeric block", &format!("{id}:{meta}"));
                self.placeholder
            }
        };
        self.legacy.lock().expect("legacy lock").insert((id, meta), resolved);
        resolved
    }

    fn exact(&self, name: &str, states: &States) -> Option<u32> {
        let properties = properties(&compound_of(states));
        let Some(ids) = self.by_name.get(name) else { return custom_states::custom_id_of(name, &properties) };
        ids.iter().copied().find(|&id| self.registry.get(id).is_some_and(|state| state.properties == properties))
    }

    /// The block's first state, or [`PLACEHOLDER`] for an unknown block; logged.
    fn nearest(&self, name: &str, states: &States) -> u32 {
        if let Some(first) = custom_states::custom_ids(name).next() {
            return first;
        }
        let Some(ids) = self.by_name.get(name) else {
            self.report("unknown block", name);
            return self.placeholder;
        };
        self.report("unknown states, using the block's first state", &format!("{name}[{}]", properties(&compound_of(states))));
        ids[0]
    }

    /// Appends the palette entry of runtime id `id`; an error when [`typed_states`] cannot type it.
    pub fn write(&self, id: u32, out: &mut Vec<u8>) -> Result<(), Error> {
        if let Some(entry) = self.written.lock().expect("writer lock").get(&id) {
            out.extend_from_slice(entry);
            return Ok(());
        }
        let (name, states) = match self.registry.get(id) {
            Some(state) => (name_field(state), typed_states(state).ok_or_else(|| Error::State(format!("{}[{}]", state.name, state.properties)))?),
            None => {
                let custom = custom_states::custom_state(id).ok_or_else(|| Error::State(format!("runtime id {id}")))?;
                (("name".into(), Value::String(custom.name.into())), custom.states.clone())
            }
        };
        let entry: Arc<[u8]> = compound(vec![name, ("states".into(), states), ("version".into(), Value::Int(STATE_VERSION))]).into();
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

fn compound_of(states: &States) -> Value {
    Value::Compound(states.iter().map(|(key, value)| (Str::from(key.as_str()), value.clone())).collect())
}

fn name_field(state: &BlockState) -> (Str, Value) {
    ("name".into(), Value::String(state.name.into()))
}

fn compound(fields: Vec<(Str, Value)>) -> Vec<u8> {
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
pub fn block_of_hash(registry: &BlockRegistry, hash: u32) -> Option<Value> {
    let Some(state) = registry.runtime_id_from_hash(hash).and_then(|id| registry.get(id)) else {
        let custom = custom_states::custom_state(custom_states::custom_id_of_hash(hash)?)?;
        return Some(Value::Compound(vec![("name".into(), Value::String(custom.name.into())), ("states".into(), custom.states.clone()), ("version".into(), Value::Int(STATE_VERSION))]));
    };
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
        return Some(Value::Compound(pairs.iter().map(|(key, text)| (Str::from(*key), Value::String(Str::from(*text)))).collect()));
    }
    (0..1u32 << numbers).find_map(|ints| {
        let mut number = 0;
        let mut typed = Vec::with_capacity(pairs.len());
        for (key, text) in &pairs {
            let value = match text.parse::<i32>() {
                Err(_) => Value::String(Str::from(*text)),
                Ok(int) => {
                    number += 1;
                    if ints >> (number - 1) & 1 == 1 { Value::Int(int) } else { Value::Byte(i8::try_from(int).ok()?) }
                }
            };
            typed.push((Str::from(*key), value));
        }
        let states = Value::Compound(typed);
        (fnv1a_32(&compound(vec![name_field(state), ("states".into(), states.clone())])) == state.network_hash).then_some(states)
    })
}

/// The registry's spelling of a state compound: `key=value` sorted by key, booleans as 0/1.
/// A palette entry's block name and its states as the registry writes them (`key=value`, sorted).
pub fn palette_block(entry: &Value) -> Option<(String, String)> {
    let Some(Value::String(name)) = entry.get("name") else { return None };
    Some((name.to_string(), entry.get("states").map(properties).unwrap_or_default()))
}

pub(crate) fn properties(states: &Value) -> String {
    let Value::Compound(entries) = states else { return String::new() };
    let text = |value: &Value| match value {
        Value::Byte(v) => v.to_string(),
        Value::Short(v) => v.to_string(),
        Value::Int(v) => v.to_string(),
        Value::String(v) => v.to_string(),
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
        let states = states.iter().map(|(k, v)| (Str::from(*k), v.clone())).collect();
        compound(vec![
            ("name".into(), Value::String(name.into())),
            ("states".into(), Value::Compound(states)),
            ("version".into(), Value::Int(STATE_VERSION)),
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
    fn old_entries_resolve_through_the_upgrader() {
        let states = BlockStates::new(BlockRegistry::vanilla_arc());
        let registry = states.registry().clone();
        let old = |fields: Vec<(&str, Value)>| Value::Compound(fields.into_iter().map(|(key, value)| (Str::from(key), value)).collect());
        let wood_states = old(vec![("wood_type", Value::String("birch".into())), ("stripped_bit", Value::Byte(1)), ("pillar_axis", Value::String("x".into()))]);
        let wood = old(vec![("name", Value::String("minecraft:wood".into())), ("states", wood_states), ("version", Value::Int(17_959_425))]);
        assert_eq!(states.resolve(&wood), registry.find("minecraft:stripped_birch_wood", "pillar_axis=x").unwrap());
        let granite = registry.find("minecraft:granite", "").unwrap();
        assert_eq!(states.resolve(&old(vec![("name", Value::String("minecraft:stone".into())), ("val", Value::Short(1))])), granite);
        assert_eq!(states.resolve_legacy(1, 1), granite);
        assert_eq!(states.resolve_legacy(0, 0), registry.air_id());
        assert!(states.reported.lock().unwrap().is_empty(), "{:?}", states.reported.lock().unwrap());
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
