//! What blocks leave when broken, as data: the block loot tables of the Java game
//! (`data/block-loot.tsv`, README.md "Block loot"), with the items in Bedrock's names. Rows keep
//! Java's block ids and block-state names; [`java_block`] gives the id of a Bedrock block.

use std::collections::HashMap;
use std::sync::LazyLock;

const ROWS: &str = include_str!("../data/block-loot.tsv");
const NOTHING: &str = include_str!("../data/block-loot-nothing.txt");
/// Bedrock block names that are not the Java id: Bedrock, Java.
const RENAMED: &[(&str, &str)] = &[
    ("web", "cobweb"),
    ("deadbush", "dead_bush"),
    ("melon_block", "melon"),
    ("snow_layer", "snow"),
    ("snow", "snow_block"),
    ("wooden_door", "oak_door"),
    ("quartz_ore", "nether_quartz_ore"),
    ("lit_redstone_ore", "redstone_ore"),
    ("lit_deepslate_redstone_ore", "deepslate_redstone_ore"),
    ("grass_path", "dirt_path"),
    ("mob_spawner", "spawner"),
    ("reeds", "sugar_cane"),
    ("beetroot", "beetroots"),
    ("unpowered_repeater", "repeater"),
    ("powered_repeater", "repeater"),
    ("unpowered_comparator", "comparator"),
    ("powered_comparator", "comparator"),
    ("standing_sign", "oak_wall_sign"),
    ("wall_sign", "oak_wall_sign"),
    ("standing_banner", "white_banner"),
    ("wall_banner", "white_wall_banner"),
    ("lit_furnace", "furnace"),
    ("lit_blast_furnace", "blast_furnace"),
    ("lit_smoker", "smoker"),
    ("lit_redstone_lamp", "redstone_lamp"),
    ("unlit_redstone_torch", "redstone_torch"),
    ("waterlily", "lily_pad"),
    ("brick_block", "bricks"),
    ("trip_wire", "tripwire"),
    ("noteblock", "note_block"),
    ("stonecutter_block", "stonecutter"),
    ("slime", "slime_block"),
    ("end_bricks", "end_stone_bricks"),
    ("magma", "magma_block"),
    ("nether_brick", "nether_bricks"),
    ("red_nether_brick", "red_nether_bricks"),
    ("undyed_shulker_box", "shulker_box"),
    ("hardened_clay", "terracotta"),
    ("frog_spawn", "frogspawn"),
];

/// Which tool a row asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tooling {
    Any,
    SilkTouch,
    NoSilkTouch,
    Shears,
    NoShears,
    ShearsOrSilkTouch,
    NoShearsNoSilkTouch,
}

/// How Fortune changes a row's count, in the Java game's formulas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fortune {
    None,
    /// Times one more than a throw below level + 2, less one, never below zero.
    OreDrops,
    /// Plus a throw up to this many times the level.
    UniformBonus(u8),
    /// The least plus as many of `extra` + level tries as succeed with chance `p`.
    Binomial { extra: u8, p: f32 },
}

/// A block state a row asks for, in Java's names: one of `values`, or none of them.
#[derive(Debug, Clone, PartialEq)]
pub struct StateTest {
    pub key: &'static str,
    pub values: Vec<&'static str>,
    pub negated: bool,
}

/// One thing a block may leave. Rows are independent: each whose conditions hold drops.
#[derive(Debug, Clone, PartialEq)]
pub struct LootRow {
    pub tooling: Tooling,
    pub states: Vec<StateTest>,
    /// Only when an entity broke the block, not when it lost its hold.
    pub by_entity: bool,
    pub pickaxe: bool,
    /// Only when the row before it left nothing.
    pub otherwise: bool,
    /// Namespaced, in Bedrock's name.
    pub item: &'static str,
    pub min: u8,
    pub max: u8,
    /// The chance by Fortune level; the last holds for every higher level.
    pub chances: Vec<f32>,
    pub fortune: Fortune,
    /// The count is kept within these after Fortune.
    pub limit: (u8, u8),
}

pub enum BlockLoot {
    /// One of the block's own item.
    Itself,
    Nothing,
    Rows(&'static [LootRow]),
}

fn tooling(word: &str) -> Option<Tooling> {
    Some(match word {
        "default" => Tooling::Any,
        "silk_touch" => Tooling::SilkTouch,
        "no_silk_touch" => Tooling::NoSilkTouch,
        "shears" => Tooling::Shears,
        "no_shears" => Tooling::NoShears,
        "shears_or_silk_touch" => Tooling::ShearsOrSilkTouch,
        "no_shears_no_silk_touch" => Tooling::NoShearsNoSilkTouch,
        _ => return None,
    })
}

/// The number after `key` in a row's notes: `chance_by_fortune=0.1,0.2` or `limit_count=1..4`.
fn noted<'a>(notes: &'a str, key: &str) -> Option<&'a str> {
    let rest = notes.split_once(key)?.1;
    Some(rest.split([' ', ';']).next().unwrap_or(rest))
}

/// `None` for a row whose item Bedrock lacks or whose count this cannot work out.
fn row(line: &'static str) -> Option<(&'static str, LootRow)> {
    let cells: Vec<&'static str> = line.split('\t').collect();
    let &[block, condition, _, min, max, chance, fortune, notes, item, known] = &cells[..] else { return None };
    if known != "yes" || notes.contains("count~binomial") {
        return None;
    }
    let mut row = LootRow {
        tooling: Tooling::Any,
        states: Vec::new(),
        by_entity: false,
        pickaxe: false,
        otherwise: notes.contains("else-branch"),
        item,
        min: min.parse().ok()?,
        max: max.parse().ok()?,
        chances: noted(notes, "chance_by_fortune=").map_or_else(|| vec![chance.parse().unwrap_or(1.0)], |list| list.split(',').filter_map(|chance| chance.parse().ok()).collect()),
        fortune: match fortune.split_once(':') {
            None if fortune == "ore_drops" => Fortune::OreDrops,
            Some(("uniform_bonus", times)) => Fortune::UniformBonus(times.parse().ok()?),
            Some(("binomial", of)) => of.split_once(',').and_then(|(extra, p)| Some(Fortune::Binomial { extra: extra.parse().ok()?, p: p.parse().ok()? }))?,
            _ => Fortune::None,
        },
        limit: noted(notes, "limit_count=").and_then(|limit| limit.split_once("..")).map_or((0, u8::MAX), |(least, most)| (least.parse().unwrap_or(0), most.parse().unwrap_or(u8::MAX))),
    };
    for word in condition.split(';') {
        match (tooling(word), word.split_once("!=").map(|test| (test, true)).or_else(|| word.split_once('=').map(|test| (test, false)))) {
            (Some(tooling), _) => row.tooling = tooling,
            (None, _) if word == "entity_breaker" => row.by_entity = true,
            (None, Some((("tool", _), _))) => row.pickaxe = true,
            (None, Some(((key, values), negated))) => row.states.push(StateTest { key, values: values.split('|').collect(), negated }),
            (None, None) => return None,
        }
    }
    Some((block, row))
}

static TABLES: LazyLock<HashMap<&'static str, Vec<LootRow>>> = LazyLock::new(|| {
    let mut tables: HashMap<&'static str, Vec<LootRow>> = HashMap::new();
    for (block, row) in ROWS.lines().skip(1).filter_map(row) {
        tables.entry(block).or_default().push(row);
    }
    // Blocks whose table is empty or missing; the rest of the file's blocks have rows above.
    for (block, reason) in NOTHING.lines().skip(1).filter_map(|line| line.split_once('\t')) {
        if reason.ends_with("loot_table") {
            tables.entry(block).or_default();
        }
    }
    tables
});

/// The Java id of a Bedrock block (either without the namespace), and whether it is the double
/// of a slab, which Java keeps as a state (`type=double`) of the slab.
pub fn java_block(bedrock: &str) -> (&str, bool) {
    if let Some(wood) = bedrock.strip_suffix("_double_slab") {
        return (SLABS.iter().find(|slab| slab.strip_suffix("_slab") == Some(wood)).copied().unwrap_or(bedrock), true);
    }
    // A sign of either kind leaves what Java's wall sign of its wood leaves: the wood's sign.
    if let Some(wood) = bedrock.strip_suffix("_standing_sign").or_else(|| bedrock.strip_suffix("_wall_sign")) {
        let wood = if wood == "darkoak" { "dark_oak" } else { wood };
        if let Some(java) = TABLES.keys().find(|block| block.strip_suffix("_wall_sign") == Some(wood)) {
            return (java, false);
        }
    }
    (RENAMED.iter().find(|(from, _)| *from == bedrock).map_or(bedrock, |(_, java)| java), false)
}

static SLABS: LazyLock<Vec<&'static str>> = LazyLock::new(|| TABLES.keys().filter(|block| block.ends_with("_slab")).copied().collect());

/// What the Java block `block` leaves.
pub fn block_loot(block: &str) -> BlockLoot {
    match TABLES.get(block) {
        None => BlockLoot::Itself,
        Some(rows) if rows.is_empty() => BlockLoot::Nothing,
        Some(rows) => BlockLoot::Rows(rows),
    }
}

/// The highest `age` the rows of a Java block ask for: the age at which its crop is ripe.
pub fn ripe_age(block: &str) -> Option<u8> {
    let ages = TABLES.get(block)?.iter().flat_map(|row| &row.states).filter(|test| test.key == "age");
    ages.flat_map(|test| &test.values).filter_map(|age| age.parse().ok()).max()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(block: &str) -> &'static [LootRow] {
        match block_loot(block) {
            BlockLoot::Rows(rows) => rows,
            _ => panic!("{block} has no rows"),
        }
    }

    #[test]
    fn the_tables_read_as_the_game_wrote_them() {
        assert!(TABLES.len() > 500, "{} blocks", TABLES.len());
        assert!(matches!(block_loot("dirt"), BlockLoot::Itself) && matches!(block_loot("spawner"), BlockLoot::Nothing));
        let diamond = rows("diamond_ore");
        assert_eq!((diamond[0].tooling, diamond[0].item, diamond[0].fortune), (Tooling::NoSilkTouch, "minecraft:diamond", Fortune::OreDrops));
        let [flint, gravel, _] = rows("gravel") else { panic!() };
        assert_eq!((flint.chances.clone(), gravel.otherwise, flint.otherwise), (vec![0.1, 0.14285715, 0.25, 1.0], true, false));
        assert_eq!((rows("glowstone")[0].limit, rows("glowstone")[0].fortune), ((1, 4), Fortune::UniformBonus(1)));
        let wheat = rows("wheat");
        assert!(wheat.iter().any(|row| row.item == "minecraft:wheat" && row.states[0] == StateTest { key: "age", values: vec!["7"], negated: false }));
        assert!(matches!(wheat.iter().find(|row| row.max == 4).unwrap().fortune, Fortune::Binomial { extra: 3, .. }));
        assert_eq!((ripe_age("wheat"), ripe_age("beetroots"), ripe_age("stone")), (Some(7), Some(3), None));
        assert!(rows("snow").iter().all(|row| row.by_entity) && rows("glass")[0].tooling == Tooling::SilkTouch);
    }

    #[test]
    fn bedrock_blocks_find_their_java_tables() {
        assert_eq!([java_block("stone"), java_block("melon_block"), java_block("oak_double_slab"), java_block("lit_redstone_ore")], [("stone", false), ("melon", false), ("oak_slab", true), ("redstone_ore", false)]);
        // Each renamed block must leave something nameable: by a table, or as its own item under either name.
        let is_item = |name: &str| crate::is_item(&format!("minecraft:{name}"));
        let lost: Vec<_> = RENAMED.iter().filter(|(bedrock, java)| !TABLES.contains_key(java) && !is_item(bedrock) && !is_item(java)).collect();
        assert!(lost.is_empty(), "{lost:?}");
    }
}
