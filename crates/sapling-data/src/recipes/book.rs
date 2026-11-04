//! The recipe book `/recipe` gives and takes from: every crafting recipe's id (CraftingData.bin)
//! and those a new player knows (UnlockedRecipes.bin), as BDS lists them.

use std::sync::LazyLock;

use acacia_proto::packets::{CraftingData, UnlockedRecipes};
use acacia_proto::types::RecipeUnlockingRequirement;

use crate::templates::{CRAFTING_DATA, UNLOCKED_RECIPES, decode};

/// Every recipe's id, and whether `/recipe give *` gives it: BDS's 4092 left out the 540 that
/// unlock by an empty list of ingredients (research/captures/bds-t1drecipe.txt).
static IDS: LazyLock<Vec<(String, bool)>> = LazyLock::new(|| {
    crate::set_shield_item_id();
    let data = decode::<CraftingData>(CRAFTING_DATA);
    let given = |requirement: &Option<RecipeUnlockingRequirement>| !requirement.as_ref().is_some_and(|requirement| requirement.ingredients.as_ref().is_some_and(Vec::is_empty));
    let shaped = data.shaped_recipes.into_iter().chain(data.shaped_chemistry_recipes).map(|recipe| (recipe.recipe_id, given(&recipe.unlocking_requirement)));
    let shapeless = data.shapeless_recipes.into_iter().chain(data.shulker_box_recipes).chain(data.shapeless_chemistry_recipes).map(|recipe| (recipe.recipe_id, given(&recipe.unlocking_requirement)));
    let mut ids: Vec<(String, bool)> = shaped.chain(shapeless).filter(|(id, _)| !id.is_empty()).collect();
    ids.dedup_by(|a, b| a.0 == b.0);
    ids
});
static ID_NAMES: LazyLock<Vec<String>> = LazyLock::new(|| IDS.iter().map(|(id, _)| id.clone()).collect());
static GIVEN: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut seen = std::collections::HashSet::new();
    IDS.iter().filter(|(id, given)| *given && seen.insert(id)).map(|(id, _)| id.clone()).collect()
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
    &ID_NAMES
}

/// The recipes `/recipe give @s *` gives, in [`recipe_ids`]' order.
pub fn given_recipe_ids() -> &'static [String] {
    &GIVEN
}

/// The recipes a new player knows, which `/recipe take` cannot take for good, in BDS's order.
pub fn initial_recipes() -> &'static [String] {
    &INITIAL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn give_all_gives_what_bds_gave() {
        let new = given_recipe_ids().iter().filter(|id| !initial_recipes().contains(id)).count();
        assert_eq!(new, 4092, "bds-t1drecipe.txt");
        assert!(recipe_ids().iter().any(|id| id == "minecraft:barrel") && !given_recipe_ids().iter().any(|id| id == "minecraft:barrel"));
    }

    #[test]
    fn the_book_has_bds_ids() {
        assert!(recipe_ids().iter().any(|id| id == "minecraft:stick"));
        assert!(recipe_ids().len() > 1000);
        let named: Vec<String> = RESENT_ORDER.iter().map(|id| format!("minecraft:{id}")).collect();
        assert_eq!(initial_recipes(), named.as_slice());
    }
}
