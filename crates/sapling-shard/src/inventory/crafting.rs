//! The server's side of a craft: a request names a recipe, uses items up and takes the result,
//! and is done only if what it used up is what the recipe asks for (requests.rs). How BDS does
//! it, where this follows it and where not: docs/research/containers-crafting.md, sections E, F.
// TODO: where the items lie in the grid is not checked, only which were used (BDS matches the
// grid against the recipe); no recipe unlocking.

use bevy_ecs::prelude::*;
use sapling_data::{Ingredient, Recipe, Station};
use sapling_world::ItemStack;

use super::{Inventory, Place, UI, change};
use crate::drops;

/// The craft a request asked for, and what it has used up so far.
pub(super) struct Craft {
    recipe: &'static Recipe,
    times: u8,
    used: Vec<ItemStack>,
    /// Which of the recipe's results is being handed out, and how many of it are still to come
    /// once the created-output slot has room: it holds one stack at a time.
    result: usize,
    pending: u32,
}

impl Craft {
    /// Starts `times` crafts of the recipe a client names, at the screen the player has open.
    /// A recipe's only result lies in the created-output slot at once; of several, each waits
    /// for the request to ask for it ([`Craft::create`]).
    pub fn start(inventory: &mut Inventory, network_id: u32, times: u8, at: Station) -> Result<Self, ()> {
        let fits = |recipe: &&Recipe| match recipe.station {
            Station::Hand => at != Station::Stonecutter,
            station => station == at,
        };
        let recipe = sapling_data::recipe(network_id).filter(fits).filter(|_| times > 0).ok_or(())?;
        let mut craft = Self { recipe, times, used: Vec::new(), result: 0, pending: 0 };
        if recipe.outputs.len() == 1 {
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

    pub fn use_up(&mut self, stack: ItemStack) {
        self.used.push(stack);
    }

    /// Whether what was used up is what the crafts need, no more and no less.
    pub fn is_paid(&self) -> bool {
        let mut left: Vec<(&ItemStack, u16)> = self.used.iter().map(|stack| (stack, stack.count.into())).collect();
        // Named cells first, so that a tag does not take what only a name can.
        let mut cells: Vec<&(Ingredient, u8)> = self.recipe.inputs.iter().collect();
        cells.sort_by_key(|(cell, _)| matches!(cell, Ingredient::Tagged(_)));
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
