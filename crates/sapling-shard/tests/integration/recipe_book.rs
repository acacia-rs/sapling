//! The recipe book: what held items and states unlock, kept with the player and sent whole at
//! join (inventory/recipe_book.rs; research/captures/bds-recipeunlock.txt, bds-recipeunlock2.txt,
//! bds-limitedcraft.txt).

use crate::common;

use acacia_proto::RawPacket;
use acacia_proto::packets::{UnlockedRecipes, UnlockedRecipesUnlockType as Unlock};
use common::stored::{enter_stored, floor_holding};
use common::{Harness, all, config};
use sapling_api::{Item, PlayerId, RecipeChange, RuleValue, ScreenAction};
use sapling_world::{MemoryStorage, PlayerData, World, WorldStorage};

const STEVE: PlayerId = PlayerId(1);

fn lists(sent: &[RawPacket]) -> Vec<(Unlock, Vec<String>)> {
    all::<UnlockedRecipes>(sent).into_iter().map(|list| (list.unlock_type, list.recipes)).collect()
}

fn newly(ids: &[&str]) -> Vec<(Unlock, Vec<String>)> {
    vec![(Unlock::NewlyUnlocked, ids.iter().map(|id| (*id).to_owned()).collect())]
}

fn give(h: &mut Harness, name: &'static str, count: u8) -> Vec<RawPacket> {
    h.remote.run(move |ctx| assert_eq!(ctx.give(STEVE, &Item::new(name, count)), 0));
    h.tick(1)
}

fn steve() -> String {
    acacia_auth::login::offline_identity("Steve")
}

/// Steve as storage keeps him, once he left.
fn kept(mut h: Harness, storage: &MemoryStorage) -> PlayerData {
    h.leave(1);
    h.shard.tick();
    h.shard.save_worlds();
    storage.load_player(&steve()).unwrap().unwrap()
}

#[test]
fn held_items_unlock_their_recipes_and_the_book_is_kept() {
    let storage = floor_holding(Vec::new(), &[]);
    storage.store_player(&steve(), &PlayerData::default()).unwrap();
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    let joined = lists(&enter_stored(&mut h, 1));
    assert_eq!(joined, [(Unlock::InitiallyUnlocked, sapling_data::initial_recipes().to_vec())], "a new player's book");

    assert_eq!(lists(&give(&mut h, "minecraft:oak_log", 1)), newly(&["minecraft:furnace_log", "minecraft:oak_planks", "minecraft:oak_wood"]));
    assert!(lists(&give(&mut h, "minecraft:oak_log", 1)).is_empty(), "nothing new");
    // Taken by `/recipe`, a recipe comes back with the next log onto the stack.
    h.remote.run(|ctx| ctx.screen_action(STEVE, &ScreenAction::Recipes { ids: vec!["minecraft:oak_planks".into()], change: RecipeChange::Forgotten }));
    h.tick(1);
    assert_eq!(lists(&give(&mut h, "minecraft:oak_log", 1)), newly(&["minecraft:oak_planks"]));

    // Limited crafting pauses unlocking; the sticks unlock at the first change after it.
    h.remote.run(|ctx| assert!(ctx.set_game_rule("doLimitedCrafting", RuleValue::Bool(true))));
    h.tick(1);
    assert!(lists(&give(&mut h, "minecraft:stick", 4)).is_empty());
    h.remote.run(|ctx| assert!(ctx.set_game_rule("doLimitedCrafting", RuleValue::Bool(false))));
    assert!(lists(&h.tick(1)).is_empty() && lists(&h.tick(1)).is_empty());
    let sticks = lists(&give(&mut h, "minecraft:dirt", 1));
    assert_eq!(sticks.len(), 1);
    assert!(sticks[0].1.starts_with(&["minecraft:torch".to_owned()]) && sticks[0].1.contains(&"Stick_bamboo_recipeId".to_owned()), "{sticks:?}");

    let book = kept(h, &storage).recipes.expect("a book");
    assert!(book.unlocked.contains(&"minecraft:oak_planks".to_owned()) && book.unlocked.contains(&"minecraft:torch".to_owned()));
    assert_eq!(book.unlocked[..2], sapling_data::initial_recipes()[..2]);

    // Back again: the whole book is the join's list, and what was held unlocks nothing anew.
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    let joined = lists(&enter_stored(&mut h, 1));
    assert_eq!(joined, [(Unlock::InitiallyUnlocked, book.unlocked.clone())]);
    assert!(lists(&h.tick(1)).is_empty());
}

#[test]
fn ten_filled_slots_unlock_the_chest_once() {
    let storage = floor_holding(Vec::new(), &[]);
    storage.store_player(&steve(), &PlayerData::default()).unwrap();
    let mut h = Harness::with(config(), World::with_storage(storage.clone()));
    enter_stored(&mut h, 1);
    for name in ["minecraft:dirt", "minecraft:sand", "minecraft:gravel", "minecraft:clay_ball", "minecraft:flint", "minecraft:feather", "minecraft:bone", "minecraft:string"] {
        assert!(!lists(&give(&mut h, name, 1)).iter().any(|(_, ids)| ids.contains(&"minecraft:Chest_recipeId".to_owned())), "{name}");
    }
    let tenth = [give(&mut h, "minecraft:egg", 1), give(&mut h, "minecraft:leather", 1)].concat();
    assert_eq!(lists(&tenth).last().map(|(_, ids)| ids[0].as_str()), Some("minecraft:Chest_recipeId"));
    assert_eq!(kept(h, &storage).recipes.map(|book| book.used_contexts), Some(1 << 3), "PlayerHasManyItems");
}
