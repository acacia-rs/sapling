//! `/recipe give|take <player> <recipe|*>`: the recipes each player's recipe book holds, starting
//! with those a new player knows, answered and sent as BDS 1.26.52 does
//! (research/captures/bds-controls.txt, bds-controls2.txt).
//! `give *` gives what BDS gave, in its order (`sapling_data::given_recipe_ids`, bds-t1drecipe.txt).
//! The shard's book (saved, filled by picked-up items) is read where the shard keeps one; `Books`
//! only stands in for a backend without books.

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
            let kept = ctx.known_recipes(player.id).map(|known| known.into_iter().collect::<HashSet<String>>());
            let book = ctx.state::<Books>().0.entry(player.id).or_insert_with(initial);
            if let Some(kept) = kept {
                *book = kept;
            }
            let (key, sent) = match (give, recipe == ALL) {
                (true, true) => {
                    let new: Vec<String> = sapling_data::given_recipe_ids().iter().filter(|id| book.insert((*id).clone())).cloned().collect();
                    // BDS sent nothing to a book that had them all (bds-t1drecipe.txt).
                    let sent = if new.is_empty() { Vec::new() } else { vec![(new, RecipeChange::Learnt)] };
                    ("commands.recipes.unlockedAllRecipes", sent)
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
