//! Custom blocks in the shard: their runtime ids follow the vanilla registry's
//! (sapling-leveldb `custom_states.rs`), so where a vanilla state comes from the registry, a custom
//! one comes from here, with what the plugin said of it (`sapling_data::custom_block`).

use sapling_api::{Block, BlockDrop, CustomBlock, Item};
use sapling_world::{BlockRegistry, CustomBlockStates, Material, Mining, custom_ids, custom_state, register_custom_block};

/// Seconds a custom block takes to break by hand when its plugin says nothing.
const DEFAULT_SECONDS: f32 = 0.75;
/// Ticks of a break per hardness by hand (mining.rs: hardness × 30 when the hand harvests).
const SECONDS_PER_HARDNESS: f32 = 1.5;

/// Registers a plugin's block for the whole process: runtime ids after the vanilla ones, its
/// StartGame entry and its item. False for a vanilla name, which is left out.
pub fn register(block: &CustomBlock) -> bool {
    if block.identifier.starts_with("minecraft:") || BlockRegistry::vanilla().states_of(&block.identifier).next().is_some() {
        return false;
    }
    let states = sapling_data::block_states(block);
    let solid = block.collision.is_some();
    let ids = register_custom_block(CustomBlockStates { name: block.identifier.clone(), states, light_emission: block.light_emission, light_filter: block.light_dampening, solid });
    let Some(first) = custom_state(ids.start) else { return false };
    sapling_data::register_block(block, first.block_id);
    sapling_data::register_block_item(&block.identifier, &block.display_name, first.block_id, first.hash, block.category, &block.group);
    true
}

/// The mining of the block `id` as a break needs it; `None` for air, a liquid or an unknown id.
pub(crate) fn breakable(registry: &BlockRegistry, id: u32) -> Option<Mining> {
    match registry.get(id) {
        Some(state) => (!state.is_air() && !state.is_liquid()).then_some(state.mining),
        None => custom_state(id).map(|state| mining(state.name)),
    }
}

/// A custom block's mining: `destructible_by_mining`'s seconds by hand, which the hand harvests.
fn mining(name: &str) -> Mining {
    let seconds = sapling_data::custom_block(name).map_or(Some(DEFAULT_SECONDS), |block| block.break_seconds);
    Mining { hardness: seconds.map_or(-1.0, |seconds| seconds / SECONDS_PER_HARDNESS), material: Material::Default, harvest_tools: 0, min_level: 0 }
}

/// The name of the block `id` when it is something a click lands on (not air).
pub(crate) fn clicked_name(registry: &BlockRegistry, id: u32) -> Option<&'static str> {
    match registry.get(id) {
        Some(state) => (!state.is_air()).then_some(state.name),
        None => custom_state(id).map(|state| state.name),
    }
}

/// The custom block the item `name` places: the block's first state.
pub(crate) fn placed_by(name: &str) -> Option<u32> {
    if name.starts_with("minecraft:") {
        return None;
    }
    custom_ids(name).next()
}

/// The custom state `id` as plugins see it.
pub(crate) fn described(id: u32) -> Option<Block> {
    custom_state(id).map(|state| Block { name: state.name.to_owned(), properties: state.properties.clone() })
}

/// The state of the custom block `block` names: its properties, or its first state without any.
pub(crate) fn resolved(block: &Block) -> Option<u32> {
    let mut ids = custom_ids(&block.name);
    let first = ids.clone().next()?;
    if block.properties.is_empty() {
        return Some(first);
    }
    let mut wanted: Vec<&str> = block.properties.split(',').collect();
    wanted.sort_unstable();
    let wanted = wanted.join(",");
    ids.find(|&id| custom_state(id).is_some_and(|state| state.properties == wanted))
}

/// What the custom block `name` leaves when broken; `None` for a vanilla block.
pub(crate) fn drops(name: &str) -> Option<Vec<Item>> {
    let block = sapling_data::custom_block(name)?;
    Some(match &block.drops {
        BlockDrop::Itself => vec![Item::new(name, 1)],
        BlockDrop::Nothing => Vec::new(),
        BlockDrop::Items(items) => items.iter().map(|(item, count)| Item::new(item.clone(), *count)).collect(),
    })
}

/// The vanilla state the server's physics reads a custom one as: stone where it stops entities,
/// air where it does not.
// TODO: a custom collision box other than a full cube is read as a full cube.
pub(crate) fn stand_in(registry: &BlockRegistry, id: u32) -> u32 {
    static STONE: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    match custom_state(id) {
        Some(state) if state.solid => *STONE.get_or_init(|| registry.find("minecraft:stone", "").unwrap_or(registry.air_id())),
        Some(_) => registry.air_id(),
        None => id,
    }
}
