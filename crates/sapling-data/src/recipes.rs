//! The crafting recipes clients are told of (`CraftingData.bin`, BDS's own list), read for the
//! server's side of a craft: what a recipe uses up and what it makes.
// TODO: the crafting grid's shaped and shapeless recipes, the stonecutter's, the three
// smelters' and campfires', the smithing table's (smithing.rs); the special recipes (maps,
// fireworks, dyed armour) are known by network id alone (`is_special`); what they make is the shard's.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use acacia_proto::packets::CraftingData;
use acacia_proto::types::{ItemLegacy, RecipeIngredient, RecipeIngredientContent as Content, RecipeIngredientContentValidContent as Valid};

use crate::items::{CreativeItem, described};
use crate::templates::{CRAFTING_DATA, decode};

mod book;
mod brewing;
mod molang;
mod smithing;
mod unlock;

pub use book::{initial_recipes, recipe_ids};
pub use unlock::{UnlockContext, unlocked_by};
pub use brewing::{brewed, is_brewing_bottle, is_brewing_ingredient};
pub use smithing::smithed;

const GRID: &str = "crafting_table";
/// The metadata of an ingredient that takes any.
const ANY: i32 = 32767;

/// What one cell of a recipe takes.
#[derive(Debug, Clone, PartialEq)]
pub enum Ingredient {
    /// One item; `damage` is `None` when any will do.
    Item { name: String, damage: Option<i16> },
    /// Any item carrying one of these tags.
    Tagged(Vec<String>),
    /// Any item this MoLang expression holds for (`query.any_tag('minecraft:planks')`).
    Expression(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Recipe {
    /// Each cell that takes something, with how many of it one craft uses.
    pub inputs: Vec<(Ingredient, u8)>,
    /// What one craft makes, with how many of each.
    pub outputs: Vec<(CreativeItem, u8)>,
    pub station: Station,
    /// How a shaped recipe lies in the grid; `None` for one whose cells may lie anywhere.
    pub shape: Option<Shape>,
}

/// The cells of a shaped recipe row by row, `width` to a row; `None` is a cell left empty.
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub width: usize,
    pub cells: Vec<Option<Ingredient>>,
}

/// Where a recipe is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Station {
    /// The 2x2 grid of the inventory screen, or a crafting table.
    Hand,
    /// A crafting table: the recipe does not fit 2x2.
    Table,
    Stonecutter,
    Furnace,
    BlastFurnace,
    Smoker,
    /// A campfire or soul campfire, which the game lists separately with the same recipes.
    Campfire,
}

/// The stations whose recipes are shapeless ones of one input, by the block the list names.
const BY_BLOCK: [(&str, Station); 6] = [
    ("stonecutter", Station::Stonecutter),
    ("furnace", Station::Furnace),
    ("blast_furnace", Station::BlastFurnace),
    ("smoker", Station::Smoker),
    ("campfire", Station::Campfire),
    ("soul_campfire", Station::Campfire),
];

static RECIPES: LazyLock<HashMap<u32, Recipe>> = LazyLock::new(|| {
    crate::set_shield_item_id();
    let data = decode::<CraftingData>(CRAFTING_DATA);
    let table_if = |needed: bool| if needed { Station::Table } else { Station::Hand };
    let shaped = data.shaped_recipes.into_iter().filter(|recipe| recipe.block == GRID).map(|recipe| {
        let inputs = inputs(&recipe.input);
        let shape = Shape { width: recipe.width.max(1) as usize, cells: recipe.input.iter().map(|cell| ingredient(cell).map(|(taken, _)| taken)).collect() };
        (recipe.network_id, Recipe { inputs, outputs: outputs(recipe.output), station: table_if(recipe.width > 2 || recipe.height > 2), shape: Some(shape) })
    });
    let shapeless = data.shapeless_recipes.into_iter().filter_map(|recipe| {
        let inputs = inputs(&recipe.input);
        let other = BY_BLOCK.iter().find(|(block, _)| *block == recipe.block).map(|(_, station)| *station);
        let station = other.or_else(|| (recipe.block == GRID).then(|| table_if(inputs.len() > 4)))?;
        Some((recipe.network_id, Recipe { station, inputs, outputs: outputs(recipe.output), shape: None }))
    });
    shaped.chain(shapeless).filter(|(_, recipe)| !recipe.inputs.is_empty() && !recipe.outputs.is_empty()).collect()
});

/// The recipes of the three smelters and campfires: one input, one result.
static SMELTING: LazyLock<Vec<&'static Recipe>> = LazyLock::new(|| {
    let smelts = |recipe: &&Recipe| matches!(recipe.station, Station::Furnace | Station::BlastFurnace | Station::Smoker | Station::Campfire);
    RECIPES.values().filter(smelts).filter(|recipe| recipe.inputs.len() == 1 && recipe.outputs.len() == 1).collect()
});

/// What one of an item becomes in the smelter `station`, and how many of it.
pub fn smelted(station: Station, name: &str, damage: i16) -> Option<&'static (CreativeItem, u8)> {
    let takes = |recipe: &&&Recipe| recipe.station == station && recipe.inputs[0].0.accepts(name, damage);
    SMELTING.iter().find(takes).map(|recipe| &recipe.outputs[0])
}

/// What a cell of a recipe takes and how many of it; `None` for a cell that takes nothing.
fn ingredient(cell: &RecipeIngredient) -> Option<(Ingredient, u8)> {
    Some((taken(cell)?, u8::try_from(cell.count).ok().filter(|count| *count > 0)?))
}

/// What a cell takes, however many.
fn taken(cell: &RecipeIngredient) -> Option<Ingredient> {
    let Content::Valid(valid) = &cell.content else { return None };
    Some(match &valid.content {
        Valid::Name(item) => Ingredient::Item { name: item.name.clone(), damage: Some(item.metadata).filter(|damage| ![ANY, -1].contains(damage)).map(|damage| damage as i16) },
        Valid::ItemTag(tagged) => Ingredient::Tagged(vec![tagged.tag.clone()]),
        Valid::Molang(molang) => Ingredient::Expression(molang.expression.clone()),
        Valid::Empty(_) | Valid::Default => return None,
    })
}

fn inputs(cells: &[RecipeIngredient]) -> Vec<(Ingredient, u8)> {
    cells.iter().filter_map(ingredient).collect()
}

/// A crafting grid row by row: the name and damage of what lies in each cell.
pub type Grid<'a> = [Option<(&'a str, i16)>];

impl Shape {
    /// Whether the grid holds these cells at `(left, top)`, mirrored or not, and nothing else.
    fn lies_at(&self, grid: &Grid<'_>, side: usize, left: usize, top: usize, mirrored: bool) -> bool {
        let height = self.cells.len() / self.width;
        grid.iter().enumerate().all(|(index, held)| {
            let (x, y) = (index % side, index / side);
            let within = (left..left + self.width).contains(&x) && (top..top + height).contains(&y);
            let cell = |across: usize| self.cells.get((y - top) * self.width + if mirrored { self.width - 1 - across } else { across });
            match (within.then(|| cell(x - left)).flatten().and_then(Option::as_ref), held) {
                (Some(cell), Some((name, damage))) => cell.accepts(name, *damage),
                (None, None) => true,
                _ => false,
            }
        })
    }
}

impl Recipe {
    /// Whether a square crafting grid is this recipe laid out, as BDS's `Recipe::matches` asks
    /// of a craft by hand (docs/research/containers-crafting.md, E1): a shaped one anywhere in
    /// the grid, mirrored or not, a shapeless one in any cells; and nothing beside it.
    pub fn is_laid_out(&self, grid: &Grid<'_>) -> bool {
        let side = grid.len().isqrt();
        let Some(shape) = &self.shape else {
            let mut free: Vec<(&str, i16)> = grid.iter().flatten().copied().collect();
            // Named cells first, so that a tag does not take what only a name can.
            let mut cells: Vec<&Ingredient> = self.inputs.iter().map(|(cell, _)| cell).collect();
            cells.sort_by_key(|cell| !matches!(cell, Ingredient::Item { .. }));
            let all_found = cells.iter().all(|cell| free.iter().position(|(name, damage)| cell.accepts(name, *damage)).map(|found| free.swap_remove(found)).is_some());
            return all_found && free.is_empty();
        };
        let height = shape.cells.len() / shape.width;
        if shape.width > side || height > side {
            return false;
        }
        let places = (0..=side - shape.width).flat_map(|left| (0..=side - height).map(move |top| (left, top)));
        places.into_iter().any(|(left, top)| [false, true].into_iter().any(|mirrored| shape.lies_at(grid, side, left, top, mirrored)))
    }
}

fn outputs(made: Vec<ItemLegacy>) -> Vec<(CreativeItem, u8)> {
    made.into_iter().filter_map(|item| Some((u8::try_from(item.count).ok()?, described(item)?))).map(|(count, item)| (item, count)).collect()
}

/// The crafting table's recipe a grid lays out, as a crafter looks for one: the lowest network id
/// of those that fit, so that the choice does not turn on the map's order.
pub fn laid_out_in(grid: &Grid<'_>) -> Option<&'static Recipe> {
    let fits = |(_, recipe): &(&u32, &Recipe)| matches!(recipe.station, Station::Hand | Station::Table) && recipe.is_laid_out(grid);
    RECIPES.iter().filter(fits).min_by_key(|(id, _)| **id).map(|(_, recipe)| recipe)
}

/// The recipe a client names by `network_id` in a craft request.
pub fn recipe(network_id: u32) -> Option<&'static Recipe> {
    RECIPES.get(&network_id)
}

/// The network ids of the special recipes, whose result the grid's items decide.
static SPECIAL: LazyLock<HashSet<u32>> = LazyLock::new(|| decode::<CraftingData>(CRAFTING_DATA).multi_recipes.into_iter().map(|recipe| recipe.network_id).collect());

static MULTI: LazyLock<HashMap<String, u32>> =
    LazyLock::new(|| decode::<CraftingData>(CRAFTING_DATA).multi_recipes.into_iter().map(|recipe| (recipe.uuid.to_string(), recipe.network_id)).collect());

/// The network id CraftingData gives the special recipe of `uuid` (the cartography table's crafts).
pub fn multi_recipe(uuid: &str) -> Option<u32> {
    MULTI.get(uuid).copied()
}

/// Whether a client's craft request names one of the special recipes.
pub fn is_special(network_id: u32) -> bool {
    crate::set_shield_item_id();
    SPECIAL.contains(&network_id)
}

/// The items of each tag the recipes name (`data/item-tags.tsv`, README.md "Item tags"): clients
/// have the tags built in and no packet carries them.
static TAGGED: LazyLock<HashSet<(&'static str, &'static str)>> = LazyLock::new(|| {
    let pair = |line: &'static str| line.split('\t').next().zip(line.split('\t').nth(1));
    include_str!("../data/item-tags.tsv").lines().skip(1).filter_map(pair).collect()
});

/// Whether the item `name` carries `tag`; a tag the file lacks is carried by nothing.
pub fn has_tag(name: &str, tag: &str) -> bool {
    TAGGED.contains(&(tag, name))
}

impl Ingredient {
    pub fn accepts(&self, name: &str, damage: i16) -> bool {
        match self {
            Ingredient::Item { name: wanted, damage: of } => wanted == name && of.is_none_or(|of| of == damage),
            Ingredient::Tagged(tags) => tags.iter().any(|tag| has_tag(name, tag)),
            Ingredient::Expression(expression) => molang::accepts(expression, name),
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
        let smelt = |station, item: &str| smelted(station, item, 0).map(|(made, count)| (made.name, *count));
        assert_eq!([smelt(Station::Furnace, "minecraft:raw_iron"), smelt(Station::BlastFurnace, "minecraft:raw_iron")], [Some(("minecraft:iron_ingot", 1)); 2]);
        assert_eq!([smelt(Station::Smoker, "minecraft:raw_iron"), smelt(Station::Smoker, "minecraft:beef"), smelt(Station::Table, "minecraft:beef")], [None, Some(("minecraft:cooked_beef", 1)), None]);
        assert_eq!([smelt(Station::Campfire, "minecraft:beef"), smelt(Station::Campfire, "minecraft:raw_iron")], [Some(("minecraft:cooked_beef", 1)), None]);
        // A pickaxe lies one way only; an axe also mirrored; sticks anywhere but not across.
        let (plank, stick) = (Some(("minecraft:oak_planks", 0)), Some(("minecraft:stick", 0)));
        let pick = [plank, plank, plank, None, stick, None, None, stick, None];
        let crooked = [plank, plank, plank, stick, None, None, None, stick, None];
        // Several recipes can make one item, so the questions are put to all of them.
        assert!(pickaxe.iter().any(|recipe| recipe.is_laid_out(&pick)) && !pickaxe.iter().any(|recipe| recipe.is_laid_out(&crooked) || recipe.is_laid_out(&pick[..4])));
        let item = |name: &str| Some(Ingredient::Item { name: name.to_owned(), damage: None });
        let cells = vec![item("minecraft:oak_planks"), item("minecraft:oak_planks"), item("minecraft:oak_planks"), item("minecraft:stick"), None, item("minecraft:stick")];
        let axe = Recipe { inputs: Vec::new(), outputs: Vec::new(), station: Station::Table, shape: Some(Shape { width: 2, cells }) };
        let axes = [[plank, plank, None, plank, stick, None, None, stick, None], [None, plank, plank, None, stick, plank, None, stick, None]];
        assert!(axes.iter().all(|grid| axe.is_laid_out(grid)) && !axe.is_laid_out(&[plank, plank, None, stick, plank, None, None, stick, None]));
        assert!(from_planks.is_laid_out(&[None, plank, None, plank]) && from_planks.is_laid_out(&[None, None, plank, None, None, plank, None, None, None]));
        assert!(!from_planks.is_laid_out(&[plank, plank, None, None]) && !from_planks.is_laid_out(&[plank, stick, plank, None]));
        let from_log = making("minecraft:oak_planks").into_iter().find(|recipe| recipe.inputs[0].0.accepts("minecraft:oak_log", 0)).expect("planks from logs");
        let log = Some(("minecraft:oak_log", 0));
        assert!(from_log.is_laid_out(&[None, None, None, log]) && !from_log.is_laid_out(&[log, None, None, log]) && !from_log.is_laid_out(&[None; 4]));
        assert!(has_tag("minecraft:birch_planks", "minecraft:planks") && has_tag("minecraft:charcoal", "minecraft:coals") && !has_tag("minecraft:stone", "minecraft:planks"));
    }

    #[test]
    fn the_special_recipes_are_known_by_id_and_listed_apart() {
        assert!(SPECIAL.len() >= 3, "{} special recipes", SPECIAL.len());
        assert!(SPECIAL.iter().all(|id| is_special(*id) && recipe(*id).is_none()));
        assert!(multi_recipe("442d85ed-8272-4543-a6f1-418f90ded05d").is_some_and(is_special), "the cartography table's copy");
    }
}
