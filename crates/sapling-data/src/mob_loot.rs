//! What a mob leaves when it dies: BDS's own loot tables (`data/mob-loot.json`, made by
//! `data/generate/mob-loot.py`), run as for a death no player, pet or other entity caused, of a
//! mob, without Looting: what `/kill` leaves, cooked when the mob was on fire.
// TODO: item data the tables set (tipped arrows, damaged gear, enchantments) is dropped; the item
// is left plain.

use std::sync::LazyLock;

use serde_json::Value;

use crate::recipes::{Station, smelted};

static LOOT: LazyLock<Value> = LazyLock::new(|| serde_json::from_str(include_str!("../data/mob-loot.json")).expect("mob-loot.json is JSON"));

/// What a mob of `kind` leaves when it dies, `on_fire` or not, as (item, count); `roll` gives
/// numbers in [0, 1). Nothing for a kind without a table.
pub fn mob_loot(kind: &str, on_fire: bool, roll: &mut dyn FnMut() -> f32) -> Vec<(String, u32)> {
    let mut left = Vec::new();
    if let Some(table) = LOOT["entities"][kind].as_str() {
        run(table, on_fire, roll, &mut left);
    }
    left
}

fn run(table: &str, on_fire: bool, roll: &mut dyn FnMut() -> f32, left: &mut Vec<(String, u32)>) {
    let Some(pools) = LOOT["tables"][table]["pools"].as_array() else { return };
    for pool in pools {
        if !holds(&pool["conditions"], on_fire, roll) {
            continue;
        }
        for _ in 0..number(&pool["rolls"], roll) {
            let mut entries = Vec::new();
            for entry in pool["entries"].as_array().into_iter().flatten() {
                if holds(&entry["conditions"], on_fire, roll) {
                    entries.push(entry);
                }
            }
            if let Some(entry) = pick(&entries, roll) {
                take(entry, on_fire, roll, left);
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

fn take(entry: &Value, on_fire: bool, roll: &mut dyn FnMut() -> f32, left: &mut Vec<(String, u32)>) {
    let Some(name) = entry["name"].as_str() else { return };
    match entry["type"].as_str() {
        Some("item") => {
            let (mut name, mut count) = (name, 1);
            for function in entry["functions"].as_array().into_iter().flatten() {
                // Looting adds per level, and there is none; the other functions change only the item's data.
                match function["function"].as_str().map(|name| name.trim_start_matches("minecraft:")) {
                    Some("set_count") if holds(&function["conditions"], on_fire, roll) => count = number(&function["count"], roll),
                    Some("furnace_smelt") if holds(&function["conditions"], on_fire, roll) => name = smelted(Station::Furnace, name, 0).map_or(name, |(made, _)| made.name),
                    _ => {}
                }
            }
            if count > 0 {
                left.push((name.to_owned(), count));
            }
        }
        Some("loot_table") => run(name, on_fire, roll, left),
        _ => {}
    }
}

/// Whether every condition holds for a death no one caused, of an adult.
fn holds(conditions: &Value, on_fire: bool, roll: &mut dyn FnMut() -> f32) -> bool {
    conditions.as_array().into_iter().flatten().all(|condition| match condition["condition"].as_str() {
        Some("random_chance" | "random_chance_with_looting") => roll() < condition["chance"].as_f64().unwrap_or(0.0) as f32,
        Some("entity_properties") => on_fire && condition["entity"] == "this" && condition["properties"] == serde_json::json!({"on_fire": true}),
        _ => false,
    })
}

/// A whole number, or one from `{"min", "max"}` inclusive.
fn number(value: &Value, roll: &mut dyn FnMut() -> f32) -> u32 {
    if let Some(n) = value.as_u64() {
        return n as u32;
    }
    let (min, max) = (value["min"].as_u64().unwrap_or(0) as u32, value["max"].as_u64().unwrap_or(0) as u32);
    min + ((roll() * (max.saturating_sub(min) + 1) as f32) as u32).min(max.saturating_sub(min))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn always(at: f32) -> impl FnMut() -> f32 {
        move || at
    }

    #[test]
    fn every_item_a_table_leaves_is_one_the_client_knows() {
        let mut unknown = Vec::new();
        for table in LOOT["tables"].as_object().unwrap().values() {
            for entry in table["pools"].as_array().into_iter().flatten().flat_map(|pool| pool["entries"].as_array().into_iter().flatten()) {
                let name = entry["name"].as_str().unwrap_or_default();
                if entry["type"] == "item" && !crate::is_item(name) {
                    unknown.push(name.to_owned());
                }
            }
        }
        assert!(unknown.is_empty(), "{unknown:?}");
    }

    #[test]
    fn a_chicken_leaves_feathers_by_chance_and_its_meat() {
        assert_eq!(mob_loot("minecraft:chicken", false, &mut always(0.0)), [("minecraft:chicken".to_owned(), 1)], "no feathers on the lowest roll");
        assert_eq!(mob_loot("minecraft:chicken", false, &mut always(0.999)), [("minecraft:feather".to_owned(), 2), ("minecraft:chicken".to_owned(), 1)]);
    }

    #[test]
    fn a_mob_on_fire_leaves_its_meat_cooked() {
        assert_eq!(mob_loot("minecraft:chicken", true, &mut always(0.0)), [("minecraft:cooked_chicken".to_owned(), 1)]);
        assert_eq!(mob_loot("minecraft:pig", true, &mut always(0.0)), [("minecraft:cooked_porkchop".to_owned(), 1)]);
    }

    #[test]
    fn what_only_a_player_kill_leaves_is_not_left() {
        let zombie: Vec<String> = mob_loot("minecraft:zombie", false, &mut always(0.0)).into_iter().map(|(name, _)| name).collect();
        assert!(!zombie.iter().any(|name| name == "minecraft:iron_ingot"), "{zombie:?}");
        assert!(mob_loot("minecraft:armor_stand", false, &mut always(0.5)).is_empty() && mob_loot("minecraft:nothing", false, &mut always(0.5)).is_empty());
        assert_eq!(mob_loot("minecraft:sheep", false, &mut always(0.0)), [("minecraft:white_wool".to_owned(), 1), ("minecraft:mutton".to_owned(), 1)]);
    }
}
