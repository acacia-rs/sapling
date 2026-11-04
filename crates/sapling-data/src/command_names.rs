//! The item and block names BDS's commands take (`data/command-names.tsv`, README.md "Provenance"):
//! the `Item` and `Block` enums of its AvailableCommands, each with and without `minecraft:`. A name
//! outside them is BDS's `commands.generic.syntax` (research/captures/bds-item-names.txt, bds-names.txt).

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

// Empty until the probe has run (build.rs).
const ROWS: &str = include_str!(concat!(env!("OUT_DIR"), "/command-names.tsv"));

static NAMES: LazyLock<[HashSet<&'static str>; 2]> = LazyLock::new(|| {
    let mut names = [HashSet::new(), HashSet::new()];
    for row in ROWS.lines() {
        match row.split_once('\t') {
            Some(("item", name)) => drop(names[0].insert(name)),
            Some(("block", name)) => drop(names[1].insert(name)),
            _ => {}
        }
    }
    names
});

fn takes(names: &HashSet<&str>, name: &str) -> bool {
    names.is_empty() || names.contains(name)
}

/// Whether BDS's commands take `name` (as typed) for an item; every name without the table.
pub fn command_takes_item(name: &str) -> bool {
    takes(&NAMES[0], name) || crate::custom_items::by_name(name).is_some()
}

/// Whether BDS's commands take `name` (as typed) for a block; every name without the table.
pub fn command_takes_block(name: &str) -> bool {
    takes(&NAMES[1], name) || crate::custom_items::by_name(name).is_some_and(|item| item.block.is_some())
}

static RULES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| ROWS.lines().filter_map(|row| row.strip_prefix("rule\t")).collect());

/// Whether `/place featurerule` takes `name`: BDS's `featureRules` enum, `minecraft:` and all; every
/// name without the table.
pub fn command_takes_feature_rule(name: &str) -> bool {
    takes(&RULES, name)
}

static TILES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    ROWS.lines().filter_map(|row| row.strip_prefix("tile\t")?.split_once('\t')).collect()
});

/// The key BDS's answers name the block `name` (`minecraft:stone`) by, in its default state
/// (`%tile.stone.stone.name`); `None` without the table or for a block it does not list.
pub fn block_tile_key(name: &str) -> Option<&'static str> {
    TILES.get(name).copied()
}
