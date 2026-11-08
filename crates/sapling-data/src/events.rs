//! Each entity's events, from BDS's behaviour packs (`data/mob-events.tsv`, README.md "Mob events").

/// Whether `identifier`'s definition has the event `event`, by the name the definition gives it
/// (`minecraft:entity_born`; the sheep's `spawn_baby` has no namespace).
pub fn has_event(identifier: &str, event: &str) -> bool {
    include_str!("../data/mob-events.tsv").lines().filter_map(|row| row.split_once('\t')).find(|(name, _)| *name == identifier).is_some_and(|(_, events)| events.split(' ').any(|own| own == event))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_are_named_as_the_packs_name_them() {
        assert!(has_event("minecraft:sheep", "minecraft:entity_born") && has_event("minecraft:sheep", "spawn_baby"));
        assert!(has_event("minecraft:villager_v2", "minecraft:become_cartographer") && has_event("minecraft:player", "minecraft:gain_raid_omen"));
        assert!(!has_event("minecraft:sheep", "entity_born") && !has_event("minecraft:cow", "minecraft:temperate_color") && !has_event("minecraft:nope", "minecraft:entity_born"));
    }
}
