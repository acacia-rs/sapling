//! What a mob leaves when it dies: BDS's own loot tables (`data/mob-loot.json`, made by
//! `data/generate/mob-loot.py`), run for a death no one caused or one a player caused with the
//! Looting it held (enchant.rs), of a mob, cooked when the mob was on fire.
// TODO: item data the tables set (tipped arrows, damaged gear, enchantments) is dropped; the item
// is left plain. A pet's kill counts as no one's.

use std::sync::LazyLock;

use serde_json::Value;

use crate::enchant;
use crate::recipes::{Station, smelted};

static LOOT: LazyLock<Value> = LazyLock::new(|| serde_json::from_str(include_str!("../data/mob-loot.json")).expect("mob-loot.json is JSON"));

/// Who killed a mob, as its loot table asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Killer {
    None,
    /// A player, with the level of Looting on what it held.
    Player { looting: u8 },
}

#[derive(Clone, Copy)]
struct Death {
    on_fire: bool,
    killer: Killer,
}

impl Death {
    fn looting(self) -> u8 {
        match self.killer {
            Killer::Player { looting } => looting,
            Killer::None => 0,
        }
    }
}

/// What a mob of `kind` leaves when it dies, `on_fire` or not, killed by `killer`, as (item,
/// count); `roll` gives numbers in [0, 1). Nothing for a kind without a table.
pub fn mob_loot(kind: &str, on_fire: bool, killer: Killer, roll: &mut dyn FnMut() -> f32) -> Vec<(String, u32)> {
    let mut left = Vec::new();
    if let Some(table) = LOOT["entities"][kind].as_str() {
        run(table, Death { on_fire, killer }, roll, &mut left);
    }
    left
}

/// What the table at `table` (its path in the packs, `loot_tables/...json`) gives a block that
/// ejects it (a trial spawner, a vault), as [`mob_loot`] gives it. Nothing for a table not kept.
pub fn table_loot(table: &str, roll: &mut dyn FnMut() -> f32) -> Vec<(String, u32)> {
    let mut left = Vec::new();
    run(table, Death { on_fire: false, killer: Killer::None }, roll, &mut left);
    left
}

fn run(table: &str, death: Death, roll: &mut dyn FnMut() -> f32, left: &mut Vec<(String, u32)>) {
    let Some(pools) = LOOT["tables"][table]["pools"].as_array() else { return };
    for pool in pools {
        if !holds(&pool["conditions"], death, roll) {
            continue;
        }
        for _ in 0..number(&pool["rolls"], roll) {
            let mut entries = Vec::new();
            for entry in pool["entries"].as_array().into_iter().flatten() {
                if holds(&entry["conditions"], death, roll) {
                    entries.push(entry);
                }
            }
            if let Some(entry) = pick(&entries, roll) {
                take(entry, death, roll, left);
            }
        }
    }
}

/// One entry, by weight (1 when unsaid).
fn pick<'a>(entries: &[&'a Value], roll: &mut dyn FnMut() -> f32) -> Option<&'a Value> {
    let weight = |entry: &Value| entry["weight"].as_u64().unwrap_or(1);
    let total: u64 = entries.iter().map(|entry| weight(entry)).sum();
    let mut at = (roll() * total as f32) as u64;
    entries.iter().copied().find(|entry| {
        let found = at < weight(entry);
        at = at.saturating_sub(weight(entry));
        found
    })
}

fn take(entry: &Value, death: Death, roll: &mut dyn FnMut() -> f32, left: &mut Vec<(String, u32)>) {
    let Some(name) = entry["name"].as_str() else { return };
    match entry["type"].as_str() {
        Some("item") => {
            let (mut name, mut count) = (name, 1);
            for function in entry["functions"].as_array().into_iter().flatten() {
                // The other functions change only the item's data.
                match function["function"].as_str().map(|name| name.trim_start_matches("minecraft:")) {
                    Some("set_count") if holds(&function["conditions"], death, roll) => count = number(&function["count"], roll),
                    Some("furnace_smelt") if holds(&function["conditions"], death, roll) => name = smelted(Station::Furnace, name, 0).map_or(name, |(made, _)| made.name),
                    Some("looting_enchant") if death.looting() > 0 => {
                        let range = [&function["count"]["min"], &function["count"]["max"]].map(|end| end.as_f64().unwrap_or(0.0) as f32);
                        count += enchant::looted(range, death.looting(), roll);
                        if let Some(limit) = function["limit"].as_u64().filter(|limit| *limit > 0) {
                            count = count.min(limit as u32);
                        }
                    }
                    _ => {}
                }
            }
            if count > 0 {
                left.push((name.to_owned(), count));
            }
        }
        Some("loot_table") => run(name, death, roll, left),
        _ => {}
    }
}

/// Whether every condition holds for this death of an adult.
fn holds(conditions: &Value, death: Death, roll: &mut dyn FnMut() -> f32) -> bool {
    let by_player = matches!(death.killer, Killer::Player { .. });
    conditions.as_array().into_iter().flatten().all(|condition| match condition["condition"].as_str() {
        Some("random_chance") => roll() < condition["chance"].as_f64().unwrap_or(0.0) as f32,
        Some("random_chance_with_looting") => {
            let [chance, per_level] = [&condition["chance"], &condition["looting_multiplier"]].map(|value| value.as_f64().unwrap_or(0.0) as f32);
            roll() < enchant::looting_chance(chance, per_level, death.looting())
        }
        Some("killed_by_player" | "killed_by_player_or_pets") => by_player,
        Some("killed_by_entity") => by_player && condition["entity_type"] == "minecraft:player",
        Some("entity_properties") => death.on_fire && condition["entity"] == "this" && condition["properties"] == serde_json::json!({"on_fire": true}),
        _ => false,
    })
}

/// A whole number, or one from `{"min", "max"}` inclusive.
fn number(value: &Value, roll: &mut dyn FnMut() -> f32) -> u32 {
    // Some tables write a count as `1.0`.
    if let Some(n) = value.as_f64() {
        return n.max(0.0) as u32;
    }
    let (min, max) = (value["min"].as_u64().unwrap_or(0) as u32, value["max"].as_u64().unwrap_or(0) as u32);
    min + ((roll() * (max.saturating_sub(min) + 1) as f32) as u32).min(max.saturating_sub(min))
}

mod fishing;

pub use fishing::{Caught, fishing_loot};

#[cfg(test)]
mod tests;
