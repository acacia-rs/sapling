//! `/recipe give|take <player> <recipe|*>`: the recipes each player's recipe book holds, starting
//! with those a new player knows, answered and sent as BDS 1.26.52 does
//! (research/captures/bds-controls.txt, bds-controls2.txt).
// TODO: BDS's UnlockedRecipes for `give *` was cut short in the capture; here it lists every
// recipe the player did not know, in CraftingData's order. The shard keeps the real book (saved,
// filled by picked-up items); this one is not saved and misses those, until sapling-api can read it.

use std::collections::{HashMap, HashSet};

use sapling_api::command::Selector;
use sapling_api::{Command, Output, PlayerId, RecipeChange, ScreenAction, Sender, ShardCtx};

use super::{no_target, players};

const ALL: &str = "*";

#[derive(Default)]
struct Books(HashMap<PlayerId, HashSet<String>>);

fn initial() -> HashSet<String> {
    sapling_data::initial_recipes().iter().cloned().collect()
}

#[derive(Command)]
#[command(crate = "crate", name = "recipe", description = "Unlocks recipe in the recipe book for a player.", permission = "operator")]
pub(crate) enum Recipe {
    Give { player: Selector, recipe: String },
    Take { player: Selector, recipe: String },
}

impl Command for Recipe {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let (selector, recipe, give) = match self {
            Self::Give { player, recipe } => (player, recipe, true),
            Self::Take { player, recipe } => (player, recipe, false),
        };
        let found = players(ctx, sender, Some(&selector));
        if found.is_empty() {
            no_target(out);
            return out.translate("commands.generic.player.notFound", Vec::new(), false);
        }
        if recipe != ALL && !sapling_data::recipe_ids().contains(&recipe) {
            return out.translate("commands.recipes.notFound", Vec::new(), false);
        }
        let mut done = 0;
        for player in found {
            let book = ctx.state::<Books>().0.entry(player.id).or_insert_with(initial);
            let (key, sent) = match (give, recipe == ALL) {
                (true, true) => {
                    let new: Vec<String> = sapling_data::recipe_ids().iter().filter(|id| book.insert((*id).clone())).cloned().collect();
                    ("commands.recipes.unlockedAllRecipes", vec![(new, RecipeChange::Learnt)])
                }
                (false, true) => {
                    *book = initial();
                    let initial = sapling_data::initial_recipes().to_vec();
                    ("commands.recipes.removedAllRecipes", vec![(Vec::new(), RecipeChange::AllForgotten), (initial, RecipeChange::Initial)])
                }
                (true, false) if !book.insert(recipe.clone()) => {
                    out.translate("commands.recipes.playerHasRecipe", vec![player.name, recipe.clone()], false);
                    continue;
                }
                (true, false) => ("commands.recipes.unlockedRecipe", vec![(vec![recipe.clone()], RecipeChange::Learnt)]),
                (false, false) if !book.contains(&recipe) => {
                    out.translate("commands.recipes.playerDoesNotHaveRecipe", vec![player.name, recipe.clone()], false);
                    continue;
                }
                (false, false) => {
                    // A recipe a new player knows comes straight back (bds-controls2.txt).
                    let mut sent = vec![(vec![recipe.clone()], RecipeChange::Forgotten)];
                    if sapling_data::initial_recipes().contains(&recipe) {
                        sent.push((vec![recipe.clone()], RecipeChange::Initial));
                    } else {
                        book.remove(&recipe);
                    }
                    ("commands.recipes.removedRecipe", sent)
                }
            };
            for (ids, change) in sent {
                ctx.screen_action(player.id, &ScreenAction::Recipes { ids, change });
            }
            let shown = if recipe == ALL { vec![player.name] } else { vec![player.name, recipe.clone()] };
            out.translate(key, shown, true);
            done += 1;
        }
        out.done_to(done);
    }
}
