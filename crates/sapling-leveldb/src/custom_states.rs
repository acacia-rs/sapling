//! Custom block states: runtime ids past the vanilla registry's end, so that no vanilla id moves.
//! Chunks travel with network hashes (`block_network_ids_are_hashes`), which a client computes for
//! custom states as for vanilla ones: FNV-1a over the `{name, states}` compound, dragonfly's
//! `networkBlockHash`. Registration is for the whole process and only adds
//! (docs/research/custom-content.md).

use std::ops::Range;
use std::sync::{Arc, RwLock};

use acacia_proto::nbt::{Str, Value};
use acacia_world::BlockRegistry;

use crate::states::{network_hash, properties};

/// One state of a custom block.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomState {
    pub name: &'static str,
    /// The typed `states` compound, in the block's own order.
    pub states: Value,
    /// `key=value` sorted, as the registry spells vanilla states.
    pub properties: String,
    pub hash: u32,
    /// `vanilla_block_data.block_id`: 10000 up, one per block.
    pub block_id: i32,
    pub light_emission: u8,
    pub light_filter: u8,
    /// Whether it stops entities: the server's physics reads it as stone, else as air.
    pub solid: bool,
}

/// A custom block to register: its name, every state compound and how it lights and stands.
pub struct CustomBlockStates {
    pub name: String,
    pub states: Vec<Value>,
    pub light_emission: u8,
    pub light_filter: u8,
    pub solid: bool,
}

/// The first number BDS gives a custom block (dragonfly's `10000+i`).
const FIRST_BLOCK_ID: i32 = 10_000;

static STATES: RwLock<Vec<Arc<CustomState>>> = RwLock::new(Vec::new());

/// The first custom runtime id: the vanilla registry's length.
pub fn first_custom_id() -> u32 {
    BlockRegistry::vanilla().len() as u32
}

/// Registers `block`; the runtime ids of its states. A name registered before keeps its ids.
pub fn register_custom_block(block: CustomBlockStates) -> Range<u32> {
    let base = first_custom_id();
    let mut states = STATES.write().expect("custom states lock");
    if let Some(at) = states.iter().position(|state| state.name == block.name) {
        let count = states.iter().filter(|state| state.name == block.name).count() as u32;
        return base + at as u32..base + at as u32 + count;
    }
    let name: &'static str = Box::leak(block.name.into_boxed_str());
    let block_id = FIRST_BLOCK_ID + states.iter().map(|state| state.block_id - FIRST_BLOCK_ID + 1).max().unwrap_or(0);
    let start = base + states.len() as u32;
    for compound in block.states {
        let entry = Value::Compound(vec![(Str::from("name"), Value::String(Str::from(name))), (Str::from("states"), compound.clone())]);
        let hash = network_hash(&entry).expect("a name and states");
        let state = CustomState { name, properties: properties(&compound), states: compound, hash, block_id, light_emission: block.light_emission, light_filter: block.light_filter, solid: block.solid };
        states.push(Arc::new(state));
    }
    start..base + states.len() as u32
}

/// The custom state of runtime id `id`; `None` for a vanilla or unknown id.
pub fn custom_state(id: u32) -> Option<Arc<CustomState>> {
    let index = id.checked_sub(first_custom_id())?;
    STATES.read().expect("custom states lock").get(index as usize).cloned()
}

/// Every custom state, in runtime id order from [`first_custom_id`].
pub fn custom_states() -> Vec<Arc<CustomState>> {
    STATES.read().expect("custom states lock").clone()
}

/// The runtime ids of the custom block `name`; empty for another name.
pub fn custom_ids(name: &str) -> Range<u32> {
    let states = STATES.read().expect("custom states lock");
    let Some(first) = states.iter().position(|state| state.name == name) else { return 0..0 };
    let count = states[first..].iter().take_while(|state| state.name == name).count();
    let base = first_custom_id() + first as u32;
    base..base + count as u32
}

/// The runtime id of the custom state with network hash `hash`.
pub fn custom_id_of_hash(hash: u32) -> Option<u32> {
    let states = STATES.read().expect("custom states lock");
    states.iter().position(|state| state.hash == hash).map(|at| first_custom_id() + at as u32)
}

/// The custom state `name` with `properties` (`key=value` sorted).
pub(crate) fn custom_id_of(name: &str, properties: &str) -> Option<u32> {
    custom_ids(name).find(|&id| custom_state(id).is_some_and(|state| state.properties == properties))
}

/// The network hash of runtime id `id`, custom states included; 0 for an unknown id.
pub fn block_hash(registry: &BlockRegistry, id: u32) -> u32 {
    match registry.get(id) {
        Some(state) => state.network_hash,
        None => custom_state(id).map_or(0, |state| state.hash),
    }
}

/// The runtime id a network hash names, custom states included.
pub fn id_of_hash(registry: &BlockRegistry, hash: u32) -> Option<u32> {
    registry.runtime_id_from_hash(hash).or_else(|| custom_id_of_hash(hash))
}

/// The name of the block of runtime id `id`, custom ones included.
pub fn block_name(registry: &BlockRegistry, id: u32) -> Option<&'static str> {
    registry.get(id).map(|state| state.name).or_else(|| custom_state(id).map(|state| state.name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_states_follow_the_vanilla_ids_and_hash_as_a_client_does() {
        let lit = |on: bool| Value::Compound(vec![(Str::from("test_states:lit"), Value::Byte(i8::from(on)))]);
        let block = || CustomBlockStates { name: "test_states:lamp".into(), states: vec![lit(false), lit(true)], light_emission: 0, light_filter: 15, solid: true };
        let ids = register_custom_block(block());
        assert_eq!(register_custom_block(block()), ids, "registered again, the same ids");
        assert!(ids.start >= first_custom_id() && ids.len() == 2);
        let registry = BlockRegistry::vanilla();
        let on = custom_state(ids.start + 1).unwrap();
        assert_eq!((on.name, on.properties.as_str(), on.block_id >= FIRST_BLOCK_ID), ("test_states:lamp", "test_states:lit=1", true));
        assert_eq!(id_of_hash(registry, on.hash), Some(ids.start + 1));
        assert_eq!(block_hash(registry, ids.start + 1), on.hash);
        assert_eq!(custom_id_of("test_states:lamp", "test_states:lit=0"), Some(ids.start));
        let saved = crate::BlockStates::new(BlockRegistry::vanilla_arc());
        let mut entry = Vec::new();
        saved.write(ids.start + 1, &mut entry).unwrap();
        assert_eq!(crate::BlockStates::new(BlockRegistry::vanilla_arc()).read(&mut &entry[..]).unwrap(), ids.start + 1, "saved and loaded by name");
        // A vanilla state's hash, made the same way, is the registry's.
        let stone = registry.find("minecraft:stone", "").unwrap();
        let entry = Value::Compound(vec![(Str::from("name"), Value::String(Str::from("minecraft:stone"))), (Str::from("states"), Value::Compound(Vec::new()))]);
        assert_eq!(network_hash(&entry), Some(registry.network_hash(stone)));
    }
}
