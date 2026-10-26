//! The experience smelting gives: each result's share (BDS `getFurnaceXPmultiplier`), paid as
//! orbs at the player who takes the results, and at the furnace when it is broken. What hoppers
//! take out is owed to the next player (`StoredXPInt`, in hundredths). docs/research/furnaces.md
//! section 2, experience.md section 1.

use acacia_proto::nbt::Value;
use bevy_ecs::prelude::*;
use sapling_api::BlockPos;
use sapling_world::ItemStack;

use crate::blocks::Dice;
use crate::orbs;

/// The key of what is owed, in hundredths, and the old one in whole points.
pub(super) const STORED: &str = "StoredXPInt";
const STORED_OLD: &str = "StoredXP";

const BY_NAME: &[(&str, f32)] = &[
    ("netherite_scrap", 2.0),
    ("diamond", 1.0),
    ("emerald", 1.0),
    ("gold_ingot", 1.0),
    ("iron_ingot", 0.7),
    ("copper_ingot", 0.7),
    ("redstone", 0.7),
    ("hardened_clay", 0.35),
    ("baked_potato", 0.35),
    ("brick", 0.3),
    ("ink_sac", 0.2),
    ("bone_meal", 0.2),
    ("cocoa_beans", 0.2),
    ("lapis_lazuli", 0.2),
    ("quartz", 0.2),
    ("sponge", 0.15),
    ("wet_sponge", 0.15),
    ("dried_kelp", 0.1),
    ("gold_nugget", 0.1),
    ("iron_nugget", 0.1),
    ("copper_nugget", 0.1),
    ("netherbrick", 0.1),
    ("popped_chorus_fruit", 0.1),
    ("resin_brick", 0.1),
    ("glass", 0.1),
    ("stone", 0.1),
    ("smooth_stone", 0.1),
    ("deepslate", 0.1),
    ("smooth_basalt", 0.1),
    ("cracked_deepslate_tiles", 0.1),
    ("cracked_deepslate_bricks", 0.1),
    ("quartz_block", 0.1),
    ("smooth_quartz", 0.1),
    ("leaf_litter", 0.1),
];
/// Cooked food: `cooked_` and the name.
const COOKED: f32 = 0.35;
const DYE: f32 = 0.2;
/// Stone bricks and sandstones of every kind.
const BUILDING: (&[&str], f32) = (&["stone_bricks", "sandstone", "quartz_pillar", "chiseled_quartz_block"], 0.1);

/// The experience one `name` out of a furnace is worth.
pub(super) fn per_item(namespaced: &str) -> f32 {
    let name = namespaced.strip_prefix("minecraft:").unwrap_or(namespaced);
    if let Some((_, xp)) = BY_NAME.iter().find(|(item, _)| *item == name) {
        return *xp;
    }
    match name {
        _ if name.starts_with("cooked_") => COOKED,
        _ if name.ends_with("_dye") => DYE,
        _ if BUILDING.0.iter().any(|end| name.ends_with(end)) => BUILDING.1,
        _ => 0.0,
    }
}

/// `points` rounded down, and up by the chance of its fraction (BDS's random rounding).
fn round_random(world: &mut World, points: f32) -> i32 {
    let whole = points.floor();
    let chance = ((points - whole) * 1_000_000.0) as u32;
    let up = chance > 0 && world.get_resource_or_insert_with(Dice::default).roll(1_000_000) < chance;
    whole as i32 + i32::from(up)
}

/// What a furnace's block entity says is owed, in hundredths.
pub(super) fn stored(entity: &Value) -> i32 {
    match (entity.get(STORED), entity.get(STORED_OLD)) {
        (Some(Value::Int(hundredths)), _) => (*hundredths).max(0),
        (None, Some(Value::Short(points))) => i32::from(*points).max(0) * 100,
        _ => 0,
    }
}

/// The experience for `count` of `item`, and what was owed (hundredths), as whole points.
pub(super) fn reward(world: &mut World, item: &ItemStack, count: u8, owed: i32) -> i32 {
    round_random(world, f32::from(count) * per_item(&item.name)) + round_random(world, owed as f32 / 100.0)
}

/// What a hopper taking `item` out leaves owed, in hundredths.
pub(super) fn owed_for(item: &ItemStack) -> i32 {
    (f32::from(item.count) * per_item(&item.name) * 100.0) as i32
}

/// Pays `points` as orbs at the corner of a broken furnace.
pub(super) fn pay_at(world: &mut World, layer: Entity, pos: BlockPos, points: i32) {
    orbs::spawn(world, layer, pos.map(|c| c as f32), points);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_are_worth_what_bds_gives() {
        let xp = |name: &str| per_item(&format!("minecraft:{name}"));
        assert_eq!([xp("iron_ingot"), xp("gold_ingot"), xp("cooked_beef"), xp("baked_potato"), xp("glass"), xp("red_dye")], [0.7, 1.0, 0.35, 0.35, 0.1, 0.2]);
        assert_eq!([xp("smooth_sandstone"), xp("cracked_stone_bricks"), xp("charcoal"), xp("iron_nugget")], [0.1, 0.1, 0.0, 0.1]);
    }
}
