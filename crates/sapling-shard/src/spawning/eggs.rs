//! Spawn eggs: one used on a block hatches its kind standing in the middle of the cell before the
//! face clicked, whatever the light, the rules or the game rules say (`doMobSpawning` is for
//! natural spawning alone).

use bevy_ecs::prelude::*;
use sapling_api::{BlockPos, SpawnReason};

const EGG: &str = "_spawn_egg";
/// Eggs not named for the kind BDS hatches from them.
const RENAMED: &[(&str, &str)] = &[
    ("minecraft:evoker", "minecraft:evocation_illager"),
    ("minecraft:tropical_fish", "minecraft:tropicalfish"),
    ("minecraft:villager", "minecraft:villager_v2"),
    ("minecraft:zombie_villager", "minecraft:zombie_villager_v2"),
];

/// The kind the egg `item` hatches; `None` for an item that is not one.
pub(crate) fn kind_of(item: &str) -> Option<&str> {
    let kind = item.strip_suffix(EGG)?;
    Some(RENAMED.iter().find(|(egg, _)| *egg == kind).map_or(kind, |(_, hatched)| hatched))
}

/// Hatches the egg `item` into `cell` of `layer`; false when it is no egg or nothing hatched.
pub(crate) fn hatch(world: &mut World, layer: Entity, item: &str, [x, y, z]: BlockPos) -> bool {
    let Some(kind) = kind_of(item) else { return false };
    crate::creatures::spawn(world, layer, kind, [x as f32 + 0.5, y as f32, z as f32 + 0.5], SpawnReason::SpawnEgg).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eggs_name_their_kind() {
        assert_eq!(kind_of("minecraft:zombie_spawn_egg"), Some("minecraft:zombie"));
        assert_eq!(kind_of("minecraft:villager_spawn_egg"), Some("minecraft:villager_v2"));
        assert_eq!(kind_of("minecraft:evoker_spawn_egg"), Some("minecraft:evocation_illager"));
        assert_eq!(kind_of("minecraft:egg"), None);
    }
}
