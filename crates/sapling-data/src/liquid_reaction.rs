//! What water does to each block, as BDS registers its vanilla blocks
//! (`data/liquid-reactions.tsv`; how it was read: sapling's docs/research/waterlogging.md).

use std::collections::HashMap;
use std::sync::LazyLock;

const ROWS: &str = include_str!("../data/liquid-reactions.tsv");

/// What water running into a block does to it (BDS `LiquidReaction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiquidReaction {
    /// Replaced, leaving nothing.
    Broken,
    /// Replaced, leaving its loot.
    Popped,
    /// Stops the water.
    Blocking,
    /// Stays, and takes the water in beside itself (layer 1).
    Noreaction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquidDetection {
    pub reaction: LiquidReaction,
    /// Placed into a water source, it keeps the source beside itself.
    pub can_contain: bool,
}

/// What a block without a rule of its own is: BDS's `Block` default.
const DEFAULT: LiquidDetection = LiquidDetection { reaction: LiquidReaction::Blocking, can_contain: false };

static TABLE: LazyLock<HashMap<&'static str, LiquidDetection>> = LazyLock::new(|| {
    let rows = ROWS.lines().filter(|line| !line.starts_with('#') && !line.is_empty());
    rows.map(|line| {
        let [name, reaction, can_contain] = line.split('\t').collect::<Vec<_>>()[..] else { panic!("a bad liquid reaction row: {line}") };
        let reaction = match reaction {
            "broken" => LiquidReaction::Broken,
            "popped" => LiquidReaction::Popped,
            "blocking" => LiquidReaction::Blocking,
            "noreaction" => LiquidReaction::Noreaction,
            _ => panic!("a bad liquid reaction: {line}"),
        };
        (name, LiquidDetection { reaction, can_contain: can_contain == "1" })
    })
    .collect()
});

/// What water does to the block `name` (namespaced).
pub fn liquid_detection(name: &str) -> LiquidDetection {
    TABLE.get(name).copied().unwrap_or(DEFAULT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stairs_keep_water_grass_is_washed_away_and_stone_stops_it() {
        let detection = |name: &str| liquid_detection(&format!("minecraft:{name}"));
        assert_eq!(detection("oak_stairs"), LiquidDetection { reaction: LiquidReaction::Blocking, can_contain: true });
        assert_eq!(detection("short_grass").reaction, LiquidReaction::Popped);
        assert_eq!([detection("stone"), detection("not_a_block")], [DEFAULT; 2]);
        assert_eq!(detection("air"), LiquidDetection { reaction: LiquidReaction::Noreaction, can_contain: true });
    }
}
