//! Monsters on peaceful: BDS's `Monster` constructor gives the actor `RemoveInPeacefulFlagComponent`
//! (bdsre 0x14298f180), and `Monster::normalTick` (0x14298f410) removes a flagged actor every tick the
//! level is peaceful, persistent or named alike, with no drops. Setting the difficulty to peaceful
//! takes them all at once (the `setDifficulty` lambda 0x140847630). docs/research/server-leftovers.md.

use bevy_ecs::prelude::*;
use sapling_api::Difficulty;

use crate::components::Settings;
use crate::entities::Kind;
use crate::mobs;

/// Monsters whose class takes the flag away again: the piglin's constructor (`HumanoidMonster`,
/// 0x142990d10) drops it and only the brute's (0x1429910f0) puts it back; the dragon's drops it. A shulker
/// stayed on peaceful too (research/captures/bds-server.txt).
const KEPT: [&str; 3] = ["minecraft:piglin", "minecraft:ender_dragon", "minecraft:shulker"];

/// Whether a mob of `kind` is taken away while the world is peaceful.
pub(crate) fn removed_in_peaceful(kind: &str) -> bool {
    sapling_data::has_family(kind, "monster") && !KEPT.contains(&kind)
}

/// Every tick on peaceful: takes the monsters away.
pub(crate) fn tick(world: &mut World) {
    if world.resource::<Settings>().0.difficulty != Difficulty::Peaceful {
        return;
    }
    let gone: Vec<Entity> = world.query::<(Entity, &Kind)>().iter(world).filter(|(_, kind)| removed_in_peaceful(&kind.0)).map(|(entity, _)| entity).collect();
    for entity in gone {
        mobs::remove(world, entity);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_kinds_bds_took_and_kept_on_peaceful() {
        let kinds = ["stray", "husk", "drowned", "bogged", "wither_skeleton", "shulker", "sulfur_cube", "camel_husk", "polar_bear", "ender_dragon"];
        let removed: Vec<bool> = kinds.iter().map(|kind| super::removed_in_peaceful(&format!("minecraft:{kind}"))).collect();
        assert_eq!(removed, [true, true, true, true, true, false, false, false, false, false], "bds-server.txt");
    }
}
