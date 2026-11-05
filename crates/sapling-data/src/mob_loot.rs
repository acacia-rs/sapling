//! What a mob leaves when it dies: BDS's own loot tables (`data/mob-loot.json`, made by
//! `data/generate/mob-loot.py`), run for a death no one caused or one a player caused with the
//! Looting it held (enchant.rs), of a mob, cooked when the mob was on fire. [`drawn_table_loot`]
//! runs a table in BDS's draw order (`LootPool::addRandomItems` 14127b8e0), for a container's seeded loot.
// TODO: only the seeded tables keep item data (item_data.rs); a mob's loot is left plain. A pet's kill counts as no one's.

use std::sync::LazyLock;

use serde_json::Value;

use crate::enchant::{self, Draws};
use crate::recipes::{Station, smelted};

mod item_data;
mod regional;

pub use item_data::Looted;
pub use regional::{Regional, moon_fullness, special_multiplier};

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
    /// Where a born mob's equipment is rolled; tables run for no mob's birth know no difficulty.
    regional: Option<Regional>,
}

impl Death {
    fn looting(self) -> u8 {
        match self.killer {
            Killer::Player { looting } => looting,
            Killer::None => 0,
        }
    }
}

/// Draws as numbers in [0, 1) give them: `int(n)` is the fraction times n.
struct Fractions<'a>(&'a mut dyn FnMut() -> f32);

impl Draws for Fractions<'_> {
    fn int(&mut self, bound: i32) -> i32 {
        if bound < 1 { 0 } else { (((self.0)() * bound as f32) as i32).min(bound - 1) }
    }

    fn float(&mut self) -> f32 {
        (self.0)()
    }
}

/// A roll's draws. In BDS's order a count the table writes as one number still draws `nextInt(1)`,
/// and every pool draws a `nextFloat` for its bonus rolls after its rolls.
struct Roll<'a> {
    draws: &'a mut dyn Draws,
    bds_order: bool,
}

impl Roll<'_> {
    /// A whole number, or one from `{"min", "max"}` inclusive.
    fn number(&mut self, value: &Value) -> u32 {
        // Some tables write a count as `1.0`.
        if let Some(n) = value.as_f64() {
            if self.bds_order {
                self.draws.int(1);
            }
            return n.max(0.0) as u32;
        }
        let (min, max) = (value["min"].as_u64().unwrap_or(0) as u32, value["max"].as_u64().unwrap_or(0) as u32);
        min + self.draws.int((max.saturating_sub(min) + 1) as i32) as u32
    }
}

/// What a mob of `kind` leaves when it dies, `on_fire` or not, killed by `killer`, as (item,
/// count); `roll` gives numbers in [0, 1). Nothing for a kind without a table.
pub fn mob_loot(kind: &str, on_fire: bool, killer: Killer, roll: &mut dyn FnMut() -> f32) -> Vec<(String, u32)> {
    let mut left = Vec::new();
    if let Some(table) = LOOT["entities"][kind].as_str() {
        run(table, Death { on_fire, killer, regional: None }, &mut Roll { draws: &mut Fractions(roll), bds_order: false }, &mut left);
    }
    counted(left)
}

fn counted(left: Vec<Looted>) -> Vec<(String, u32)> {
    left.into_iter().map(|looted| (looted.name, looted.count)).collect()
}

/// What the table at `table` (its path in the packs, `loot_tables/...json`) gives a block that
/// ejects it (a trial spawner, a vault), as [`mob_loot`] gives it. Nothing for a table not kept.
pub fn table_loot(table: &str, roll: &mut dyn FnMut() -> f32) -> Vec<(String, u32)> {
    table_loot_for(table, Killer::None, roll)
}

/// [`table_loot`] as `killer` sees it, as `/loot ... loot <table> <tool>` runs a table.
pub fn table_loot_for(table: &str, killer: Killer, roll: &mut dyn FnMut() -> f32) -> Vec<(String, u32)> {
    let mut left = Vec::new();
    run(table, Death { on_fire: false, killer, regional: None }, &mut Roll { draws: &mut Fractions(roll), bds_order: false }, &mut left);
    counted(left)
}

/// What a mob's equipment table gives it at its birth under `regional`, enchanted where it says.
pub fn gear_loot(table: &str, regional: Regional, roll: &mut dyn FnMut() -> f32) -> Vec<Looted> {
    let mut left = Vec::new();
    run(table, Death { on_fire: false, killer: Killer::None, regional: Some(regional) }, &mut Roll { draws: &mut Fractions(roll), bds_order: false }, &mut left);
    left
}

/// [`table_loot`] drawing from BDS's `Random` in BDS's order, the stacks in the order BDS makes
/// them, with the item data the tables set (item_data.rs).
pub fn drawn_table_loot(table: &str, draws: &mut dyn Draws) -> Vec<Looted> {
    let mut left = Vec::new();
    run(table, Death { on_fire: false, killer: Killer::None, regional: None }, &mut Roll { draws, bds_order: true }, &mut left);
    left
}

/// Whether BDS has the table at `table`: every one of its packs' is kept.
pub fn has_loot_table(table: &str) -> bool {
    LOOT["tables"].get(table).is_some()
}

fn run(table: &str, death: Death, roll: &mut Roll, left: &mut Vec<Looted>) {
    run_pools(&LOOT["tables"][table]["pools"], death, roll, left);
}

/// A table's pools, or those an item entry runs after its item (an armour set's next piece).
fn run_pools(pools: &Value, death: Death, roll: &mut Roll, left: &mut Vec<Looted>) {
    for pool in pools.as_array().into_iter().flatten() {
        if !holds(&pool["conditions"], death, roll) {
            continue;
        }
        if pool.get("tiers").is_some() {
            if let Some(entry) = regional::tier(pool, roll.draws) {
                take(entry, death, roll, left);
            }
            continue;
        }
        let rolls = roll.number(&pool["rolls"]);
        if roll.bds_order {
            roll.draws.float();
        }
        for _ in 0..rolls {
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
fn pick<'a>(entries: &[&'a Value], roll: &mut Roll) -> Option<&'a Value> {
    let weight = |entry: &Value| entry["weight"].as_u64().unwrap_or(1) as i32;
    let total: i32 = entries.iter().map(|entry| weight(entry)).sum();
    if total < 1 {
        return None;
    }
    let mut at = roll.draws.int(total);
    entries.iter().copied().find(|entry| {
        at -= weight(entry);
        at < 0
    })
}

fn take(entry: &Value, death: Death, roll: &mut Roll, left: &mut Vec<Looted>) {
    let Some(name) = entry["name"].as_str() else { return };
    match entry["type"].as_str() {
        Some("item") => {
            let mut looted = Looted { name: name.to_owned(), count: 1, data: 0, tag: None };
            for function in entry["functions"].as_array().into_iter().flatten() {
                match function["function"].as_str().map(|name| name.trim_start_matches("minecraft:")) {
                    Some("set_count") if holds(&function["conditions"], death, roll) => looted.count = roll.number(&function["count"]),
                    Some("furnace_smelt") if holds(&function["conditions"], death, roll) => {
                        if let Some((made, _)) = smelted(Station::Furnace, &looted.name, 0) {
                            looted.name = made.name.to_owned();
                        }
                    }
                    Some("looting_enchant") if death.looting() > 0 => {
                        let range = [&function["count"]["min"], &function["count"]["max"]].map(|end| end.as_f64().unwrap_or(0.0) as f32);
                        looted.count += enchant::looted(range, death.looting(), &mut || roll.draws.float());
                        if let Some(limit) = function["limit"].as_u64().filter(|limit| *limit > 0) {
                            looted.count = looted.count.min(limit as u32);
                        }
                    }
                    Some(function_name) if item_data::SETS.contains(&function_name) => {
                        if holds(&function["conditions"], death, roll) {
                            item_data::apply(function, &mut looted, roll.draws);
                        }
                    }
                    Some("enchant_random_gear") => {
                        if let Some(regional) = death.regional {
                            item_data::enchant_gear(function["chance"].as_f64().unwrap_or(0.0) as f32, regional.multiplier, &mut looted, roll.draws);
                        }
                    }
                    // TODO: the other functions' item data (damage, trims), and their draws.
                    _ => {}
                }
            }
            if looted.count > 0 {
                left.push(looted);
                run_pools(&entry["pools"], death, roll, left);
            }
        }
        Some("loot_table") => run(name, death, roll, left),
        _ => {}
    }
}

/// Whether every condition holds for this death of an adult.
fn holds(conditions: &Value, death: Death, roll: &mut Roll) -> bool {
    let by_player = matches!(death.killer, Killer::Player { .. });
    conditions.as_array().into_iter().flatten().all(|condition| match condition["condition"].as_str() {
        Some("random_chance") => roll.draws.float() < condition["chance"].as_f64().unwrap_or(0.0) as f32,
        Some("random_chance_with_looting") => {
            let [chance, per_level] = [&condition["chance"], &condition["looting_multiplier"]].map(|value| value.as_f64().unwrap_or(0.0) as f32);
            roll.draws.float() < enchant::looting_chance(chance, per_level, death.looting())
        }
        Some("random_difficulty_chance") => death.regional.is_some_and(|regional| roll.draws.float() < regional::difficulty_chance(condition, regional.difficulty)),
        Some("random_regional_difficulty_chance") => death.regional.is_some_and(|regional| roll.draws.float() < condition["max_chance"].as_f64().unwrap_or(0.0) as f32 * regional.multiplier),
        Some("killed_by_player" | "killed_by_player_or_pets") => by_player,
        Some("killed_by_entity") => by_player && condition["entity_type"] == "minecraft:player",
        Some("entity_properties") => death.on_fire && condition["entity"] == "this" && condition["properties"] == serde_json::json!({"on_fire": true}),
        _ => false,
    })
}

mod fishing;

pub use fishing::{Caught, fishing_loot};

#[cfg(test)]
mod tests;
