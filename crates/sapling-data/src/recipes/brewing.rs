//! The brewing stand's recipes, from the `CraftingData` clients are sent (BDS's own list): a
//! potion mix turns a bottle of one potion into another by its ingredient (water and nether wart
//! into awkward), a container mix turns any potion into its splash or lingering bottle, keeping
//! the potion (gunpowder, dragon's breath).

use std::sync::LazyLock;

use acacia_proto::packets::CraftingData;

use crate::items::item_name;
use crate::templates::{CRAFTING_DATA, decode};

/// What a mix takes and makes: the bottle item and its potion, the ingredient and its data.
struct Mix {
    bottle: (&'static str, i16),
    ingredient: (&'static str, i16),
    made: (&'static str, i16),
}

/// The container mixes: the bottle item, the ingredient, the bottle item made.
type Change = (&'static str, &'static str, &'static str);

static MIXES: LazyLock<(Vec<Mix>, Vec<Change>)> = LazyLock::new(|| {
    crate::set_shield_item_id();
    let data = decode::<CraftingData>(CRAFTING_DATA);
    let named = |id: i32, meta: i32| Some((item_name(id)?, i16::try_from(meta).ok()?));
    let potions = data.potion_type_recipes.iter().filter_map(|mix| {
        Some(Mix { bottle: named(mix.input_item_id, mix.input_item_meta)?, ingredient: named(mix.ingredient_id, mix.ingredient_meta)?, made: named(mix.output_item_id, mix.output_item_meta)? })
    });
    let changes = data.potion_container_recipes.iter().filter_map(|mix| Some((item_name(mix.input_item_id)?, item_name(mix.ingredient_id)?, item_name(mix.output_item_id)?)));
    (potions.collect(), changes.collect())
});

/// What a bottle `bottle` of potion `potion` becomes when brewed with `ingredient` (of data
/// `data`): the bottle item and its potion. `None` where no mix takes the two.
pub fn brewed(bottle: &str, potion: i16, ingredient: &str, data: i16) -> Option<(&'static str, i16)> {
    let (mixes, changes) = &*MIXES;
    let takes = |(name, of): (&str, i16), wanted: (&str, i16)| name == wanted.0 && of == wanted.1;
    let mixed = mixes.iter().find(|mix| takes((bottle, potion), mix.bottle) && takes((ingredient, data), mix.ingredient)).map(|mix| mix.made);
    mixed.or_else(|| changes.iter().find(|(from, by, _)| *from == bottle && *by == ingredient).map(|(_, _, made)| (*made, potion)))
}

/// Whether `ingredient` is one any mix takes: what a brewing stand's top slot holds.
pub fn is_brewing_ingredient(ingredient: &str, data: i16) -> bool {
    let (mixes, changes) = &*MIXES;
    mixes.iter().any(|mix| mix.ingredient == (ingredient, data)) || changes.iter().any(|(_, by, _)| *by == ingredient)
}

/// Whether `bottle` is a bottle some mix takes: what a brewing stand's three lower slots hold.
pub fn is_brewing_bottle(bottle: &str) -> bool {
    let (mixes, changes) = &*MIXES;
    mixes.iter().any(|mix| mix.bottle.0 == bottle) || changes.iter().any(|(from, ..)| *from == bottle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottles_brew_by_bds_mixes() {
        let (mixes, changes) = &*MIXES;
        assert!(mixes.len() > 100 && changes.len() >= 2, "{} mixes, {} changes", mixes.len(), changes.len());
        let potion = "minecraft:potion";
        assert_eq!(brewed(potion, 0, "minecraft:nether_wart", 0), Some((potion, 4)), "water and wart: awkward");
        assert_eq!(brewed(potion, 4, "minecraft:sugar", 0), Some((potion, 14)), "awkward and sugar: swiftness");
        assert_eq!(brewed(potion, 4, "minecraft:gunpowder", 0), Some(("minecraft:splash_potion", 4)));
        assert_eq!(brewed("minecraft:splash_potion", 4, "minecraft:dragon_breath", 0), Some(("minecraft:lingering_potion", 4)));
        assert_eq!(brewed(potion, 4, "minecraft:stone_sword", 0), None);
        assert!(is_brewing_ingredient("minecraft:blaze_powder", 0) && !is_brewing_ingredient("minecraft:diamond", 0));
        assert!(is_brewing_bottle(potion) && is_brewing_bottle("minecraft:lingering_potion") && !is_brewing_bottle("minecraft:glass_bottle"));
    }
}
