//! What a player's effects do to its hits and its digging.

use bevy_ecs::prelude::*;
use sapling_api::Effect;

use super::level;

/// A level of strength multiplies a hit by the first and adds the second; of weakness likewise
/// (research/captures/bds-fx.txt: a fist's 1 was 3.99 at strength's second level, 0.3 weak).
const STRENGTHENS: [f32; 2] = [1.3, 0.3];
const WEAKENS: [f32; 2] = [0.8, 0.4];

/// What a hit of `amount` by a player costs under its strength and its weakness.
pub(crate) fn hits_for(world: &World, player: Entity, amount: f32) -> f32 {
    hit_under(amount, level(world, player, Effect::Strength), level(world, player, Effect::Weakness))
}

/// What a hit of `amount` costs under strength and weakness of these levels (from 0), a mob's as a
/// player's (bds-a3fx.txt, bds-a3fx3.txt: a zombie's 3 was 4.9, 7.37, 1.9 and 1.02 under strength I
/// and II and weakness I and II).
pub(crate) fn hit_under(amount: f32, strength: Option<u8>, weakness: Option<u8>) -> f32 {
    let under = |amount: f32, level: Option<u8>, [times, plus]: [f32; 2]| match level {
        Some(level) => {
            let by = times.powi(i32::from(level) + 1);
            amount * by + (by - 1.0) / plus
        }
        None => amount,
    };
    under(under(amount, strength, STRENGTHENS), weakness, WEAKENS).max(0.0)
}

/// What a player's haste and mining fatigue multiply its digging speed by. BDS's stone breaks
/// (research/captures/bds-dig.txt) fit Java's 1 + 0.2 a level of haste times another 1.2 a level,
/// and Java's 0.3 a level of fatigue times another 0.7: 1.44 and 2.76 faster, 0.21 and 0.044.
/// Conduit power digs as haste of its level (bds-dig.txt: the same 629 crack speed); the stronger counts.
pub(crate) fn dig_speed(world: &World, player: Entity) -> f32 {
    let levels = |effect| level(world, player, effect).map(|level| i32::from(level) + 1);
    let hastes = levels(Effect::Haste).max(levels(Effect::ConduitPower));
    let haste = hastes.map_or(1.0, |n| (1.0 + 0.2 * n as f32) * 1.2f32.powi(n));
    let fatigue = levels(Effect::MiningFatigue).map_or(1.0, |n| 0.21f32.powi(n));
    haste * fatigue
}
