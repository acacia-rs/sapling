//! What a block a survival player broke leaves besides its items: an ore its experience, ice
//! its water (research/captures/bds-loot.txt).
// TODO: how much each ore leaves is the Java game's range, which what 20 ores left on BDS was
// within. A spawner and sculk leave none yet.

use bevy_ecs::prelude::*;
use sapling_api::BlockPos;

use super::liquid::{self, Liquid};
use super::loot::{Breaker, Dice};
use super::level;
use crate::orbs;

/// The least and most experience an ore leaves, by its name without `deepslate_` and `lit_`.
const ORES: [(&str, [u32; 2]); 7] = [("coal_ore", [0, 2]), ("diamond_ore", [3, 7]), ("emerald_ore", [3, 7]), ("lapis_ore", [2, 5]), ("quartz_ore", [2, 5]), ("redstone_ore", [1, 5]), ("nether_gold_ore", [0, 1])];
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
    // Ice over something that holds water, or over a liquid, melts where it was.
    let holds = level(world, layer).is_some_and(|level| level.registry().get(level.block(pos[0], pos[1] - 1, pos[2])).is_some_and(|below| below.is_solid() || below.is_liquid()));
    if name == ICE && holds {
        liquid::pour(world, layer, pos, Liquid::Water, false);
    }
}
