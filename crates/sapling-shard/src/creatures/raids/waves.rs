//! A raid's waves: BDS's raid groups (`definitions/spawn_groups/raid_group_<n>.json`, one a wave)
//! and how many a raid has (`Village::_createRaid` at 0x142a29ac0: 3, 5 or 7 by difficulty),
//! spawned together at a spot 32 to 48 out from the village's farthest player ([`spot`]).

use bevy_ecs::prelude::*;
use sapling_api::{Difficulty, SpawnReason};

use crate::blocks::Dice;
use crate::components::{InLayer, Layer, Position, in_world};
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
/// A wave comes 32 to 48 out from its player; a spot is looked for this many times.
const OUT: f32 = 32.0;
const OUT_MORE: f32 = 16.0;
const TRIES: usize = 10;
/// Players this near the middle (across) stand for BDS's village bounds grown by 16.
const BOUNDS: f32 = 32.0;

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

/// The player in the village farthest from its middle (across), BDS's anchor for a wave.
fn farthest_player(world: &World, layer: Entity, middle: [f32; 3]) -> Option<[f32; 3]> {
    let across = |feet: [f32; 3]| (feet[0] - middle[0]).hypot(feet[2] - middle[2]);
    let in_layer = |player: &Entity| world.get::<InLayer>(*player).is_some_and(|on| on.0 == layer);
    let feet = in_world(world).into_iter().filter(in_layer).filter_map(|player| world.get::<Position>(player).map(|position| position.feet));
    feet.filter(|&feet| across(feet) <= BOUNDS).max_by(|a, b| across(*a).total_cmp(&across(*b)))
}

/// A spot on the ground 32 to 48 out from the village's farthest player (else its middle), on
/// the side away from the middle, in a loaded column (`Village::_createRaid` lambda_9 case 1,
/// 0x142a50c10: the farthest actor in the bounds, then a draw folded to the far side).
fn spot(world: &mut World, layer: Entity, middle: [f32; 3]) -> Option<[f32; 3]> {
    let base = farthest_player(world, layer, middle).unwrap_or(middle);
    let away = [base[0] - middle[0], base[2] - middle[2]];
    let length = away[0].hypot(away[1]);
    let away = if length >= 1e-4 { away.map(|axis| axis / length) } else { [0.0, 0.0] };
    for _ in 0..TRIES {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        let (out, angle) = (OUT + dice.fraction() * OUT_MORE, dice.fraction() * std::f32::consts::TAU);
        let mut way = [angle.cos() * out, angle.sin() * out];
        let along = way[0] * away[0] + way[1] * away[1];
        if along < 0.0 {
            way = [way[0] - 2.0 * along * away[0], way[1] - 2.0 * along * away[1]];
        }
        let [x, z] = [base[0] + way[0], base[2] + way[1]].map(f32::floor);
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
