//! Each entity's families (`minecraft:type_family`), from BDS's behaviour packs (`data/mob-families.tsv`,
//! README.md "Mob families").

/// The families of `identifier` (`minecraft:zombie`: `zombie undead monster mob`); none for a kind
/// the packs give none (items, falling blocks).
pub fn mob_families(identifier: &str) -> Vec<&'static str> {
    let row = include_str!("../data/mob-families.tsv").lines().filter_map(|row| row.split_once('\t')).find(|(name, _)| *name == identifier);
    row.map_or_else(Vec::new, |(_, families)| families.split(' ').collect())
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
}
