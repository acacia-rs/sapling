//! `LegacyStructureSettings`' wood replacement (placeInWorld 141eeb8d0, lists from the settings
//! constructor 141ef1240): a block of a wooden family keeps its states and changes wood when the
//! settings map its wood type (oak 0, spruce 1, birch 2, jungle 3, acacia 4, dark oak 5).

use super::named;
use crate::block::Block;

/// Every family BDS swaps, by wood type; the generator emits each member a template block uses.
pub const WOOD_FAMILIES: [[&str; 6]; 10] = [
    ["oak_log", "spruce_log", "birch_log", "jungle_log", "acacia_log", "dark_oak_log"],
    ["oak_slab", "spruce_slab", "birch_slab", "jungle_slab", "acacia_slab", "dark_oak_slab"],
    ["oak_double_slab", "spruce_double_slab", "birch_double_slab", "jungle_double_slab", "acacia_double_slab", "dark_oak_double_slab"],
    ["oak_planks", "spruce_planks", "birch_planks", "jungle_planks", "acacia_planks", "dark_oak_planks"],
    ["oak_fence", "spruce_fence", "birch_fence", "jungle_fence", "acacia_fence", "dark_oak_fence"],
    ["oak_wood", "spruce_wood", "birch_wood", "jungle_wood", "acacia_wood", "dark_oak_wood"],
    ["wooden_door", "spruce_door", "birch_door", "jungle_door", "acacia_door", "dark_oak_door"],
    ["trapdoor", "spruce_trapdoor", "birch_trapdoor", "jungle_trapdoor", "acacia_trapdoor", "dark_oak_trapdoor"],
    ["oak_stairs", "spruce_stairs", "birch_stairs", "jungle_stairs", "acacia_stairs", "dark_oak_stairs"],
    ["stripped_oak_log", "stripped_spruce_log", "stripped_birch_log", "stripped_jungle_log", "stripped_acacia_log", "stripped_dark_oak_log"],
];

/// `block` in the wood `remap` gives its wood type, unchanged when it is not wooden or unmapped.
pub fn rewood(block: Block, remap: &[Option<u8>; 6]) -> Block {
    let name = block.name();
    for family in &WOOD_FAMILIES {
        if let Some(wood) = family.iter().position(|&n| n == name) {
            return match remap[wood] {
                Some(to) => named(family[to as usize], block.states(), block.is_waterlogged()),
                None => block,
            };
        }
    }
    block
}
