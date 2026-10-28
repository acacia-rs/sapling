//! Piglins picking gold up off the ground, by the pack's `behavior.pickup_items` (`max_dist` 10,
//! `speed_multiplier` 0.8 of its 0.35) and `shareables` (`singular_pickup`), as
//! research/captures/bds-goldpickup.txt showed: an adult not admiring walks to gold lying within
//! 10 and takes it once within a block of its box (`TakeItemEntity`), shows it a tick later and
//! is heard to `admire` a tick after that. An ingot it admires in its offhand and barters; another
//! `admire` item (a golden sword in the drill) it admires as long and then holds; nuggets it keeps
//! in its main hand.
// TODO: what it holds is lost on death; its other shareables (armour, food, crossbows) are not
// picked up; it is not shown admiring (`FlagsExtended` bit 30).

use acacia_proto::packets::TakeItemEntity;
use bevy_ecs::prelude::*;

use super::{Admiring, GOLD, Hands, PIGLIN, admire, main_hand, stack};
use crate::body::Body;
use crate::components::NetId;
use crate::creatures::{Dying, is_baby, legs};
use crate::drops;
use crate::late_sounds;

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
    let Some((item, taken, at)) = nearest else { return false };
    let reach = HALF_WIDTH + REACH;
    let touches = (at[0] - body.at[0]).abs() <= reach && (at[2] - body.at[2]).abs() <= reach && (body.at[1]..=body.at[1] + HEIGHT).contains(&at[1]);
    if !touches {
        legs::walk(world, piglin, layer, &body, [at[0], at[2]], SPEED, 0.0);
        return true;
    }
    let Some(item_id) = world.get::<NetId>(item).map(|id| id.0) else { return false };
    let cell = body.at.map(|c| c.floor() as i32);
    let told = sapling_net::encode(&TakeItemEntity { runtime_entity_id: item_id, target: id as u32 });
    crate::blocks::to_viewers(world, layer, at.map(|c| c.floor() as i32), &[told]);
    let nuggets = taken.name == NUGGET;
    drops::take(world, item, if nuggets { taken.count } else { 1 });
    if nuggets {
        let kept = world.get::<Hands>(piglin).and_then(|hands| hands.main.as_ref()).filter(|held| held.name == NUGGET).map_or(0, |held| held.count);
        let held = stack(NUGGET, kept.saturating_add(taken.count));
        if let Some(shown) = main_hand(id, &held) {
            late_sounds::play_later(world, 1, layer, cell, shown);
        }
        let off = world.get_mut::<Hands>(piglin).and_then(|mut hands| hands.off.take());
        world.entity_mut(piglin).insert(Hands { main: Some(held), off });
        crate::creatures::reshow(world, piglin);
        return true;
    }
    admire(world, piglin, layer, sapling_world::ItemStack { count: 1, ..taken }, 1);
    true
}
