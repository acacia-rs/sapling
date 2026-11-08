//! What a block a survival player broke leaves besides its items: an ore its experience, ice
//! its water (research/captures/bds-loot.txt). Ore ranges: docs/research/experience.md, "Ores".
//! A spawner and sculk leave theirs however they break, `/fill ... destroy` too (bds-i1misc.txt).

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::liquid::{self, Liquid};
use super::loot::{Breaker, Dice};
use super::level;
use crate::orbs;

/// The least and most experience an ore leaves, by its name without `deepslate_` and `lit_`.
const ORES: [(&str, [u32; 2]); 7] = [("coal_ore", [0, 2]), ("diamond_ore", [3, 7]), ("emerald_ore", [3, 7]), ("lapis_ore", [2, 5]), ("quartz_ore", [2, 5]), ("redstone_ore", [1, 5]), ("nether_gold_ore", [0, 1])];
/// Fixed experience of the sculk family.
const SCULK: [(&str, u32); 5] = [("sculk", 1), ("sculk_sensor", 5), ("calibrated_sculk_sensor", 5), ("sculk_catalyst", 5), ("sculk_shrieker", 5)];
const SPAWNER: &str = "mob_spawner";
const ICE: &str = "minecraft:ice";

/// The block `name` at `pos` is gone, broken by `by`.
pub(super) fn leave(world: &mut World, layer: Entity, pos: BlockPos, name: &str, by: &Breaker) {
    super::infested::release(world, layer, pos, name, by.silk_touch);
    if !by.harvests || by.silk_touch {
        return;
    }
    let plain = name.trim_start_matches("minecraft:").trim_start_matches("lit_").trim_start_matches("deepslate_");
    if let Some((_, [least, most])) = ORES.iter().find(|(ore, _)| *ore == plain) {
        let points = least + world.get_resource_or_insert_with(Dice::default).roll(most - least + 1);
        orbs::spawn(world, layer, pos.map(|c| c as f32 + 0.5), points as i32);
    }
    leave_experience(world, layer, pos, name);
    // Ice over something that holds water, or over a liquid, melts where it was.
    let holds = level(world, layer).is_some_and(|level| level.registry().get(level.block(pos[0], pos[1] - 1, pos[2])).is_some_and(|below| below.is_solid() || below.is_liquid()));
    if name == ICE && holds {
        liquid::pour(world, layer, pos, Liquid::Water, false);
    }
}

/// Drops the experience a broken spawner (15 and two rolls of 0..15, as Java's 16..42 seen) or
/// sculk block leaves at `pos`; nothing for any other `name`.
pub(super) fn leave_experience(world: &mut World, layer: Entity, pos: BlockPos, name: &str) {
    let plain = name.trim_start_matches("minecraft:");
    let points = match SCULK.iter().find(|(sculk, _)| *sculk == plain) {
        Some((_, points)) => *points,
        None if plain == SPAWNER => {
            let mut dice = world.get_resource_or_insert_with(Dice::default);
            15 + dice.roll(15) + dice.roll(15)
        }
        None => return,
    };
    orbs::spawn(world, layer, pos.map(|c| c as f32 + 0.5), points as i32);
}
