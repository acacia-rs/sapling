//! Grid crafts BDS allows and sapling once refused (docs/research/containers-crafting.md, E3 and
//! E4): a shift-click making more than a stack, and a recipe with two outputs.

use acacia_proto::types::{
    ContainerSlotType as Container, ItemStackRequestActionsItemContent as Action, ItemStackRequestActionsItemContentCraftRecipe as Craft,
    ItemStackRequestActionsItemContentCreate as Create, ItemStackResponsesItemStatus as Status,
};

use crate::common::Harness;
use crate::common::requests::{answer, consume, place, request, slot};
use crate::special_recipes::{kept, lay, stack, steve};

/// The network id of the hand or table recipe making `made` whose first cell takes `from`.
fn recipe_making(made: &str, from: &str) -> u32 {
    let fits = |recipe: &sapling_data::Recipe| recipe.outputs[0].0.name == made && recipe.inputs.iter().any(|(cell, _)| cell.accepts(from, 0));
    (0..20_000).find(|id| sapling_data::recipe(*id).is_some_and(fits)).expect("the recipe")
}

fn send(h: &mut Harness, actions: Vec<Action>) -> Status {
    h.receive(1, &request(-9, actions));
    answer(&h.tick(1)).0
}

#[test]
fn seventeen_logs_shift_crafted_make_68_planks_the_surplus_waiting_behind_the_output() {
    let (mut h, storage) = steve(vec![stack("minecraft:oak_log", 17, 0, None)], false);
    assert_eq!(send(&mut h, vec![place(17, slot(Container::Hotbar, 0, 0), slot(Container::CraftingInput, 28, 0))]), Status::Ok);
    let recipe_network_id = recipe_making("minecraft:oak_planks", "minecraft:oak_log");
    let output = || slot(Container::CreativeOutput, 50, 0);
    let actions = vec![
        Action::CraftRecipe(Craft { recipe_network_id, times_crafted: 17 }),
        consume(17, slot(Container::CraftingInput, 28, 0)),
        place(64, output(), slot(Container::HotbarAndInventory, 2, 0)),
        place(4, output(), slot(Container::HotbarAndInventory, 3, 0)),
    ];
    assert_eq!(send(&mut h, actions), Status::Ok);
    let kept = kept(h, &storage);
    let count = |slot: usize| kept.main[slot].as_ref().map(|planks| (planks.name.as_str(), planks.count));
    assert_eq!((count(2), count(3), count(0)), (Some(("minecraft:oak_planks", 64)), Some(("minecraft:oak_planks", 4)), None));
}

#[test]
fn a_cake_is_made_with_its_three_buckets_given_back() {
    let milk = || stack("minecraft:milk_bucket", 1, 0, None);
    let held = vec![milk(), milk(), milk(), stack("minecraft:sugar", 2, 0, None), stack("minecraft:egg", 1, 0, None), stack("minecraft:wheat", 3, 0, None)];
    let (mut h, storage) = steve(held, true);
    let laid = [(0, 32), (1, 33), (2, 34), (3, 35), (4, 36), (3, 37), (5, 38), (5, 39), (5, 40)];
    laid.iter().for_each(|(from, cell)| lay(&mut h, *from, *cell));
    let recipe_network_id = recipe_making("minecraft:cake", "minecraft:milk_bucket");
    let output = || slot(Container::CreativeOutput, 50, 0);
    let mut actions = vec![Action::CraftRecipe(Craft { recipe_network_id, times_crafted: 1 })];
    actions.extend(laid.iter().map(|(_, cell)| consume(1, slot(Container::CraftingInput, *cell, 0))));
    actions.extend([Action::Create(Create { result_slot_id: 0 }), place(1, output(), slot(Container::HotbarAndInventory, 6, 0))]);
    actions.extend([Action::Create(Create { result_slot_id: 1 }), place(3, output(), slot(Container::HotbarAndInventory, 7, 0))]);
    assert_eq!(send(&mut h, actions), Status::Ok);
    let kept = kept(h, &storage);
    let count = |slot: usize| kept.main[slot].as_ref().map(|made| (made.name.as_str(), made.count));
    assert_eq!((count(6), count(7)), (Some(("minecraft:cake", 1)), Some(("minecraft:bucket", 3))));
}
