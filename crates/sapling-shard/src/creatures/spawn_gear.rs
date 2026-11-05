//! What a new mob holds and wears from its pack's `minecraft:equipment` table, rolled as it comes:
//! BDS showed each armed kind's hand in `MobEquipment` right after its `AddEntity` at a summon
//! (research/captures/bds-y3gear.txt). Drowned (drowned.rs), piglins (piglins.rs), vexes
//! (shown.rs) and riders (riders.rs) arm their own way.
// TODO: tables' `enchant_random_gear` and difficulty-conditioned armour pools are not rolled, and a
// dead mob does not drop what it holds (`slot_drop_chance`).

use bevy_ecs::prelude::*;
use sapling_api::Item;

use crate::blocks::Dice;
use crate::components::{InLayer, Layer};
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

/// Arms the new mob `mob` of `kind` from its table, if it has one.
pub(super) fn arm(world: &mut World, mob: Entity, kind: &str) {
    if let Some((_, table)) = TABLES.iter().find(|(of, _)| *of == kind) {
        equip(world, mob, table);
    }
}

/// Gives `mob` what `table` rolls: armour worn, the rest held.
pub(super) fn equip(world: &mut World, mob: Entity, table: &str) {
    let rolled = {
        let mut dice = world.get_resource_or_insert_with(Dice::default);
        sapling_data::table_loot(table, &mut || dice.fraction())
    };
    let Some(layer) = world.get::<InLayer>(mob).map(|layer| layer.0) else { return };
    let Some(registry) = world.get::<Layer>(layer).map(|layer| layer.world.registry()) else { return };
    let stacks: Vec<_> = rolled.into_iter().filter_map(|(name, count)| inventory::stored(&Item::new(name, count.min(64) as u8), registry).map(|stack| (inventory::worn_in(&stack.name), stack))).collect();
    for (piece, stack) in stacks {
        match piece {
            Some(piece) => drop(super::worn::wear(world, mob, piece, Some(stack))),
            None => super::command_gear::hold_from_birth(world, mob, stack),
        }
    }
    super::shown::reshow(world, mob);
}
