//! Disk palette entries (`{name, states, version}` in little-endian NBT) to runtime ids.
//!
//! Entries saved by an older game version need the block-state upgrader (docs/DESIGN.md open
//! items). Until it exists: a known block with unknown states becomes that block's first state,
//! an unknown block becomes [`PLACEHOLDER`]; each is logged once.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use acacia_proto::nbt::{self, LittleEndian, Value};
use acacia_world::BlockRegistry;

use crate::Error;

/// The block vanilla itself shows for blocks it does not know.
pub const PLACEHOLDER: &str = "minecraft:info_update";

pub struct BlockStates {
    registry: Arc<BlockRegistry>,
    by_name: HashMap<&'static str, Vec<u32>>,
    placeholder: u32,
    /// Raw entry bytes to runtime id: a world repeats a few hundred entries in every palette.
    resolved: Mutex<HashMap<Box<[u8]>, u32>>,
    reported: Mutex<HashSet<String>>,
}

impl BlockStates {
    pub fn new(registry: Arc<BlockRegistry>) -> Self {
        let mut by_name: HashMap<&'static str, Vec<u32>> = HashMap::new();
        for id in 0..registry.len() as u32 {
            by_name.entry(registry.get(id).expect("an id below the length").name).or_default().push(id);
        }
        let placeholder = by_name.get(PLACEHOLDER).map_or(registry.air_id(), |ids| ids[0]);
        Self { registry, by_name, placeholder, resolved: Mutex::default(), reported: Mutex::default() }
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

    fn report(&self, what: &str, block: &str) {
        if self.reported.lock().expect("report lock").insert(block.to_string()) {
            tracing::warn!(block, "{what}");
        }
    }
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
    use acacia_proto::nbt::Nbt;
    use bytes::BytesMut;

    use super::*;

    pub fn entry(name: &str, states: &[(&str, Value)]) -> Vec<u8> {
        let states = states.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
        let fields = vec![
            ("name".to_string(), Value::String(name.into())),
            ("states".to_string(), Value::Compound(states)),
            ("version".to_string(), Value::Int(18_168_865)),
        ];
        let mut out = BytesMut::new();
        nbt::write::<LittleEndian>(&mut out, &Nbt { name: String::new(), value: Value::Compound(fields) });
        out.to_vec()
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
