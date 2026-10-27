//! The recipe book `/recipe` gives and takes from: every crafting recipe's id (CraftingData.bin)
//! and those a new player knows (UnlockedRecipes.bin), as BDS lists them.

use std::sync::LazyLock;

use acacia_proto::packets::{CraftingData, UnlockedRecipes};

use crate::templates::{CRAFTING_DATA, UNLOCKED_RECIPES, decode};

static IDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    crate::set_shield_item_id();
    let data = decode::<CraftingData>(CRAFTING_DATA);
    let shaped = data.shaped_recipes.into_iter().chain(data.shaped_chemistry_recipes).map(|recipe| recipe.recipe_id);
    let shapeless = data.shapeless_recipes.into_iter().chain(data.shulker_box_recipes).chain(data.shapeless_chemistry_recipes).map(|recipe| recipe.recipe_id);
    let mut ids: Vec<String> = shaped.chain(shapeless).filter(|id| !id.is_empty()).collect();
    ids.dedup();
    ids
});

/// The order BDS listed the starting recipes in after `/recipe take @s *` (bds-controls.txt),
/// not that of the list it sends on joining.
const RESENT_ORDER: [&str; 20] = [
    "WorkBench_recipeId_from_oak", "furnace_acacia_wood", "furnace_birch_wood", "furnace_dark_oak_wood", "furnace_jungle_wood", "furnace_log2_acacia", "furnace_log2_dark_oak",
    "furnace_log_birch", "furnace_log_jungle", "furnace_log_oak", "furnace_log_spruce", "furnace_oak_wood", "furnace_spruce_wood", "furnace_stripped_acacia_wood", "furnace_stripped_birch_wood",
    "furnace_stripped_dark_oak_wood", "furnace_stripped_jungle_wood", "furnace_stripped_oak_wood", "furnace_stripped_spruce_wood", "WorkBench_recipeId",
];

static INITIAL: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut ids = decode::<UnlockedRecipes>(UNLOCKED_RECIPES).recipes;
    let place = |id: &String| RESENT_ORDER.iter().position(|known| id.strip_prefix("minecraft:") == Some(*known)).unwrap_or(RESENT_ORDER.len());
    ids.sort_by_key(place);
    ids
});

/// Every crafting recipe's id (`minecraft:stick`), in BDS's order.
pub fn recipe_ids() -> &'static [String] {
    &IDS
}

/// The recipes a new player knows, which `/recipe take` cannot take for good, in BDS's order.
pub fn initial_recipes() -> &'static [String] {
    &INITIAL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_book_has_bds_ids() {
        assert!(recipe_ids().iter().any(|id| id == "minecraft:stick"));
        assert!(recipe_ids().len() > 1000);
        let named: Vec<String> = RESENT_ORDER.iter().map(|id| format!("minecraft:{id}")).collect();
        assert_eq!(initial_recipes(), named.as_slice());
    }
}
