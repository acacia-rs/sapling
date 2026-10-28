//! What unlocks a recipe in a player's recipe book: each recipe's unlocking requirement in
//! `CraftingData.bin` (BDS's `unlock` of its recipe file), read for BDS's RecipeUnlockingSystem
//! (research/captures/bds-recipeunlock.txt, bds-recipeunlock2.txt, bds-limitedcraft.txt).

use std::collections::HashSet;
use std::sync::LazyLock;

use acacia_proto::packets::CraftingData;
use acacia_proto::types::{RecipeUnlockingRequirement as Requirement, RecipeUnlockingRequirementContext as Context};

use super::{Ingredient, taken};
use crate::templates::{CRAFTING_DATA, decode};

/// A state of the player that unlocks recipes once, whatever it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnlockContext {
    InWater,
    /// Ten slots filled (the chest).
    ManyItems,
}

impl UnlockContext {
    /// Its bit in the `used_contexts` a player record keeps: BDS's context number as a bit
    /// (inferred from the field being an int of flags; no record read back from BDS).
    pub fn bit(self) -> u32 {
        let context = match self {
            UnlockContext::InWater => Context::PlayerInWater,
            UnlockContext::ManyItems => Context::PlayerHasManyItems,
        };
        1 << context.to_raw()
    }
}

enum Unlock {
    Items(Vec<Ingredient>),
    Context(UnlockContext),
}

/// A recipe the book can learn: its id, what unlocks it, and what it makes.
struct Entry {
    id: String,
    unlock: Unlock,
    makes: Option<&'static str>,
}

/// The learnable recipes by network id.
static UNLOCKS: LazyLock<Vec<Entry>> = LazyLock::new(|| {
    crate::set_shield_item_id();
    let data = decode::<CraftingData>(CRAFTING_DATA);
    let shaped = data.shaped_recipes.into_iter().chain(data.shaped_chemistry_recipes).map(|recipe| (recipe.network_id, recipe.recipe_id, recipe.unlocking_requirement));
    let shapeless = data.shapeless_recipes.into_iter().chain(data.shulker_box_recipes).chain(data.shapeless_chemistry_recipes).map(|recipe| (recipe.network_id, recipe.recipe_id, recipe.unlocking_requirement));
    let mut all: Vec<_> = shaped.chain(shapeless).filter(|(_, id, _)| !id.is_empty()).collect();
    all.sort_by_key(|(network_id, ..)| *network_id);
    let makes = |network_id: u32| super::recipe(network_id).and_then(|recipe| recipe.outputs.first()).map(|(item, _)| item.name);
    all.into_iter().filter_map(|(network_id, id, requirement)| Some(Entry { id, unlock: unlock(requirement?)?, makes: makes(network_id) })).collect()
});

fn unlock(requirement: Requirement) -> Option<Unlock> {
    match requirement.context {
        Context::None => Some(Unlock::Items(requirement.ingredients?.iter().filter_map(taken).collect())),
        Context::PlayerInWater => Some(Unlock::Context(UnlockContext::InWater)),
        Context::PlayerHasManyItems => Some(Unlock::Context(UnlockContext::ManyItems)),
        _ => None,
    }
}

/// The recipes that items newly held (name and damage) and states newly met unlock, in the
/// order BDS lists them. An item also unlocks the recipes that make it: a stick brought
/// `Stick_bamboo_recipeId`, a diamond the recipe from diamond blocks, neither in their unlock
/// lists (bds-recipeunlock.txt). Each way lists an id once, however many recipes share it (the
/// furnace's and the blast furnace's `furnace_diamond_ore`), but a recipe unlocked both ways is
/// listed twice, as BDS listed `minecraft:stick` for planks and sticks (bds-limitedcraft.txt). One
/// with an empty unlock list (sticks from mangrove planks) never unlocks.
/// The states' recipes come first (the chest before leather's own, bds-recipeunlock2.txt).
pub fn unlocked_by(items: &[(&str, i16)], contexts: &[UnlockContext]) -> Vec<&'static str> {
    let in_context = |entry: &&Entry| matches!(entry.unlock, Unlock::Context(context) if contexts.contains(&context));
    let mut unlocked: Vec<&'static str> = UNLOCKS.iter().filter(in_context).map(|entry| entry.id.as_str()).collect();
    let (mut by_unlock, mut by_result) = (HashSet::new(), HashSet::new());
    for entry in UNLOCKS.iter() {
        let met = match &entry.unlock {
            Unlock::Items(cells) if cells.is_empty() => continue,
            Unlock::Items(cells) => cells.iter().any(|cell| items.iter().any(|(name, damage)| cell.accepts(name, *damage))),
            Unlock::Context(_) => false,
        };
        let made = entry.makes.is_some_and(|made| items.iter().any(|(name, _)| *name == made));
        let id = entry.id.as_str();
        unlocked.extend((met && by_unlock.insert(id)).then_some(id));
        unlocked.extend((made && by_result.insert(id)).then_some(id));
    }
    unlocked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn by(items: &[(&str, i16)], contexts: &[UnlockContext]) -> Vec<&'static str> {
        unlocked_by(items, contexts)
    }

    #[test]
    fn items_and_states_unlock_what_bds_unlocked() {
        assert_eq!(by(&[("minecraft:oak_log", 0)], &[]), ["minecraft:furnace_log", "minecraft:oak_planks", "minecraft:oak_wood"]);
        let bds_stick = ["minecraft:torch", "minecraft:wooden_spear", "minecraft:campfire", "minecraft:ladder", "minecraft:stick", "minecraft:wooden_axe", "minecraft:wooden_hoe", "minecraft:wooden_pickaxe", "minecraft:wooden_shovel", "minecraft:wooden_sword", "Stick_bamboo_recipeId"];
        assert_eq!(by(&[("minecraft:stick", 0)], &[]), bds_stick);
        let bds_diamond = [
            "minecraft:furnace_deepslate_diamond_ore", "minecraft:furnace_diamond_ore", "minecraft:diamond_spear", "minecraft:diamond", "minecraft:diamond_axe", "minecraft:diamond_block", "minecraft:diamond_boots",
            "minecraft:diamond_chestplate", "minecraft:diamond_helmet", "minecraft:diamond_hoe", "minecraft:diamond_leggings", "minecraft:diamond_pickaxe", "minecraft:diamond_shovel", "minecraft:diamond_sword",
            "minecraft:Jukebox_recipeId",
        ];
        assert_eq!(by(&[("minecraft:diamond", 0)], &[]), bds_diamond);
        let bds_leather = ["minecraft:Chest_recipeId", "minecraft:saddle", "minecraft:item_frame", "minecraft:leather", "minecraft:leather_boots", "minecraft:leather_chestplate", "minecraft:leather_helmet", "minecraft:leather_horse_armor", "minecraft:leather_leggings"];
        // String had brought the bundle before.
        let leather: Vec<_> = by(&[("minecraft:leather", 0)], &[UnlockContext::ManyItems]).into_iter().filter(|id| *id != "minecraft:bundle").collect();
        assert_eq!(leather, bds_leather);
        let bds_boats = [
            "minecraft:poplar_boat", "minecraft:pale_oak_boat", "minecraft:acacia_boat", "minecraft:birch_boat", "minecraft:boat", "minecraft:dark_oak_boat", "minecraft:jungle_boat", "minecraft:spruce_boat",
            "minecraft:bamboo_raft", "minecraft:cherry_boat", "minecraft:mangrove_boat",
        ];
        assert_eq!(by(&[], &[UnlockContext::InWater]), bds_boats);
        // Flint had brought the arrow before the feather came.
        let none = by(&[("minecraft:dirt", 0), ("minecraft:feather", 0), ("minecraft:apple", 0), ("minecraft:wheat_seeds", 0)], &[]);
        assert_eq!(none, ["minecraft:arrow"]);
    }

    #[test]
    fn two_items_unlock_one_list_in_recipe_order() {
        let bds = [
            "minecraft:torch", "minecraft:wooden_spear", "minecraft:fence", "minecraft:fence_gate", "minecraft:oak_fence", "minecraft:oak_planks", "minecraft:oak_planks_from_stripped",
            "minecraft:oak_stairs", "minecraft:oak_wooden_slab", "minecraft:sign_oak", "minecraft:wooden_door", "minecraft:oak_planks_from_stripped_wood", "minecraft:oak_planks_from_wood",
            "minecraft:barrel_with_planks", "minecraft:campfire", "minecraft:ladder", "minecraft:stick", "minecraft:stick", "minecraft:wooden_axe", "minecraft:wooden_hoe",
            "minecraft:wooden_pickaxe", "minecraft:wooden_shovel", "minecraft:wooden_sword", "Stick_bamboo_recipeId", "WoodButton_recipeId", "WoodPressurePlate_recipeId", "Trapdoor_recipeId",
            "oak_stairs_oak_recipeId",
        ];
        assert_eq!(by(&[("minecraft:oak_planks", 0), ("minecraft:stick", 0)], &[]), bds);
    }
}
