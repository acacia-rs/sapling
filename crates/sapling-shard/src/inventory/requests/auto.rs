//! What BDS answers a recipe-book craft (`CraftRecipeAuto`) whose Consumes it does not take, checked
//! in the request's order before it is tried (research/captures/bds-y3auto.txt). BDS resends nothing.

use acacia_proto::types::{ContainerSlotType, ItemStackRequest as Request, ItemStackRequestActionsItemContent as Action, ItemStackResponsesItemStatus as Status};

use super::super::{Inventory, Place};
use super::slots::place;

/// A craft on a screen with no craft handler: an open chest's.
const NO_CRAFT_HANDLER: Status = Status::Unknown(7);
/// Less used up than the crafts need.
const TOO_LITTLE: Status = Status::Unknown(19);
/// An item no cell of the recipe takes.
const NOT_AN_INGREDIENT: Status = Status::Unknown(25);
/// Taken from another container than `HotbarAndInventory` or the crafting grid (`Hotbar`, `Inventory`).
const NOT_ALLOWED: Status = Status::Unknown(27);
/// Taken from a slot that is empty, past the inventory's end, or emptied by the request.
const NOTHING_THERE: Status = Status::Unknown(49);

/// The status BDS refuses `request` with, if it is a recipe-book craft it refuses; `container_open`:
/// a container's screen is open rather than the inventory's or a table's.
pub(super) fn refusal(inventory: &Inventory, request: &Request, container_open: bool) -> Option<Status> {
    let mut actions = request.actions.iter().map(|action| &action.content).skip_while(|action| !matches!(action, Action::CraftRecipeAuto(_)));
    let Some(Action::CraftRecipeAuto(asked)) = actions.next() else { return None };
    if container_open {
        return Some(NO_CRAFT_HANDLER);
    }
    let recipe = sapling_data::recipe(asked.recipe_network_id)?;
    let mut taken: Vec<(Place, u16)> = Vec::new();
    for action in actions {
        let Action::Consume(consume) = action else { continue };
        if !matches!(consume.source.slot_type.container_id, ContainerSlotType::HotbarAndInventory | ContainerSlotType::CraftingInput) {
            return Some(NOT_ALLOWED);
        }
        let Some(at) = place(&consume.source) else { return Some(NOTHING_THERE) };
        let before: u16 = taken.iter().filter(|(place, _)| *place == at).map(|(_, count)| count).sum();
        let held = inventory.slot(at).and_then(Option::as_ref).map(|held| &held.stack);
        let Some(stack) = held.filter(|stack| u16::from(stack.count) >= before + u16::from(consume.count)) else { return Some(NOTHING_THERE) };
        if !recipe.inputs.iter().any(|(cell, _)| cell.accepts(&stack.name, stack.damage)) {
            return Some(NOT_AN_INGREDIENT);
        }
        taken.push((at, consume.count.into()));
    }
    let needed = recipe.inputs.iter().map(|(_, count)| u16::from(*count)).sum::<u16>() * u16::from(asked.times_crafted);
    (taken.iter().map(|(_, count)| count).sum::<u16>() < needed).then_some(TOO_LITTLE)
}
