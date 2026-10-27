//! What a fishing hook reels in: BDS's `loot_tables/gameplay/fishing.json`, or
//! `jungle_fishing.json` for a hook spawned in a jungle (the hook's `loot_jungle` group). Its
//! entries name junk, treasure and fish tables, each weighed by its `quality` times the luck
//! (Luck of the Sea's level).

use acacia_proto::nbt::Value;
use serde_json::Value as Json;

use super::LOOT;
use crate::enchant::Draws;
use crate::enchant::table::with_levels;
use crate::gear::{gear, with_damage};
use crate::items::Stack;

const FISHING: &str = "loot_tables/gameplay/fishing.json";
const JUNGLE: &str = "loot_tables/gameplay/jungle_fishing.json";

/// One stack reeled in.
#[derive(Clone, Debug, PartialEq)]
pub struct Caught {
    pub name: String,
    pub count: u8,
    pub data: i16,
    pub tag: Option<Value>,
}

/// What one catch gives, in a jungle or not, with `luck`.
pub fn fishing_loot(jungle: bool, luck: i32, draws: &mut dyn Draws) -> Vec<Caught> {
    let mut caught = Vec::new();
    run(if jungle { JUNGLE } else { FISHING }, luck, draws, &mut caught);
    caught
}

fn run(table: &str, luck: i32, draws: &mut dyn Draws, caught: &mut Vec<Caught>) {
    for pool in LOOT["tables"][table]["pools"].as_array().into_iter().flatten() {
        let entries: Vec<&Json> = pool["entries"].as_array().into_iter().flatten().collect();
        for _ in 0..pool["rolls"].as_u64().unwrap_or(1) {
            match pick(&entries, luck, draws) {
                Some(entry) if entry["type"] == "loot_table" => run(entry["name"].as_str().unwrap_or_default(), luck, draws, caught),
                Some(entry) => caught.extend(item(entry, draws)),
                None => {}
            }
        }
    }
}

/// An entry's weight with the luck: `weight + quality * luck`, never below 0.
fn weight(entry: &Json, luck: i32) -> i32 {
    let (weight, quality) = (entry["weight"].as_i64().unwrap_or(1) as i32, entry["quality"].as_i64().unwrap_or(0) as i32);
    (weight + quality * luck).max(0)
}

fn pick<'a>(entries: &[&'a Json], luck: i32, draws: &mut dyn Draws) -> Option<&'a Json> {
    let total: i32 = entries.iter().map(|entry| weight(entry, luck)).sum();
    let mut at = draws.int(total);
    entries.iter().copied().find(|entry| {
        at -= weight(entry, luck);
        at < 0
    })
}

fn item(entry: &Json, draws: &mut dyn Draws) -> Option<Caught> {
    let mut caught = Caught { name: entry["name"].as_str()?.to_owned(), count: 1, data: 0, tag: None };
    for function in entry["functions"].as_array().into_iter().flatten() {
        match function["function"].as_str().map(|name| name.trim_start_matches("minecraft:")) {
            Some("set_count") => caught.count = function["count"].as_f64().unwrap_or(1.0) as u8,
            Some("set_data") => caught.data = function["data"].as_i64().unwrap_or(0) as i16,
            Some("set_damage") => {
                let lasts = f32::from(gear(&caught.name)?.lasts);
                let [low, high] = [&function["damage"]["min"], &function["damage"]["max"]].map(|end| end.as_f64().unwrap_or(1.0) as f32);
                let left = low + draws.float() * (high - low);
                caught.tag = with_damage(caught.tag.take(), (lasts * (1.0 - left)).floor() as i32);
            }
            Some("enchant_with_levels") => {
                let levels = function["levels"].as_u64().unwrap_or(0) as u8;
                let stack = Stack { name: &caught.name, count: caught.count, damage: caught.data, block: None, tag: caught.tag.as_ref() };
                if let Some(made) = with_levels(&stack, levels, draws) {
                    (caught.name, caught.tag) = (made.name, made.tag);
                }
            }
            _ => {}
        }
    }
    Some(caught)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Draws that always give the same fraction of their bound.
    struct Always(f32);

    impl Draws for Always {
        fn int(&mut self, bound: i32) -> i32 {
            ((bound as f32 * self.0) as i32).min(bound - 1).max(0)
        }
        fn float(&mut self) -> f32 {
            self.0
        }
    }

    fn names(jungle: bool, luck: i32, at: f32) -> Vec<String> {
        fishing_loot(jungle, luck, &mut Always(at)).into_iter().map(|caught| caught.name).collect()
    }

    #[test]
    fn a_catch_is_junk_treasure_or_fish_by_weight_and_luck() {
        assert_eq!(names(false, 0, 0.0), ["minecraft:leather_boots"], "the lowest roll: the first junk");
        assert_eq!(names(false, 0, 0.999), ["minecraft:pufferfish"], "the highest: the last fish");
        assert_eq!(names(false, 0, 0.5), ["minecraft:cod"]);
        // Luck 3: junk 4, treasure 11, fish 82 of 97.
        assert_eq!(names(false, 3, 0.1), ["minecraft:nautilus_shell"]);
        assert_eq!(names(true, 0, 0.999), ["minecraft:pufferfish"]);
        let boots = &fishing_loot(false, 0, &mut Always(0.0))[0];
        assert_eq!(crate::gear::damage(boots.tag.as_ref()), 65, "worn to none of its 65 uses left");
    }
}
