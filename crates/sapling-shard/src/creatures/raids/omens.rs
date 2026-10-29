//! The omens that start a raid, as the player's pack has them: bad omen in a village turns to raid
//! omen for 30 s (`minecraft:gain_raid_omen`), where it was taken kept
//! (`RaidOmenPosition`); as the raid omen runs out, a raid starts there (`minecraft:trigger_raid`).

use bevy_ecs::prelude::*;
use sapling_api::{Difficulty, Effect};

use super::village;
use crate::components::{InLayer, Position, Settings, in_world};
use crate::vitals::{self, effects};

const RAID_OMEN_LASTS: u32 = 600;

/// Where a player took its raid omen.
#[derive(Component)]
struct OmenAt([f32; 3]);

/// A raid omen run out: its world, where it took the omen, and the omen's level from 1.
pub(super) struct Trigger {
    pub layer: Entity,
    pub at: [f32; 3],
    pub level: u8,
}

/// Turns bad omens in villages to raid omens; the raid omens that run out this tick.
pub(super) fn tick(world: &mut World) -> Vec<Trigger> {
    if world.resource::<Settings>().0.difficulty == Difficulty::Peaceful {
        return Vec::new();
    }
    let mut triggers = Vec::new();
    for player in in_world(world) {
        let (Some(layer), Some(feet)) = (world.get::<InLayer>(player).map(|layer| layer.0), world.get::<Position>(player).map(|position| position.feet)) else { continue };
        if !vitals::is_mortal(world, player) {
            continue;
        }
        if let Some((amplifier, left)) = effects::on(world, player, Effect::RaidOmen) {
            if left == 1 {
                let at = world.get::<OmenAt>(player).map_or(feet, |at| at.0);
                triggers.push(Trigger { layer, at, level: amplifier.saturating_add(1) });
            }
            continue;
        }
        if effects::on(world, player, Effect::BadOmen).is_none() {
            continue;
        }
        if village::centre(world, layer, feet).is_some() {
            effects::end(world, player, Effect::BadOmen);
            // Amplifier 0 whatever the bad omen's, so a level 1 raid (bds-raid.txt, bds-raidhero.txt).
            vitals::give_effect(world, player, Effect::RaidOmen, RAID_OMEN_LASTS, 0, true);
            world.entity_mut(player).insert(OmenAt(feet));
        }
    }
    triggers
}
