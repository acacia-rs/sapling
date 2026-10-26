//! The trade tables of villagers and the wandering trader: BDS's `trading/economy_trades/*.json`
//! (`data/trades.json`, made by `data/generate/trades.py`, README "Trades"), and the offers a
//! trader draws from one of their tiers (roll.rs).

mod roll;

use std::collections::HashMap;
use std::sync::LazyLock;

use serde_json::Value;

pub use roll::{ANY_DAMAGE, Offer, Ware, roll};

/// A table: what each tier of a trader offers, and the experience it takes to reach it.
#[derive(Debug)]
pub struct Table {
    pub tiers: Vec<Tier>,
}

#[derive(Debug)]
pub struct Tier {
    /// The trader experience (`TradeExperience`) this tier takes.
    pub exp: i32,
    groups: Vec<Group>,
}

/// Trades of which a tier offers `select`, all of them for -1.
#[derive(Debug)]
struct Group {
    select: i32,
    trades: Vec<Trade>,
}

#[derive(Debug)]
struct Trade {
    /// Each item, as the alternatives it is drawn from (a `choice`, or one).
    wants: Vec<Vec<Item>>,
    gives: Vec<Vec<Item>>,
    trader_exp: i32,
    max_uses: i32,
    reward_exp: bool,
    weight: u32,
}

#[derive(Debug)]
struct Item {
    name: String,
    data: Option<i16>,
    count: [u8; 2],
    price_multiplier: f32,
    functions: Vec<Function>,
    /// The trader types (`MarkVariant`) it is offered by; all without.
    marks: Option<Vec<i32>>,
}

/// What a table's `functions` do to an item, of those its tables use (`set_potion` the generator
/// turns into the item's data).
#[derive(Debug, Clone, PartialEq)]
enum Function {
    EnchantWithLevels([u8; 2]),
    /// The price its book asks: a base, a random part, and both again for each level.
    EnchantBook { base: i32, random: i32, per_level: i32, per_level_random: i32 },
    RandomAux([i16; 2]),
    RandomDye,
    /// A map to a structure, which sapling cannot locate: its trade is never offered.
    ExplorationMap,
}

static TABLES: LazyLock<HashMap<String, Table>> = LazyLock::new(|| {
    let all: Value = serde_json::from_str(include_str!("../data/trades.json")).expect("trades.json is JSON");
    all.as_object().into_iter().flatten().map(|(name, table)| (name.clone(), parse_table(table))).collect()
});

/// The table of the file `name` names (`armorer` for `armorer_trades.json`).
pub fn trade_table(name: &str) -> Option<&'static Table> {
    TABLES.get(name)
}

impl Table {
    /// The experience each tier takes, the first's 0.
    pub fn exp_needed(&self) -> Vec<i32> {
        self.tiers.iter().map(|tier| tier.exp).collect()
    }
}

fn int(value: &Value) -> i64 {
    value.as_i64().unwrap_or(0)
}

fn pair(value: &Value) -> [i64; 2] {
    [int(&value["min"]), int(&value["max"])]
}

fn function(value: &Value) -> Option<Function> {
    Some(match value["function"].as_str()? {
        "enchant_with_levels" => Function::EnchantWithLevels(pair(&value["levels"]).map(|level| level.clamp(0, 255) as u8)),
        "enchant_book_for_trading" => Function::EnchantBook {
            base: int(&value["base_cost"]) as i32,
            random: int(&value["base_random_cost"]) as i32,
            per_level: int(&value["per_level_cost"]) as i32,
            per_level_random: int(&value["per_level_random_cost"]) as i32,
        },
        "random_aux_value" => Function::RandomAux(pair(&value["values"]).map(|aux| aux as i16)),
        "random_dye" => Function::RandomDye,
        "exploration_map" => Function::ExplorationMap,
        _ => return None,
    })
}

fn item(value: &Value) -> Item {
    let count = value["count"].as_array().map_or([1, 1], |count| [0, 1].map(|at| count.get(at).map_or(1, int).clamp(0, 255) as u8));
    Item {
        name: value["name"].as_str().unwrap_or_default().to_owned(),
        data: value["data"].as_i64().map(|data| data as i16),
        count,
        price_multiplier: value["price_multiplier"].as_f64().unwrap_or(0.0) as f32,
        functions: value["functions"].as_array().into_iter().flatten().filter_map(function).collect(),
        marks: value["marks"].as_array().map(|marks| marks.iter().map(|mark| int(mark) as i32).collect()),
    }
}

fn items(value: &Value) -> Vec<Vec<Item>> {
    value.as_array().into_iter().flatten().map(|choice| choice.as_array().into_iter().flatten().map(item).collect()).collect()
}

fn parse_table(table: &Value) -> Table {
    let trade = |trade: &Value| Trade {
        wants: items(&trade["wants"]),
        gives: items(&trade["gives"]),
        trader_exp: int(&trade["trader_exp"]) as i32,
        max_uses: int(&trade["max_uses"]) as i32,
        reward_exp: trade["reward_exp"].as_bool().unwrap_or(true),
        weight: int(&trade["weight"]).max(1) as u32,
    };
    let group = |group: &Value| Group { select: int(&group["select"]) as i32, trades: group["trades"].as_array().into_iter().flatten().map(trade).collect() };
    let tier = |tier: &Value| Tier { exp: int(&tier["exp"]) as i32, groups: tier["groups"].as_array().into_iter().flatten().map(group).collect() };
    Table { tiers: table["tiers"].as_array().into_iter().flatten().map(tier).collect() }
}

#[cfg(test)]
mod tests;
