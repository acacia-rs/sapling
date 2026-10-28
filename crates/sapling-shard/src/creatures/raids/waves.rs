//! A raid's waves: BDS's raid groups (`definitions/spawn_groups/raid_group_<n>.json`, one a wave)
//! and how many a raid has (`Village::_createRaid` at 0x142a29ac0: 3, 5 or 7 by difficulty),
//! spawned together at a spot 32 out from the village's middle (the Java game's
//! `Raid.findRandomSpawnPos`).

use bevy_ecs::prelude::*;
use sapling_api::{Difficulty, SpawnReason};

use crate::blocks::Dice;
use crate::components::Layer;
use crate::entities;

const PILLAGER: &str = "minecraft:pillager";
const VINDICATOR: &str = "minecraft:vindicator";
const EVOKER: &str = "minecraft:evocation_illager";
const WITCH: &str = "minecraft:witch";
const RAVAGER: &str = "minecraft:ravager";
const FOR_RAID: &str = "minecraft:spawn_for_raid";
const WITH_PILLAGER: &str = "minecraft:spawn_for_raid_with_pillager_rider";
const WITH_EVOKER: &str = "minecraft:spawn_for_raid_with_evoker_rider";

/// Each group's kinds, how many of each, and the event each spawns with.
const GROUPS: [&[(&str, u8, &str)]; 7] = [
    &[(PILLAGER, 4, FOR_RAID)],
    &[(PILLAGER, 3, FOR_RAID), (VINDICATOR, 2, FOR_RAID)],
    &[(PILLAGER, 3, FOR_RAID), (RAVAGER, 1, FOR_RAID)],
    &[(WITCH, 3, FOR_RAID), (PILLAGER, 3, FOR_RAID)],
    &[(VINDICATOR, 4, FOR_RAID), (EVOKER, 1, FOR_RAID), (RAVAGER, 1, FOR_RAID), (RAVAGER, 1, WITH_PILLAGER)],
    &[(PILLAGER, 5, FOR_RAID), (VINDICATOR, 2, FOR_RAID), (EVOKER, 1, FOR_RAID)],
    &[(VINDICATOR, 6, FOR_RAID), (WITCH, 1, FOR_RAID), (EVOKER, 2, FOR_RAID), (RAVAGER, 1, WITH_PILLAGER), (RAVAGER, 1, WITH_EVOKER)],
];
/// A wave comes this far out from the middle; a spot is looked for this many times.
const OUT: f32 = 32.0;
const TRIES: usize = 20;

/// How many waves a raid on `difficulty` has: `0x07050300 >> 8 * difficulty`.
pub(super) fn count(difficulty: Difficulty) -> u8 {
    match difficulty {
        Difficulty::Peaceful => 0,
        Difficulty::Easy => 3,
        Difficulty::Normal => 5,
        Difficulty::Hard => 7,
    }
}

/// The kinds wave `wave` (from 0) brings, riders included, as (kind, how many).
#[cfg(test)]
fn kinds(wave: u8) -> Vec<(&'static str, u8)> {
    let mut kinds = Vec::new();
    for &(kind, many, event) in GROUPS[usize::from(wave).min(GROUPS.len() - 1)] {
        kinds.push((kind, many));
        match event {
            WITH_PILLAGER => kinds.push((PILLAGER, many)),
            WITH_EVOKER => kinds.push((EVOKER, many)),
            _ => {}
        }
    }
    kinds
}

/// A spot on the ground 32 out from `middle` at a random way, in a loaded column.
fn spot(world: &mut World, layer: Entity, middle: [f32; 3]) -> Option<[f32; 3]> {
    for _ in 0..TRIES {
        let angle = world.resource_mut::<Dice>().fraction() * std::f32::consts::TAU;
        let [x, z] = [middle[0] + angle.cos() * OUT, middle[2] + angle.sin() * OUT].map(f32::floor);
        let top = world.get::<Layer>(layer)?.world.surface(x as i32, z as i32);
        if let Some(top) = top {
            return Some([x + 0.5, (top + 1) as f32, z + 0.5]);
        }
    }
    None
}

/// Spawns wave `wave` (from 0) of a raid on the village about `middle`: its raiders.
pub(super) fn spawn(world: &mut World, layer: Entity, middle: [f32; 3], wave: u8) -> Vec<Entity> {
    let Some(feet) = spot(world, layer, middle) else { return Vec::new() };
    let mut raiders = Vec::new();
    let mut born = |world: &mut World, kind: &str, event: &str| {
        let raider = super::super::spawn_as(world, layer, kind, feet, SpawnReason::Raid, 0.0, Some(event)).and_then(|id| entities::find(world, id));
        raiders.extend(raider);
        raider
    };
    for &(kind, many, event) in GROUPS[usize::from(wave).min(GROUPS.len() - 1)] {
        for _ in 0..many {
            let Some(mount) = born(world, kind, event) else { continue };
            let rider = match event {
                WITH_PILLAGER => born(world, PILLAGER, FOR_RAID),
                WITH_EVOKER => born(world, EVOKER, FOR_RAID),
                _ => None,
            };
            if let Some(rider) = rider {
                let _ = crate::riding::seat(world, rider, mount);
            }
        }
    }
    raiders
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_last_wave_on_hard_brings_eleven_and_two_riders() {
        let all: u8 = kinds(6).iter().map(|(_, many)| many).sum();
        assert_eq!((count(Difficulty::Hard), all), (7, 13));
        assert_eq!(kinds(0), [(PILLAGER, 4)]);
    }
}
