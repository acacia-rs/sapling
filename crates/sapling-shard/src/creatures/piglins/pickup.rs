//! Piglins picking gold up off the ground, by the pack's `behavior.pickup_items` (`max_dist` 10,
//! `speed_multiplier` 0.8 of its 0.35) and `shareables` (`singular_pickup`), as
//! research/captures/bds-goldpickup.txt showed: an adult not admiring walks to gold lying within
//! 10 and takes it once within a block of its box (`TakeItemEntity`), shows it a tick later and
//! is heard to `admire` a tick after that. An ingot it admires in its offhand and barters; another
//! `admire` item (a golden sword in the drill) it admires as long and keeps; nuggets it keeps in
//! its main hand, which is empty here.
// TODO: what it keeps is lost on death and with the world; its other shareables (armour, food,
// crossbows) are not picked up; it is not shown admiring (`FlagsExtended` bit 30).

use acacia_proto::packets::TakeItemEntity;
use bevy_ecs::prelude::*;

use super::{ADMIRES, Admiring, GOLD, HEAD, Nuggets, PIGLIN, offhand};
use crate::body::Body;
use crate::components::{NetId, Tick};
use crate::drops;
use crate::late_sounds;
use crate::creatures::{Dying, hunting, is_baby, legs};

pub(super) const NUGGET: &str = "minecraft:gold_nugget";
const SIGHT: [f32; 2] = [10.0, 4.0];
const SPEED: f32 = 0.35 * 0.8;
/// Its box widened by this much across takes an item up (the Java game's `inflate(1, 0, 1)`).
const REACH: f32 = 1.0;
const HALF_WIDTH: f32 = 0.3;
const HEIGHT: f32 = 1.9;
/// The pack's `admire: true` shareables besides the bartered ingot.
const ADMIRED: &[&str] = &[
    "minecraft:golden_sword", "minecraft:golden_spear", "minecraft:golden_axe", "minecraft:golden_hoe", "minecraft:golden_pickaxe", "minecraft:golden_shovel",
    "minecraft:golden_helmet", "minecraft:golden_chestplate", "minecraft:golden_leggings", "minecraft:golden_boots", "minecraft:golden_apple", "minecraft:appleEnchanted",
    "minecraft:enchanted_golden_apple", "minecraft:golden_carrot", "minecraft:gold_block", "minecraft:raw_gold", "minecraft:gold_ore", "minecraft:nether_gold_ore",
    "minecraft:deepslate_gold_ore", "minecraft:raw_gold_block", "minecraft:gilded_blackstone", "minecraft:horsearmorgold", "minecraft:golden_horse_armor",
    "minecraft:golden_nautilus_armor", "minecraft:bell", "minecraft:clock", "minecraft:speckled_melon", "minecraft:glistering_melon_slice",
    "minecraft:light_weighted_pressure_plate",
];

fn wanted(name: &str) -> bool {
    name == GOLD || name == NUGGET || ADMIRED.contains(&name)
}

/// Before its move: a piglin free to do so goes for gold lying near or takes it up. False where none is near.
pub(in crate::creatures) fn steer(world: &mut World, piglin: Entity, layer: Entity, id: u64, kind: &str) -> bool {
    let busy = world.get::<Dying>(piglin).is_some() || world.get::<Admiring>(piglin).is_some() || is_baby(world, piglin);
    let Some(body) = world.get::<Body>(piglin).copied().filter(|_| kind == PIGLIN && !busy) else { return false };
    let [across, up] = SIGHT;
    let lying = drops::lying_in(world, layer, [body.at[0] - across, body.at[1] - up, body.at[2] - across], [body.at[0] + across, body.at[1] + up, body.at[2] + across]);
    let far = |at: [f32; 3]| (0..3).map(|axis| (at[axis] - body.at[axis]).powi(2)).sum::<f32>().sqrt();
    let nearest = lying
        .into_iter()
        .filter(|(_, stack)| wanted(&stack.name))
        .filter_map(|(item, stack)| world.get::<Body>(item).map(|lies| (item, stack, lies.at)))
        .min_by(|a, b| far(a.2).total_cmp(&far(b.2)));
    let Some((item, stack, at)) = nearest else { return false };
    let reach = HALF_WIDTH + REACH;
    let touches = (at[0] - body.at[0]).abs() <= reach && (at[2] - body.at[2]).abs() <= reach && (body.at[1]..=body.at[1] + HEIGHT).contains(&at[1]);
    if !touches {
        legs::walk(world, piglin, layer, &body, [at[0], at[2]], SPEED, 0.0);
        return true;
    }
    let Some(item_id) = world.get::<NetId>(item).map(|id| id.0) else { return false };
    let cell = body.at.map(|c| c.floor() as i32);
    let taken = sapling_net::encode(&TakeItemEntity { runtime_entity_id: item_id, target: id as u32 });
    crate::blocks::to_viewers(world, layer, at.map(|c| c.floor() as i32), &[taken]);
    let nuggets = stack.name == NUGGET;
    drops::take(world, item, if nuggets { stack.count } else { 1 });
    if nuggets {
        let kept = world.get::<Nuggets>(piglin).map_or(0, |nuggets| nuggets.0);
        world.entity_mut(piglin).insert(Nuggets(kept.saturating_add(stack.count)));
        if let Some(held) = super::held(world, piglin, id) {
            late_sounds::play_later(world, 1, layer, cell, held);
        }
        crate::creatures::reshow(world, piglin);
        return true;
    }
    let now = world.resource::<Tick>().0;
    world.entity_mut(piglin).insert(Admiring { until: now + ADMIRES, barters: stack.name == GOLD });
    late_sounds::play_later(world, 1, layer, cell, sapling_net::encode(&offhand(id, Some(&stack.name))));
    late_sounds::play_later(world, 2, layer, cell, hunting::sound("admire", id, PIGLIN, &body, HEAD));
    true
}
