//! The state BDS gives a block named without states (`/setblock ~ ~ ~ chest`), for the blocks
//! where it is not their first (`data/block-defaults.tsv`, from the `sapling` example
//! `bds_defaults_probe`).

use std::collections::HashMap;
use std::sync::LazyLock;

const ROWS: &str = include_str!("../data/block-defaults.tsv");

static TABLE: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let rows = ROWS.lines().filter(|line| !line.starts_with('#') && !line.is_empty());
    rows.map(|line| line.split_once('\t').unwrap_or_else(|| panic!("a bad block default row: {line}"))).collect()
});

/// The properties of the block `name`'s default state, as the registry writes them; `None` where
/// it is the block's first.
pub fn default_properties(name: &str) -> Option<&'static str> {
    TABLE.get(name).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chest_faces_north_and_stone_is_its_first_state() {
        assert_eq!(default_properties("minecraft:chest"), Some("minecraft:cardinal_direction=north"));
        assert_eq!(default_properties("minecraft:stone"), None);
    }
}
