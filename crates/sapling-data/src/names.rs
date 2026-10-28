//! The names BDS gives items and entities in command answers (`data/names.tsv`, README.md
//! "Provenance"): `Raw Chicken` for `minecraft:chicken`, `Potion of Slowness` for a potion of data 17.

use std::collections::HashMap;
use std::sync::LazyLock;

// Empty until the probe has run (build.rs).
const ROWS: &str = include_str!(concat!(env!("OUT_DIR"), "/names.tsv"));

struct Names {
    items: HashMap<(&'static str, u16), &'static str>,
    entities: HashMap<&'static str, &'static str>,
}

static NAMES: LazyLock<Names> = LazyLock::new(|| {
    let mut names = Names { items: HashMap::new(), entities: HashMap::new() };
    for row in ROWS.lines() {
        match row.split('\t').collect::<Vec<_>>()[..] {
            ["item", identifier, data, name] => drop(names.items.insert((identifier, data.parse().unwrap_or(0)), name)),
            ["entity", identifier, name] => drop(names.entities.insert(identifier, name)),
            _ => {}
        }
    }
    names
});

/// What BDS calls the item `identifier` (namespaced) of `data`, or of data 0 where its name is the same for all.
pub fn item_display_name(identifier: &str, data: u16) -> Option<&'static str> {
    let recorded = NAMES.items.get(&(identifier, data)).or_else(|| NAMES.items.get(&(identifier, 0))).copied();
    recorded.or_else(|| crate::custom_items::by_name(identifier).map(|item| item.display_name))
}

/// What BDS calls an entity of `identifier` (namespaced).
pub fn entity_display_name(identifier: &str) -> Option<&'static str> {
    NAMES.entities.get(identifier).copied()
}
