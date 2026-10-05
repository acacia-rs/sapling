//! Items that place a block without being block items: a stack of them names no block (its
//! `block_runtime_id` is 0), and the game knows which block each one builds
//! (docs/research/block-rules.md, "Items whose block has another name").
// TODO: only those whose block needs nothing sapling lacks. Signs, skulls, banners, flower pots,
// cauldrons, brewing stands and campfires need their block entities; crops, reeds and redstone
// need their rules for where they may stand.

use sapling_world::BlockRegistry;

/// Items whose block carries their own name: every door, and these.
const SAME_NAME: &[&str] = &["minecraft:bed", "minecraft:hopper", "minecraft:cake"];

/// The block the item `name` places, in the state the item gives it before it is turned.
pub(super) fn block_of(registry: &BlockRegistry, name: &str) -> Option<u32> {
    let places = name.ends_with("_door") && !name.ends_with("trapdoor") || SAME_NAME.contains(&name);
    registry.states_of(name).next().filter(|_| places).map(|(id, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doors_beds_and_hoppers_place_the_block_of_their_name() {
        let registry = BlockRegistry::vanilla();
        let items = ["minecraft:wooden_door", "minecraft:spruce_door", "minecraft:iron_door", "minecraft:copper_door", "minecraft:bed", "minecraft:hopper", "minecraft:cake"];
        for item in items {
            assert!(sapling_data::is_item(item) && block_of(&registry, item).is_some(), "{item}");
        }
        assert_eq!([block_of(&registry, "minecraft:diamond"), block_of(&registry, "minecraft:stone"), block_of(&registry, "minecraft:oak_trapdoor")], [None; 3]);
    }
}
