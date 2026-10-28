//! A player's recipe book, as BDS's UnlockedRecipesServerComponent keeps it and its
//! RecipeUnlockingSystem fills it (research/captures/bds-recipeunlock.txt, bds-recipeunlock2.txt):
//! a stack landing in or growing in a slot of the main inventory unlocks the recipes its item is
//! an unlocking ingredient of, in any game mode and whatever `recipesUnlock` says (the offhand
//! unlocks nothing); a state of the player (in water, holding many items) unlocks its recipes once
//! ever. The client hears of a tick's new recipes in one `NewlyUnlocked` list, in recipe order.
//! The book goes with the player record (`recipe_unlocking`) and to the client whole as it joins.
//! `doLimitedCrafting` only pauses unlocking: BDS crafted a recipe the book lacked with it on.

use std::collections::HashSet;

use acacia_proto::packets::{UnlockedRecipes, UnlockedRecipesUnlockType as Unlock};
use bevy_ecs::prelude::*;
use sapling_api::RecipeChange;
use sapling_data::UnlockContext;
use sapling_world::RecipeRecord;

use super::{Inventory, MAIN, Place};
use crate::body::{Body, InLiquid, Liquid, in_liquid};
use crate::components::{InLayer, Layer, Outgoing, Position};

const LIMITED_CRAFTING: &str = "doLimitedCrafting";
const MANY_SLOTS: usize = 10;
const PLAYER_SIZE: [f32; 2] = [0.6, 1.8];

/// What a slot holds, as far as unlocking goes: name, damage, count.
type Seen = Option<(String, i16, u8)>;

#[derive(Component, Clone)]
pub(crate) struct RecipeBook {
    /// What it holds, in the order it learnt them.
    known: Vec<String>,
    lookup: HashSet<String>,
    /// [`UnlockContext::bit`]s of the states already met.
    used_contexts: u32,
    /// What each main slot held when unlocking last looked at it, and the tick before.
    seen: Vec<Seen>,
    last: Vec<Seen>,
    sent: bool,
    pub unsaved: bool,
}

impl Default for RecipeBook {
    /// A new player's book.
    fn default() -> Self {
        let mut book = Self { known: Vec::new(), lookup: HashSet::new(), used_contexts: 0, seen: Vec::new(), last: Vec::new(), sent: false, unsaved: false };
        book.learn(sapling_data::initial_recipes().iter().map(String::as_str));
        book.unsaved = false;
        book
    }
}

impl RecipeBook {
    /// Adds recipes; those it did not know, in order.
    fn learn<'a>(&mut self, ids: impl IntoIterator<Item = &'a str>) -> Vec<String> {
        let new: Vec<String> = ids.into_iter().filter(|id| self.lookup.insert((*id).to_owned())).map(str::to_owned).collect();
        self.known.extend(new.iter().cloned());
        self.unsaved |= !new.is_empty();
        new
    }

    fn forget(&mut self, ids: &[String]) {
        self.lookup.retain(|id| !ids.contains(id));
        self.known.retain(|id| !ids.contains(id));
        self.unsaved = true;
    }

    fn knows(&self, id: &str) -> bool {
        self.lookup.contains(id)
    }

    /// Follows what `/recipe` told the client.
    pub fn changed(&mut self, ids: &[String], change: RecipeChange) {
        match change {
            RecipeChange::Initial | RecipeChange::Learnt => drop(self.learn(ids.iter().map(String::as_str))),
            RecipeChange::Forgotten => self.forget(ids),
            RecipeChange::AllForgotten => self.forget(&self.known.clone()),
            _ => {}
        }
    }

    /// The book a record kept, the items it was kept with taken as already looked at. A record
    /// without a book leaves the new player's, and its items then unlock as newly held.
    pub fn restore(&mut self, kept: Option<RecipeRecord>, inventory: &Inventory) {
        let Some(kept) = kept else { return };
        (self.known, self.lookup, self.used_contexts, self.seen) = (Vec::new(), HashSet::new(), kept.used_contexts as u32, held(inventory));
        self.last = self.seen.clone();
        self.learn(kept.unlocked.iter().map(String::as_str));
        self.unsaved = false;
    }

    pub fn kept(&self) -> RecipeRecord {
        RecipeRecord { unlocked: self.known.clone(), used_contexts: self.used_contexts as i32 }
    }
}

fn held(inventory: &Inventory) -> Vec<Seen> {
    let slot = |slot: usize| inventory.slot(Place::Main(slot)).cloned().flatten().map(|held| (held.stack.name, held.stack.damage, held.stack.count));
    (0..MAIN).map(slot).collect()
}

/// Sends each player its whole book once its items are shown, as the first list of a join.
pub(crate) fn send_book(mut players: Query<(&Inventory, &mut RecipeBook, &mut Outgoing)>) {
    for (inventory, mut book, mut out) in &mut players {
        if inventory.sent && !book.sent {
            out.push(&UnlockedRecipes { unlock_type: Unlock::InitiallyUnlocked, recipes: book.known.clone() });
            book.sent = true;
        }
    }
}

/// Unlocks what the stacks new or grown in players' slots, on a tick they changed, and the states
/// players are in unlock. While a world does limited crafting nothing unlocks there; the slots
/// that changed meanwhile unlock theirs at the first change after it stops (bds-limitedcraft.txt:
/// planks given and sticks crafted with it on unlocked their recipes at the next craft).
pub(crate) fn unlock(world: &mut World) {
    let players: Vec<Entity> = world.query_filtered::<Entity, With<RecipeBook>>().iter(world).collect();
    for player in players {
        let Some(layer) = world.get::<InLayer>(player).map(|layer| crate::dimensions::home(world, layer.0)) else { continue };
        let Some(now) = world.get::<Inventory>(player).filter(|inventory| inventory.loaded).map(held) else { continue };
        let limited = crate::rules::on(world, layer, LIMITED_CRAFTING);
        let met = if limited { Vec::new() } else { met_contexts(world, player, &now) };
        let mut query = world.query::<(&mut RecipeBook, &mut Outgoing)>();
        let Ok((mut book, mut out)) = query.get_mut(world, player) else { continue };
        let changed = std::mem::replace(&mut book.last, now.clone()) != now;
        if limited {
            continue;
        }
        let contexts: Vec<UnlockContext> = met.into_iter().filter(|context| book.used_contexts & context.bit() == 0).collect();
        let before = if changed { std::mem::replace(&mut book.seen, now.clone()) } else { now.clone() };
        let landed: Vec<(&str, i16)> = now.iter().enumerate().filter(|(slot, held)| held.is_some() && before.get(*slot) != Some(*held)).filter_map(|(_, held)| held.as_ref().map(|(name, damage, _)| (name.as_str(), *damage))).collect();
        if contexts.is_empty() && landed.is_empty() {
            continue;
        }
        book.used_contexts |= contexts.iter().fold(0, |bits, context| bits | context.bit());
        book.unsaved |= !contexts.is_empty();
        let new: Vec<String> = sapling_data::unlocked_by(&landed, &contexts).into_iter().filter(|id| !book.knows(id)).map(str::to_owned).collect();
        book.learn(new.iter().map(String::as_str));
        if !new.is_empty() && book.sent {
            out.push(&UnlockedRecipes { unlock_type: Unlock::NewlyUnlocked, recipes: new });
        }
    }
}

/// The unlocking states a player is in: ten slots or more filled (the tenth slot's item brought
/// the chest; 74 items in nine slots had not), and water its box is in.
fn met_contexts(world: &World, player: Entity, held: &[Seen]) -> Vec<UnlockContext> {
    let many = held.iter().flatten().count() >= MANY_SLOTS;
    let level = world.get::<InLayer>(player).and_then(|layer| world.get::<Layer>(layer.0));
    let feet = world.get::<Position>(player).map(|position| position.feet);
    let wet = level.zip(feet).is_some_and(|(level, feet)| in_liquid(&level.world, &Body::new(feet, [0.0; 3], PLAYER_SIZE), InLiquid::Sinks, Liquid::Water));
    [(many, UnlockContext::ManyItems), (wet, UnlockContext::InWater)].into_iter().filter(|(met, _)| *met).map(|(_, context)| context).collect()
}
