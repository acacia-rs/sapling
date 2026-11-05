//! The item data a loot table's functions set on a stack: `set_data`, `set_potion` (a tipped
//! arrow's data is its potion's + 1) and `enchant_randomly`, which takes one `nextInt(30)` of the
//! loot's random as its cost and the enchantments from a thread's own random
//! (`EnchantRandomlyFunction` vtable 14a8fbe40, apply 14752bc20, then `EnchantUtils::randomlyEnchant`
//! 141b62a20); and a mob's equipment's `enchant_random_gear`.

use std::hash::{BuildHasher, RandomState};

use acacia_proto::nbt::Value as Nbt;
use serde_json::Value;

use crate::enchant::Draws;
use crate::enchant::table::with_levels;
use crate::items::Stack;

/// `enchant_randomly`'s cost bound.
const RANDOM_COST: i32 = 30;
const TIPPED_ARROW: &str = "minecraft:arrow";

/// `set_potion`'s names and each potion's data value.
const POTIONS: [(&str, i16); 23] = [
    ("water", 0), ("nightvision", 5), ("invisibility", 7), ("leaping", 9), ("fire_resistance", 12), ("swiftness", 14), ("slowness", 17), ("water_breathing", 19),
    ("healing", 21), ("harming", 23), ("poison", 25), ("regeneration", 28), ("strong_regeneration", 30), ("strength", 31), ("weakness", 34), ("wither", 36),
    ("turtle_master", 37), ("slow_falling", 40), ("strong_slowness", 42), ("wind_charged", 43), ("weaving", 44), ("oozing", 45), ("infested", 46),
];

/// One stack a table gives.
#[derive(Clone, Debug, PartialEq)]
pub struct Looted {
    pub name: String,
    pub count: u32,
    pub data: i16,
    pub tag: Option<Nbt>,
}

/// A thread's own random as BDS keeps one: seeded from the system, so no two runs agree.
struct Unrepeatable(u64);

impl Unrepeatable {
    fn new() -> Self {
        Self(RandomState::new().hash_one(0u8) | 1)
    }

    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 32) as u32
    }
}

impl Draws for Unrepeatable {
    fn int(&mut self, bound: i32) -> i32 {
        if bound < 1 { 0 } else { (self.next() % bound as u32) as i32 }
    }

    fn float(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// The functions [`apply`] takes.
pub(super) const SETS: [&str; 3] = ["set_data", "set_potion", "enchant_randomly"];

/// Applies one of [`SETS`] to `looted`.
pub(super) fn apply(function: &Value, looted: &mut Looted, draws: &mut dyn Draws) {
    match function["function"].as_str().map(|name| name.trim_start_matches("minecraft:")) {
        Some("set_data") => looted.data = function["data"].as_i64().unwrap_or(0) as i16,
        Some("set_potion") => {
            if let Some(&(_, data)) = POTIONS.iter().find(|(name, _)| function["id"].as_str() == Some(name)) {
                looted.data = if looted.name == TIPPED_ARROW { data + 1 } else { data };
            }
        }
        Some("enchant_randomly") => {
            let cost = draws.int(RANDOM_COST) as u8;
            let stack = Stack { name: &looted.name, count: looted.count.min(64) as u8, damage: looted.data, block: None, tag: looted.tag.as_ref() };
            if let Some(made) = with_levels(&stack, cost, function["treasure"].as_bool().unwrap_or(false), &mut Unrepeatable::new()) {
                (looted.name, looted.tag) = (made.name, made.tag);
            }
        }
        _ => {}
    }
}

/// `enchant_random_gear` (apply 1412863a0): with `chance` times the special `multiplier`, `looted`
/// enchanted at 5 more than `multiplier` of a `nextInt(18)`, both drawn from the loot's random.
pub(super) fn enchant_gear(chance: f32, multiplier: f32, looted: &mut Looted, draws: &mut dyn Draws) {
    if draws.float() >= chance * multiplier {
        return;
    }
    let cost = (multiplier * draws.int(18) as f32 + 5.0) as u8;
    let stack = Stack { name: &looted.name, count: looted.count.min(64) as u8, damage: looted.data, block: None, tag: looted.tag.as_ref() };
    if let Some(made) = with_levels(&stack, cost, false, &mut Unrepeatable::new()) {
        (looted.name, looted.tag) = (made.name, made.tag);
    }
}
