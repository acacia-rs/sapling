//! What a new mob holds and wears from its pack's `minecraft:equipment` table, rolled as it comes:
//! BDS showed each armed kind's hand in `MobEquipment` right after its `AddEntity` at a summon
//! (research/captures/bds-y3gear.txt). The tables' armour pools and `enchant_random_gear` read the
//! world's difficulty and special multiplier (sapling-data mob_loot/regional.rs). An adult piglin
//! is a ranged or a melee unit at even odds (its pack's `spawn_adult`). Drowned (drowned.rs),
//! vexes (shown.rs) and riders (riders.rs) arm their own way. At death it lets go of what it holds
//! and wears as `Mob::dropEquipmentOnDeath` (1429f04f0) does: [`let_go`].

use bevy_ecs::prelude::*;
use sapling_api::{Difficulty, MobSlot};
use sapling_data::Regional;
use sapling_data::enchant::Enchant;
use sapling_world::ItemStack;

use crate::blocks::Dice;
use crate::components::{InLayer, Layer, Settings};
use crate::containers::loot::kept_stack;
use crate::inventory;

/// Kinds whose table is always theirs (a `components` table, or a group their spawn adds).
const TABLES: [(&str, &str); 9] = [
    ("minecraft:skeleton", "loot_tables/entities/skeleton_gear.json"),
    ("minecraft:stray", "loot_tables/entities/skeleton_gear.json"),
    ("minecraft:bogged", "loot_tables/entities/skeleton_gear.json"),
    ("minecraft:parched", "loot_tables/entities/skeleton_gear.json"),
    ("minecraft:wither_skeleton", "loot_tables/entities/wither_skeleton_gear.json"),
    ("minecraft:zombie_pigman", "loot_tables/entities/zombie_pigman_gear.json"),
    ("minecraft:piglin_brute", "loot_tables/entities/piglin_brute_gear.json"),
    ("minecraft:pillager", "loot_tables/entities/pillager_gear.json"),
    ("minecraft:vindicator", "loot_tables/entities/vindicator_gear.json"),
];

/// A zombie's table comes with the group its spawn's age events add (`can_have_equipment`), a husk's
/// with its components; a rider's is its rider group's instead (riders.rs). research/captures/
/// bds-f14cgear.txt, 40 of each on hard: 7 zombies and 6 husks in armour, 3 and 6 armed, 1 and 2 enchanted.
const ZOMBIE_GEAR: &str = "loot_tables/entities/zombie_equipment.json";
const ZOMBIE: &str = "minecraft:zombie";
const HUSK: &str = "minecraft:husk";
const ZOMBIE_ARMING_EVENTS: [&str; 5] = ["entity_spawned", "entity_born", "as_adult", "as_baby", "as_baby_jockey"];
const RIDES: &str = "spawn_as_rider";
/// Items a table gives that go in the off hand (bds-f14cgear.txt: a drowned's shell).
const OFF_HAND: [&str; 1] = ["minecraft:nautilus_shell"];

const PIGLIN: &str = "minecraft:piglin";
const RANGED: &str = "loot_tables/entities/piglin_gear_ranged.json";
const MELEE: &str = "loot_tables/entities/piglin_gear_melee.json";

/// A slot's drop chance where no kind's `slot_drop_chance` names it (the table at 14a73e498).
const DROPS: f32 = 0.25;
/// Each level of the killer's Looting adds this.
const PER_LOOTING: f32 = 0.01;
/// Kinds' own `slot_drop_chance` for their main hand.
const MAIN_HAND_DROPS: [(&str, f32); 4] = [("minecraft:villager_v2", 0.0), ("minecraft:copper_golem", 1.0), ("minecraft:fox", 1.0), ("minecraft:sulfur_cube", 1.0)];

/// Arms the new mob `mob` of `kind`, come by spawn `event`, from its table, if it has one.
pub(super) fn arm(world: &mut World, mob: Entity, kind: &str, event: Option<&str>) {
    if kind == PIGLIN {
        if !super::shown::is_baby(world, mob) {
            // An event the pack lacks (`minecraft:spawn_adult`) gives no unit: bds-goldpickup.txt's piglins held nothing.
            let table = match event.unwrap_or("minecraft:entity_spawned") {
                "spawn_adult_ranged" | "spawn_adult_ranged_no_hunting" => RANGED,
                "spawn_adult_melee" | "spawn_adult_melee_no_hunting" => MELEE,
                "minecraft:entity_spawned" | "spawn_adult" | "spawn_adult_no_hunting" if world.get_resource_or_insert_with(Dice::default).roll(2) == 0 => RANGED,
                "minecraft:entity_spawned" | "spawn_adult" | "spawn_adult_no_hunting" => MELEE,
                _ => return,
            };
            equip(world, mob, table);
        }
    } else if let Some((_, table)) = TABLES.iter().find(|(of, _)| *of == kind) {
        equip(world, mob, table);
    } else if kind == ZOMBIE || kind == HUSK {
        let event = event.unwrap_or("minecraft:entity_spawned").trim_start_matches("minecraft:");
        if if kind == ZOMBIE { ZOMBIE_ARMING_EVENTS.contains(&event) } else { event != RIDES } {
            equip(world, mob, ZOMBIE_GEAR);
        }
    }
}

/// The difficulty and special multiplier a mob in `layer` is born under.
fn regional(world: &World, layer: Entity) -> Regional {
    let difficulty = match world.resource::<Settings>().0.difficulty {
        Difficulty::Peaceful => 0,
        Difficulty::Easy => 1,
        Difficulty::Normal => 2,
        Difficulty::Hard => 3,
    };
    Regional::at(difficulty, world.get::<Layer>(layer).map_or(0, |layer| layer.world.level().time))
}

/// Gives `mob` what `table` rolls: armour worn, the rest held.
pub(super) fn equip(world: &mut World, mob: Entity, table: &str) {
    let Some(layer) = world.get::<InLayer>(mob).map(|layer| layer.0) else { return };
    let regional = regional(world, layer);
    let rolled = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        sapling_data::gear_loot(table, regional, &mut || dice.fraction())
    };
    let Some(registry) = world.get::<Layer>(layer).map(|layer| layer.world.registry()) else { return };
    let stacks: Vec<_> = rolled.into_iter().filter_map(|looted| kept_stack(looted, registry)).map(|stack| (inventory::worn_in(&stack.name), stack)).collect();
    for (piece, stack) in stacks {
        match piece {
            Some(piece) => drop(super::worn::wear(world, mob, piece, Some(stack))),
            None if super::piglins::hold(world, mob, &stack) => {}
            None => {
                let off = OFF_HAND.contains(&stack.name.as_str());
                super::command_gear::hold_from_birth(world, mob, stack, off);
            }
        }
    }
    super::shown::reshow(world, mob);
}

/// What the dead `mob` of `kind` lets go of from its hands and armour: a slot drops by its chance
/// (and the killer's Looting) only when a player slew it (`slain`, the Looting) or the chance is
/// whole; Curse of Vanishing never drops; a piece dropped by chance is worn by
/// `nextInt(nextInt(max(lasts, 4) - 3) + 1)` short of its full wear.
/// Also whether its main hand's item went.
pub(super) fn let_go(world: &mut World, mob: Entity, kind: &str, slain: Option<u8>) -> (Vec<ItemStack>, bool) {
    let gear = super::command_gear::gear_of(world, mob);
    let mut dice = world.get_resource_or_insert_with(Dice::default);
    let (mut dropped, mut main_went) = (Vec::new(), false);
    for (slot, item) in gear {
        let Some(mut item) = item.filter(|item| crate::enchants::level_of(item, Enchant::Vanishing) == 0) else {
            main_went |= slot == MobSlot::MainHand;
            continue;
        };
        let own = MAIN_HAND_DROPS.iter().find(|(of, _)| slot == MobSlot::MainHand && *of == kind).map(|(_, chance)| *chance);
        let mut chance = own.unwrap_or(DROPS);
        if chance == 0.0 {
            continue;
        }
        chance += f32::from(slain.unwrap_or(0)) * PER_LOOTING;
        if !(slain.is_some() || chance >= 1.0) || dice.fraction() >= chance {
            continue;
        }
        let lasts = sapling_data::gear(&item.name).map_or(0, |gear| i32::from(gear.lasts));
        if chance < 1.0 && lasts > 0 {
            let spread = dice.roll(lasts.max(4) as u32 - 3) + 1;
            inventory::set_damage(&mut item.tag, lasts - dice.roll(spread) as i32);
        }
        main_went |= slot == MobSlot::MainHand;
        dropped.push(item);
    }
    (dropped, main_went)
}
