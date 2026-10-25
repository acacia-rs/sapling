//! The server's side of a craft: a request names a recipe, uses items up and takes the result,
//! and is done only if what it used up is what the recipe asks for (requests.rs). How BDS does
//! it, where this follows it and where not: docs/research/containers-crafting.md, sections E, F.
// TODO: a craft by the recipe book is only held to what it used, not to where it took it from;
// no recipe unlocking.

use std::borrow::Cow;
use std::ops::Range;

use bevy_ecs::prelude::*;
use sapling_data::{Ingredient, Recipe, Station};
use sapling_world::ItemStack;

use super::{Inventory, Place, UI, change};
use crate::drops;

/// The `Ui` slots of the crafting grids, row by row: the inventory screen's and a table's.
const HAND_GRID: Range<usize> = 28..32;
const TABLE_GRID: Range<usize> = 32..41;

/// The craft a request asked for, and what it has used up so far.
pub(super) struct Craft {
    recipe: Cow<'static, Recipe>,
    times: u8,
    used: Vec<ItemStack>,
    /// The grid cells a craft by hand has yet to use up, `times` of each.
    cells: Option<Vec<Place>>,
    /// Which of the recipe's results is being handed out, and how many of it are still to come
    /// once the created-output slot has room: it holds one stack at a time.
    result: usize,
    pending: u32,
}

/// The filled cells of the grid a player crafts in at `at`, if the recipe is laid out in it.
fn laid_out(inventory: &Inventory, recipe: &Recipe, at: Station) -> Result<Vec<Place>, ()> {
    let cells = if at == Station::Table { TABLE_GRID } else { HAND_GRID };
    let held = |slot: usize| inventory.slot(Place::Ui(slot)).and_then(Option::as_ref);
    let grid: Vec<Option<(&str, i16)>> = cells.clone().map(|slot| held(slot).map(|held| (held.stack.name.as_str(), held.stack.damage))).collect();
    recipe.is_laid_out(&grid).then(|| cells.filter(|slot| held(*slot).is_some()).map(Place::Ui).collect()).ok_or(())
}

/// A special recipe as what the grid at `at` holds makes of it (fireworks/recipes.rs): one of
/// each filled cell for what it makes.
fn special(inventory: &Inventory, at: Station) -> Option<Recipe> {
    let cells = if at == Station::Table { TABLE_GRID } else { HAND_GRID };
    let grid: Vec<&ItemStack> = cells.filter_map(|slot| inventory.slot(Place::Ui(slot)).and_then(Option::as_ref)).map(|held| &held.stack).collect();
    let made = crate::fireworks::crafted(&grid)?;
    let inputs = grid.iter().map(|stack| (Ingredient::Item { name: stack.name.clone(), damage: Some(stack.damage) }, 1)).collect();
    Some(Recipe { inputs, outputs: vec![made], station: Station::Hand, shape: None })
}

impl Craft {
    /// Starts `times` crafts of the recipe a client names, at the screen the player has open.
    /// One made by hand in a grid needs the recipe laid out there, and is then held to using up
    /// those cells. A recipe's only result lies in the created-output slot at once; of several,
    /// each waits for the request to ask for it ([`Craft::create`]).
    pub fn start(inventory: &mut Inventory, network_id: u32, times: u8, at: Station, by_hand: bool) -> Result<Self, ()> {
        let fits = |recipe: &&Recipe| match recipe.station {
            Station::Hand => at != Station::Stonecutter,
            station => station == at,
        };
        let recipe = match sapling_data::recipe(network_id) {
            Some(listed) => Cow::Borrowed(listed),
            None if sapling_data::is_special_recipe(network_id) && at != Station::Stonecutter => Cow::Owned(special(inventory, at).ok_or(())?),
            None => return Err(()),
        };
        let recipe = Some(recipe).filter(|recipe| fits(&&**recipe)).filter(|_| times > 0).ok_or(())?;
        let cells = (by_hand && at != Station::Stonecutter).then(|| laid_out(inventory, &recipe, at)).transpose()?;
        let single = recipe.outputs.len() == 1;
        let mut craft = Self { recipe, times, used: Vec::new(), cells, result: 0, pending: 0 };
        if single {
            craft.create(inventory, 0)?;
        }
        Ok(craft)
    }

    /// Lays result `index` into the created-output slot, which must be empty: a request takes
    /// one result out before it asks for the next.
    pub fn create(&mut self, inventory: &mut Inventory, index: usize) -> Result<(), ()> {
        let (_, each) = self.recipe.outputs.get(index).ok_or(())?;
        if self.pending > 0 || inventory.slot(Place::Created).is_some_and(Option::is_some) {
            return Err(());
        }
        (self.result, self.pending) = (index, u32::from(*each) * u32::from(self.times));
        self.refill(inventory);
        Ok(())
    }

    /// Puts the next stack of the result into the created-output slot once it was emptied.
    pub fn refill(&mut self, inventory: &mut Inventory) {
        if self.pending == 0 || inventory.slot(Place::Created).is_some_and(Option::is_some) {
            return;
        }
        let (item, _) = &self.recipe.outputs[self.result];
        let count = self.pending.min(sapling_data::max_stack(item.name).into()) as u8;
        self.pending -= u32::from(count);
        inventory.put(Place::Created, Some(ItemStack { name: item.name.to_owned(), count, damage: item.damage, block: item.block, tag: item.tag.clone() }));
    }

    /// Counts `stack`, taken from `from`, as used up; a craft by hand takes `times` from each
    /// filled cell of its grid, once, and from nowhere else.
    pub fn use_up(&mut self, from: Place, stack: ItemStack) -> Result<(), ()> {
        if let Some(cells) = self.cells.as_mut() {
            let cell = cells.iter().position(|cell| *cell == from).filter(|_| stack.count == self.times).ok_or(())?;
            cells.swap_remove(cell);
        }
        self.used.push(stack);
        Ok(())
    }

    /// Whether what was used up is what the crafts need, no more and no less.
    pub fn is_paid(&self) -> bool {
        if self.cells.as_ref().is_some_and(|cells| !cells.is_empty()) {
            return false;
        }
        let mut left: Vec<(&ItemStack, u16)> = self.used.iter().map(|stack| (stack, stack.count.into())).collect();
        // Named cells first, so that a tag does not take what only a name can.
        let mut cells: Vec<&(Ingredient, u8)> = self.recipe.inputs.iter().collect();
        cells.sort_by_key(|(cell, _)| !matches!(cell, Ingredient::Item { .. }));
        for (cell, count) in cells {
            let mut need = u16::from(*count) * u16::from(self.times);
            for entry in left.iter_mut().filter(|entry| cell.accepts(&entry.0.name, entry.0.damage)) {
                let taken = need.min(entry.1);
                (entry.1, need) = (entry.1 - taken, need - taken);
            }
            if need > 0 {
                return false;
            }
        }
        left.iter().all(|(_, unused)| *unused == 0)
    }
}

/// Puts what a player left in a screen's own slots (a crafting grid, a stonecutter) back among
/// its items when the screen closes; what does not fit is thrown.
pub(crate) fn clear_grid(world: &mut World, player: Entity) {
    if !world.get::<Inventory>(player).is_some_and(|inventory| inventory.grid.iter().any(Option::is_some)) {
        return;
    }
    let returned = |inventory: &mut Inventory| {
        let mut unplaced = Vec::new();
        for held in std::mem::replace(&mut inventory.grid, vec![None; UI]).into_iter().flatten() {
            let left = inventory.add(&held.stack);
            unplaced.extend((left > 0).then(|| ItemStack { count: left, ..held.stack }));
        }
        unplaced
    };
    for stack in change(world, player, returned).unwrap_or_default() {
        drops::throw(world, player, stack);
    }
}
