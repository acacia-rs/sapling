//! What each block withstands of a blast, as BDS holds it (`data/block-resistance.tsv`, read out of
//! a running BDS by `data/generate/block-resistance.py`).

use std::collections::HashMap;
use std::sync::LazyLock;

const ROWS: &str = include_str!("../data/block-resistance.tsv");

static TABLE: LazyLock<HashMap<&'static str, f32>> = LazyLock::new(|| {
    let rows = ROWS.lines().filter(|line| !line.starts_with('#') && !line.is_empty());
    rows.map(|line| {
        let (name, value) = line.split_once('\t').unwrap_or_else(|| panic!("a bad block resistance row: {line}"));
        (name, value.parse().unwrap_or_else(|_| panic!("a bad block resistance: {line}")))
    })
    .collect()
});

/// The explosion resistance of the block `name`, for every state of it; `None` for a block BDS
/// does not have.
pub fn blast_resistance(name: &str) -> Option<f32> {
    TABLE.get(name).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stone_obsidian_and_bedrock_withstand_what_bds_gives_them() {
        assert_eq!(blast_resistance("minecraft:stone"), Some(6.0));
        assert_eq!(blast_resistance("minecraft:obsidian"), Some(1200.0));
        assert_eq!(blast_resistance("minecraft:bedrock"), Some(3_600_000.0));
        assert_eq!(blast_resistance("minecraft:water"), Some(100.0));
        assert_eq!(blast_resistance("minecraft:nothing"), None);
    }
}
