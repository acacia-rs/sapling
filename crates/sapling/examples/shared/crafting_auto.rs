//! `!auto`: a recipe-book craft (`CraftRecipeAuto`) forged with Consumes from chosen slots, to see
//! what the server answers a craft that takes from where its recipe book would not.

use acacia_bot::proto::types::{RecipeIngredient2, RecipeIngredient2Content as Cell, RecipeIngredient2ContentName as Name, RecipeIngredient2Type as CellType};
use acacia_bot::state::Ingredient;

use super::{CREATED, HOTBAR, Kind, NEXT, believed, Consume, Content, Place, TypeId, act_of, at, drop_results, empty_hotbar, request};
use acacia_bot::Bot;
use acacia_bot::proto::types::ItemStackRequestActionsItemContentCraftRecipeAuto as Auto;

/// What a slot word names: `n5` HotbarAndInventory 5, `h0` Hotbar 0, `i9` Inventory 9, `c0` the
/// open container's 0, `g28` a crafting-grid UI slot; `*2` consumes two.
fn source(word: &str) -> Option<(Kind, u8, u8)> {
    let (slot, count) = word.split_once('*').map_or((word, 1), |(slot, count)| (slot, count.parse().unwrap_or(1)));
    let kind = match slot.get(..1)? {
        "n" => Kind::HotbarAndInventory,
        "h" => Kind::Hotbar,
        "i" => Kind::Inventory,
        "c" => Kind::Container,
        "g" => Kind::CraftingInput,
        _ => return None,
    };
    Some((kind, slot[1..].parse().ok()?, count))
}

fn named(name: &str) -> RecipeIngredient2 {
    let content = Cell::Name(Name { name: name.to_owned(), metadata: 32767 });
    RecipeIngredient2 { r#type: CellType::Name, legacy_type: CellType::Name.to_raw() as u8, content, count: 1 }
}

/// `!auto item:with times slots...`: the first 2x2 recipe for `item` all of whose cells take
/// `with`, its cells named as the client names them (a tag cell by `with`), crafted `times` with a Consume per slot word,
/// the result placed in an empty hotbar slot and dropped.
pub(super) async fn craft(actor: &mut Bot, words: &[&str]) {
    let word = words.first().copied().unwrap_or("stick");
    let (item, with) = word.split_once(':').unwrap_or((word, "oak_planks"));
    let (item, with) = (format!("minecraft:{item}"), format!("minecraft:{with}"));
    let times: u8 = words.get(1).and_then(|word| word.parse().ok()).unwrap_or(1);
    let fits = |recipe: &&acacia_bot::state::Recipe| recipe.fits_2x2() && recipe.inputs.iter().filter(|cell| !cell.is_empty()).all(|cell| cell.accepts(&with, 0));
    let Some((network_id, cells, made)) = actor.recipes_for(&item).into_iter().find(fits).map(|recipe| {
        let cells: Vec<RecipeIngredient2> = recipe
            .inputs
            .iter()
            .filter(|cell| !cell.is_empty())
            .map(|cell| match cell {
                Ingredient::Item { name, .. } => named(name),
                _ => named(&with),
            })
            .collect();
        (recipe.network_id, cells, recipe.outputs.first().map_or(1, |output| output.count as u8))
    }) else {
        return println!("## no 2x2 recipe for {item}");
    };
    let id = NEXT.load(std::sync::atomic::Ordering::Relaxed);
    let mut actions = vec![act_of(TypeId::CraftRecipeAuto, Content::CraftRecipeAuto(Auto { recipe_network_id: network_id, times_crafted: times, ingredients: cells }))];
    let mut used: Vec<(Kind, u8)> = Vec::new();
    for word in &words[2..] {
        let Some((kind, slot, count)) = source(word) else { return println!("## no slot {word}") };
        // A slot already consumed from is named by the request's own id, as the client does.
        let stack_id = if used.contains(&(kind, slot)) { id } else if slot < 36 && matches!(kind, Kind::HotbarAndInventory | Kind::Hotbar | Kind::Inventory) { believed(actor, slot).0 } else { 0 };
        used.push((kind, slot));
        actions.push(act_of(TypeId::Consume, Content::Consume(Consume { count, source: at(kind, slot, stack_id) })));
    }
    let dests: Vec<u8> = empty_hotbar(actor).into_iter().filter(|slot| *slot < HOTBAR).take(1).collect();
    let Some(&dest) = dests.first() else { return println!("## no empty hotbar slot") };
    actions.push(act_of(TypeId::Place, Content::Place(Place { count: made * times, source: at(Kind::CreativeOutput, CREATED, id), destination: at(Kind::Hotbar, dest, 0) })));
    let (ok, slots, _) = request(actor, actions, Vec::new()).await;
    println!("## auto {item} x{times}: {ok}");
    if ok {
        drop_results(actor, &slots, &dests).await;
    }
}
