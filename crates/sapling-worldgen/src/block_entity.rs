//! What a generated block holds beyond its state, as BDS's generator gives it: a spawner's mob, a
//! container's loot table and seed, a trial spawner's configs, a vault's reward. Each is filed with the
//! cell it belongs to and kept only while that cell holds a block of its kind (`fits`); what BDS saves
//! for each: docs/research/structure-block-entities.md.

/// One generated block entity; tables are paths in the behaviour packs (`loot_tables/chests/igloo_chest.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockEntity {
    /// A monster spawner of this mob (`minecraft:zombie`).
    Spawner(&'static str),
    /// A chest, barrel, dispenser or suspicious block whose table is rolled when it is first opened or brushed.
    Loot { table: &'static str, seed: i32 },
    /// A decorated pot: its loot, if any, and its four faces when they are not plain brick.
    Pot { table: Option<&'static str>, seed: i32, sherds: Option<[&'static str; 4]> },
    /// A trial spawner's configs (`minecraft:trial_chamber/breeze/normal` and `.../ominous`).
    TrialSpawner { normal: &'static str, ominous: &'static str },
    /// A vault's reward table and the key it takes (`minecraft:trial_key`).
    Vault { table: &'static str, key: &'static str },
}

impl BlockEntity {
    /// The same with its loot seeded `seed`; kinds without loot are unchanged.
    pub fn seeded(self, seed: i32) -> Self {
        match self {
            Self::Loot { table, .. } => Self::Loot { table, seed },
            Self::Pot { table, sherds, .. } => Self::Pot { table, seed, sherds },
            other => other,
        }
    }

    /// Whether a block named `name` (no namespace) holds this kind of entity.
    pub fn fits(&self, name: &str) -> bool {
        match self {
            Self::Spawner(_) => name == "mob_spawner",
            Self::Loot { .. } => {
                matches!(name, "chest" | "trapped_chest" | "barrel" | "dispenser" | "suspicious_sand" | "suspicious_gravel") || name.ends_with("copper_chest")
            }
            Self::Pot { .. } => name == "decorated_pot",
            Self::TrialSpawner { .. } => name == "trial_spawner",
            Self::Vault { .. } => name == "vault",
        }
    }
}

/// Whether BDS's block actor for a block named `name` is a randomizable container (`BlockActor`
/// +0x20), which a v2 template placement draws a `LootTableSeed` for even without a table: an empty
/// barrel does, the hopper above it does not (trial chambers, oracle 20261003-structures).
pub fn randomizable(name: &str) -> bool {
    matches!(name, "chest" | "trapped_chest" | "barrel" | "dispenser" | "dropper" | "decorated_pot" | "suspicious_sand" | "suspicious_gravel") || name.ends_with("copper_chest")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_entity_fits_only_its_blocks() {
        let loot = BlockEntity::Loot { table: "loot_tables/chests/simple_dungeon.json", seed: 1 };
        assert!(loot.fits("chest") && loot.fits("oxidized_copper_chest") && !loot.fits("stone"));
        assert!(BlockEntity::Spawner("minecraft:zombie").fits("mob_spawner") && !BlockEntity::Spawner("minecraft:zombie").fits("chest"));
    }

    #[test]
    fn a_dungeon_keeps_bds_mob_and_chest_seed() {
        // The dungeon at 123,-20,270 of the oracle world 20261003-structures, as BDS saved it.
        let entities = crate::OverworldGenerator::new(20261003).chunk_blocks(7, 16).block_entities();
        assert!(entities.contains(&([11, -20, 14], BlockEntity::Spawner("minecraft:spider"))), "{entities:?}");
        assert!(entities.contains(&([13, -20, 14], BlockEntity::Loot { table: "loot_tables/chests/simple_dungeon.json", seed: 357601146 })), "{entities:?}");
    }
}
