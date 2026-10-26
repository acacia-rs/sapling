//! Each entity's families (`minecraft:type_family`), from BDS's behaviour packs (`data/mob-families.tsv`,
//! README.md "Mob families").

fn row(identifier: &str) -> Option<&'static str> {
    include_str!("../data/mob-families.tsv").lines().filter_map(|row| row.split_once('\t')).find(|(name, _)| *name == identifier).map(|(_, families)| families)
}

/// The families of `identifier` (`minecraft:zombie`: `zombie undead monster mob`); none for a kind
/// the packs give none (items, falling blocks).
pub fn mob_families(identifier: &str) -> Vec<&'static str> {
    row(identifier).map_or_else(Vec::new, |families| families.split(' ').collect())
}

/// Whether `identifier`'s families include `family`.
pub fn has_family(identifier: &str, family: &str) -> bool {
    row(identifier).is_some_and(|families| families.split(' ').any(|f| f == family))
}

/// Whether a mob of `kind` is undead: instant health harms it, Smite hurts it more.
pub fn is_undead(kind: &str) -> bool {
    has_family(kind, "undead")
}

/// Whether a mob of `kind` is an arthropod, which Bane of Arthropods hurts more.
pub fn is_arthropod(kind: &str) -> bool {
    has_family(kind, "arthropod")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families_are_the_packs() {
        assert_eq!(mob_families("minecraft:player"), ["player"]);
        assert_eq!(mob_families("minecraft:zombie"), ["zombie", "undead", "monster", "mob"]);
        assert!(mob_families("minecraft:item").is_empty());
    }

    #[test]
    fn undead_and_arthropods_come_from_the_families() {
        for undead in ["minecraft:zombie_villager_v2", "minecraft:skeleton_horse", "minecraft:wither", "minecraft:phantom"] {
            assert!(is_undead(undead), "{undead}");
        }
        for arthropod in ["minecraft:bee", "minecraft:spider", "minecraft:endermite"] {
            assert!(is_arthropod(arthropod), "{arthropod}");
        }
        assert!(!is_undead("minecraft:spider") && !is_arthropod("minecraft:zombie") && !is_undead("minecraft:nope"));
    }
}
