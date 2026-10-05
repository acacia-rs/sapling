//! The crafting recipes clients are told of (`CraftingData.bin`, BDS's own list), read for the
//! server's side of a craft: what a recipe uses up and what it makes.
// TODO: only the crafting grid's shaped and shapeless recipes and the stonecutter's; furnaces,
// smithing and the special recipes (maps, fireworks, dyed armour) are sent to clients but not
// read here.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use acacia_proto::packets::CraftingData;
use acacia_proto::types::{ItemLegacy, RecipeIngredient, RecipeIngredientContent as Content, RecipeIngredientContentValidContent as Valid};

use crate::items::{CreativeItem, described};
use crate::templates::{CRAFTING_DATA, decode};

const GRID: &str = "crafting_table";
const STONECUTTER: &str = "stonecutter";
/// The metadata of an ingredient that takes any.
const ANY: i32 = 32767;

/// What one cell of a recipe takes.
#[derive(Debug, Clone, PartialEq)]
pub enum Ingredient {
    /// One item; `damage` is `None` when any will do.
    Item { name: String, damage: Option<i16> },
    /// Any item carrying one of these tags.
    Tagged(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Recipe {
    /// Each cell that takes something, with how many of it one craft uses.
    pub inputs: Vec<(Ingredient, u8)>,
    /// What one craft makes, with how many of each.
    pub outputs: Vec<(CreativeItem, u8)>,
    pub station: Station,
}

/// Where a recipe is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Station {
    /// The 2x2 grid of the inventory screen, or a crafting table.
    Hand,
    /// A crafting table: the recipe does not fit 2x2.
    Table,
    Stonecutter,
}

static RECIPES: LazyLock<HashMap<u32, Recipe>> = LazyLock::new(|| {
    crate::set_shield_item_id();
    let data = decode::<CraftingData>(CRAFTING_DATA);
    let table_if = |needed: bool| if needed { Station::Table } else { Station::Hand };
    let shaped = data.shaped_recipes.into_iter().filter(|recipe| recipe.block == GRID).map(|recipe| {
        let inputs = inputs(&recipe.input);
        (recipe.network_id, Recipe { inputs, outputs: outputs(recipe.output), station: table_if(recipe.width > 2 || recipe.height > 2) })
    });
    // A stonecutter's recipes are shapeless ones of one input, told apart by their block.
    let shapeless = data.shapeless_recipes.into_iter().filter(|recipe| [GRID, STONECUTTER].contains(&recipe.block.as_str())).map(|recipe| {
        let inputs = inputs(&recipe.input);
        let station = if recipe.block == STONECUTTER { Station::Stonecutter } else { table_if(inputs.len() > 4) };
        (recipe.network_id, Recipe { station, inputs, outputs: outputs(recipe.output) })
    });
    shaped.chain(shapeless).filter(|(_, recipe)| !recipe.inputs.is_empty() && !recipe.outputs.is_empty()).collect()
});

fn inputs(cells: &[RecipeIngredient]) -> Vec<(Ingredient, u8)> {
    let any_tag = |expression: &str| {
        let arguments = expression.split_once("any_tag(").map_or("", |(_, rest)| rest.split(')').next().unwrap_or(""));
        arguments.split(',').map(|tag| tag.trim().trim_matches(['\'', '"']).to_owned()).filter(|tag| !tag.is_empty()).collect()
    };
    let cell = |cell: &RecipeIngredient| {
        let Content::Valid(valid) = &cell.content else { return None };
        let ingredient = match &valid.content {
            Valid::Name(item) => Ingredient::Item { name: item.name.clone(), damage: Some(item.metadata).filter(|damage| ![ANY, -1].contains(damage)).map(|damage| damage as i16) },
            Valid::ItemTag(tagged) => Ingredient::Tagged(vec![tagged.tag.clone()]),
            Valid::Molang(molang) => Ingredient::Tagged(any_tag(&molang.expression)),
            Valid::Empty(_) | Valid::Default => return None,
        };
        Some((ingredient, u8::try_from(cell.count).ok().filter(|count| *count > 0)?))
    };
    cells.iter().filter_map(cell).collect()
}

fn outputs(made: Vec<ItemLegacy>) -> Vec<(CreativeItem, u8)> {
    made.into_iter().filter_map(|item| Some((u8::try_from(item.count).ok()?, described(item)?))).map(|(count, item)| (item, count)).collect()
}

/// The recipe a client names by `network_id` in a craft request.
pub fn recipe(network_id: u32) -> Option<&'static Recipe> {
    RECIPES.get(&network_id)
}

/// The items of each tag the recipes name (`data/item-tags.tsv`, README.md "Item tags"): clients
/// have the tags built in and no packet carries them.
static TAGGED: LazyLock<HashSet<(&'static str, &'static str)>> = LazyLock::new(|| {
    let pair = |line: &'static str| line.split('\t').next().zip(line.split('\t').nth(1));
    include_str!("../data/item-tags.tsv").lines().skip(1).filter_map(pair).collect()
});

/// Whether the item `name` carries `tag`; a tag the file lacks is carried by nothing.
fn has_tag(name: &str, tag: &str) -> bool {
    TAGGED.contains(&(tag, name))
}

impl Ingredient {
    pub fn accepts(&self, name: &str, damage: i16) -> bool {
        match self {
            Ingredient::Item { name: wanted, damage: of } => wanted == name && of.is_none_or(|of| of == damage),
            Ingredient::Tagged(tags) => tags.iter().any(|tag| has_tag(name, tag)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn making(name: &str) -> Vec<&'static Recipe> {
        RECIPES.values().filter(|recipe| recipe.outputs[0].0.name == name).collect()
    }

    #[test]
    fn the_grids_recipes_say_what_they_use_and_make() {
        assert!(RECIPES.len() > 1000, "{} recipes", RECIPES.len());
        let sticks = making("minecraft:stick");
        let of_planks = |recipe: &&&Recipe| recipe.inputs.iter().all(|(cell, count)| *count == 1 && cell.accepts("minecraft:birch_planks", 0) && !cell.accepts("minecraft:stone", 0));
        let from_planks = sticks.iter().find(of_planks).expect("sticks from planks");
        assert_eq!((from_planks.inputs.len(), from_planks.outputs[0].1, from_planks.station), (2, 4, Station::Hand));
        let pickaxe = making("minecraft:wooden_pickaxe");
        assert!(pickaxe[0].station == Station::Table && pickaxe[0].inputs.len() == 5);
        assert!(making("minecraft:oak_planks").iter().any(|recipe| recipe.inputs[0].0.accepts("minecraft:oak_log", 0) && recipe.outputs[0].1 == 4));
        assert!(making("minecraft:crafting_table")[0].outputs[0].0.block.is_some(), "a block item names its block");
        let cut: Vec<_> = RECIPES.values().filter(|recipe| recipe.station == Station::Stonecutter).collect();
        assert!(cut.len() > 300 && cut.iter().all(|recipe| recipe.inputs.len() == 1 && recipe.outputs.len() == 1), "{} stonecutter recipes", cut.len());
        assert_eq!(making("minecraft:cake")[0].outputs.len(), 2, "a cake gives its buckets back");
        assert!(has_tag("minecraft:birch_planks", "minecraft:planks") && has_tag("minecraft:charcoal", "minecraft:coals") && !has_tag("minecraft:stone", "minecraft:planks"));
    }
}
