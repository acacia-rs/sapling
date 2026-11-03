//! Drills of the still-open wave's mounts and plants (research/parity/OPEN-BRIEF.md "Group 1"): a
//! saddled mount's flags and a summon's spawn sound, a chested mount's slots by command and what its
//! folder keeps, and kelp and vines that lose their hold or meet water. On creature_drill.rs's floor.

use super::creatures::{floor, waits, with};

/// Packets shown of the plant drill: blocks named (UpdateSubchunkBlocks turns naming on).
const PLANTS_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "LevelEvent", "AddItemEntity"];
/// Packets shown of the mount drills.
const MOUNTS_SHOWN: &[&str] = &["AddEntity", "SetEntityData", "LevelSoundEvent", "MobEquipment", "MobArmorEquipment", "UpdateEquipment", "InventoryContent", "InventorySlot"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "mountflags" => (mount_flags(), MOUNTS_SHOWN),
        "mountslots" => (mount_slots(), MOUNTS_SHOWN),
        "plantpop" => (plant_pop(), PLANTS_SHOWN),
        "plantpop2" => (plant_pop2(), PLANTS_SHOWN),
        "mountdeath" => (mount_death(), &["AddItemEntity", "AddEntity", "SetEntityData"]),
        _ => return None,
    })
}

/// Each mount killed alone with its tack on, its drops then cleared: the saddled horse family and a
/// camel, a horse summoned saddled, a horse in armour, a chested saddled donkey, a carpeted llama;
/// last a saddled horse with `doMobLoot` off.
fn mount_death() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["gamerule domobloot true", ""].map(String::from));
    let killed = |lines: &mut Vec<String>, kind: &str| lines.extend([format!("kill @e[type={kind}]"), String::new(), String::new(), "kill @e[type=item]".into(), String::new()]);
    for (kind, event) in [("horse", "spawn_tame_adult"), ("donkey", "spawn_tame_adult"), ("mule", "spawn_tame_adult"), ("zombie_horse", "spawn_tame_adult"), ("skeleton_horse", "spawn_adult"), ("camel", "spawn_adult")] {
        lines.extend([format!("summon {kind} 2.5 -26 0.5 0 0 minecraft:{event}"), format!("effect @e[type={kind}] slowness 200 255 true"), String::new()]);
        lines.extend([format!("replaceitem entity @e[type={kind}] slot.saddle 0 saddle"), String::new()]);
        killed(&mut lines, kind);
    }
    lines.extend(["summon horse 2.5 -26 0.5 0 0 minecraft:horse_saddled".to_owned(), String::new()]);
    killed(&mut lines, "horse");
    lines.extend(["summon horse 2.5 -26 0.5 0 0 minecraft:spawn_tame_adult".to_owned(), "replaceitem entity @e[type=horse] slot.armor.body 0 iron_horse_armor".into(), String::new()]);
    killed(&mut lines, "horse");
    chested(&mut lines, "donkey", 0);
    lines.extend(["replaceitem entity @e[type=donkey] slot.saddle 0 saddle".to_owned(), String::new()]);
    killed(&mut lines, "donkey");
    lines.extend(["summon llama 2.5 -26 0.5 0 0 minecraft:spawn_tame_adult".to_owned(), "replaceitem entity @e[type=llama] slot.armor.body 0 white_carpet".into(), String::new()]);
    killed(&mut lines, "llama");
    lines.extend(["gamerule domobloot false", "summon horse 2.5 -26 0.5 0 0 minecraft:spawn_tame_adult", "replaceitem entity @e[type=horse] slot.saddle 0 saddle", ""].map(String::from));
    killed(&mut lines, "horse");
    lines
}

/// Tamed horses, donkeys, mules and zombie horses saddled and unsaddled by command; then a plain
/// horse and skeleton horse, a camel saddled, and a horse summoned saddled.
fn mount_flags() -> Vec<String> {
    let mut lines = floor();
    for (kind, z) in [("horse", -6), ("donkey", -3), ("mule", 0), ("zombie_horse", 3)] {
        lines.extend([format!("summon {kind} 4.5 -26 {z}.5 0 0 minecraft:spawn_tame_adult"), format!("effect @e[type={kind}] slowness 200 255 true"), String::new()]);
        lines.extend([format!("replaceitem entity @e[type={kind}] slot.saddle 0 saddle"), String::new(), format!("replaceitem entity @e[type={kind}] slot.saddle 0 air"), String::new()]);
    }
    lines.extend(["kill @e[type=!player]", "", "summon horse 4.5 -26 -6.5", "", "summon skeleton_horse 4.5 -26 -3.5", ""].map(String::from));
    lines.extend(["summon camel 4.5 -26 3.5 0 0 minecraft:spawn_adult", "effect @e[type=camel] slowness 200 255 true", "", "replaceitem entity @e[type=camel] slot.saddle 0 saddle", ""].map(String::from));
    lines.extend(["summon horse 8.5 -26 0.5 0 0 minecraft:horse_saddled", "", "kill @e[type=!player]", ""].map(String::from));
    lines
}

/// A tamed `kind` held still at `z`, given a chest by the actor's click from beside it.
fn chested(lines: &mut Vec<String>, kind: &str, z: i32) {
    lines.extend([format!("summon {kind} 2.5 -26 {z}.5 0 0 minecraft:spawn_tame_adult"), format!("effect @e[type={kind}] slowness 200 255 true"), String::new()]);
    lines.extend(["clear @s".to_owned(), "give @s chest".into(), format!("tp @s 0.5 -26 {z}.5 -90 0"), String::new(), format!("!interact {kind}"), String::new()]);
}

/// The actor opens `kind`'s screen by a sneaking click with an empty hand.
fn opened(lines: &mut Vec<String>, kind: &str, z: i32) {
    lines.extend(["clear @s".to_owned(), format!("tp @s 0.5 -26 {z}.5 -90 0"), String::new(), "!sneak".into(), format!("!interact {kind}"), String::new(), "!unsneak".into(), String::new()]);
}

/// A chested donkey and llama given goods by `slot.chest` and `slot.inventory` at the ends of their
/// chests and past them, then opened; a zombie given hand items; all left for the folder.
fn mount_slots() -> Vec<String> {
    let mut lines = floor();
    chested(&mut lines, "donkey", 0);
    for (slot, item) in [("slot.chest 0", "diamond 1"), ("slot.chest 14", "emerald 2"), ("slot.chest 15", "gold_ingot 1"), ("slot.inventory 1", "iron_ingot 3"), ("slot.inventory 15", "coal 4"), ("slot.inventory 16", "coal 5"), ("slot.inventory 0", "saddle")] {
        lines.push(format!("replaceitem entity @e[type=donkey] {slot} {item}"));
    }
    lines.push(String::new());
    opened(&mut lines, "donkey", 0);
    chested(&mut lines, "llama", 4);
    for (slot, item) in [("slot.chest 0", "diamond 1"), ("slot.chest 2", "emerald 2"), ("slot.chest 3", "gold_ingot 1"), ("slot.chest 14", "iron_ingot 1"), ("slot.chest 15", "coal 1"), ("slot.inventory 1", "stone 2"), ("slot.inventory 4", "dirt 3"), ("slot.inventory 0", "white_carpet")] {
        lines.push(format!("replaceitem entity @e[type=llama] {slot} {item}"));
    }
    lines.push(String::new());
    opened(&mut lines, "llama", 4);
    lines.extend(["summon zombie 8.5 -26 0.5", "effect @e[type=zombie] slowness 200 255 true", "replaceitem entity @e[type=zombie] slot.weapon.mainhand 0 iron_sword", "replaceitem entity @e[type=zombie] slot.weapon.offhand 0 shield", ""].map(String::from));
    with(lines, &["clear @s", "tp @s 0.5 -26 0.5", "", ""])
}

/// Kelp set in glass basins (z -6; under glass or water, at the surface, dry; on glass or stone),
/// each then given a neighbour's setblock; twisting, weeping and cave vines (z 4) beside a set water
/// source, set into one, or losing the block they stand or hang on.
fn plant_pop() -> Vec<String> {
    let mut lines = floor();
    for x in [-6, -3, 0, 3, 6, 9] {
        lines.push(format!("fill {} -26 -7 {} -23 -5 glass", x - 1, x + 1));
    }
    lines.extend(["setblock -3 -26 -6 stone", "setblock 3 -26 -6 stone", "setblock 6 -26 -6 stone", "setblock 9 -26 -6 stone"].map(String::from));
    lines.extend(["fill 5 -24 -7 7 -23 -5 air", "fill 8 -24 -7 10 -23 -5 air", "setblock 9 -25 -6 air"].map(String::from));
    lines.extend(["setblock -6 -25 -6 water", "setblock -3 -25 -6 water", "setblock 0 -25 -6 water", "setblock 0 -24 -6 water", "setblock 3 -25 -6 water", "setblock 3 -24 -6 water", "setblock 6 -25 -6 water"].map(String::from));
    waits(&mut lines, 2);
    for x in [-6, -3, 0, 3, 6, 9] {
        lines.extend([format!("setblock {x} -25 -6 kelp"), String::new(), format!("setblock {} -25 -6 stone", x + 1), String::new(), String::new()]);
    }
    for x in [-6, -2, 2, 6, 10, 14] {
        lines.push(format!("fill {} -26 3 {} -24 5 glass", x - 1, x + 2));
    }
    lines.extend(["setblock -6 -26 4 air", "fill -2 -26 4 -1 -26 4 air", "fill 2 -25 4 3 -25 4 air", "setblock 2 -24 4 stone", "fill 6 -25 4 7 -25 4 air", "setblock 6 -24 4 stone"].map(String::from));
    lines.extend(["setblock 10 -25 4 air", "setblock 10 -24 4 stone", "setblock 14 -26 4 air", ""].map(String::from));
    lines.extend(["setblock -6 -26 4 twisting_vines", "", "setblock -5 -26 4 stone", "", "setblock -6 -27 4 air", "", ""].map(String::from));
    lines.extend(["setblock -2 -26 4 twisting_vines", "", "setblock -1 -26 4 water", "", ""].map(String::from));
    lines.extend(["setblock 2 -25 4 weeping_vines", "", "setblock 3 -25 4 water", "", ""].map(String::from));
    lines.extend(["setblock 6 -25 4 cave_vines", "", "setblock 7 -25 4 water", "", ""].map(String::from));
    lines.extend(["setblock 10 -25 4 weeping_vines", "", "setblock 11 -25 4 stone", "", "setblock 10 -24 4 air", "", ""].map(String::from));
    lines.extend(["setblock 14 -26 4 water", "", "setblock 14 -26 4 twisting_vines", "", "setblock 15 -26 4 stone", "", ""].map(String::from));
    lines
}

/// Twisting, weeping and cave vines and kelp on stone (z 4) with a water source beside them set
/// running by a setblock past it; a kelp on glass and one on stone set into air (z -6).
fn plant_pop2() -> Vec<String> {
    let mut lines = floor();
    for x in [-6, -2, 2, 6] {
        lines.push(format!("fill {} -26 3 {} -24 5 glass", x - 1, x + 2));
    }
    lines.extend(["fill -6 -26 4 -5 -26 4 air", "fill -2 -25 4 -1 -25 4 air", "setblock -2 -24 4 stone", "fill 2 -25 4 3 -25 4 air", "setblock 2 -24 4 stone", "fill 6 -26 4 7 -26 4 air", "setblock 6 -27 4 stone", ""].map(String::from));
    for (x, y, plant) in [(-6, -26, "twisting_vines"), (-2, -25, "weeping_vines"), (2, -25, "cave_vines"), (6, -26, "kelp")] {
        lines.extend([format!("setblock {x} {y} 4 {plant}"), String::new(), format!("setblock {} {y} 4 water", x + 1), String::new(), format!("setblock {} {y} 4 stone", x + 2), String::new(), String::new(), String::new()]);
    }
    lines.extend(["fill -7 -26 -7 -5 -24 -5 glass", "setblock -6 -25 -6 air", "fill -1 -26 -7 1 -24 -5 glass", "setblock 0 -26 -6 stone", "setblock 0 -25 -6 air", ""].map(String::from));
    lines.extend(["setblock -6 -25 -6 kelp", "", "setblock -5 -25 -6 stone", "", "", "setblock 0 -25 -6 kelp", "", "setblock 1 -25 -6 stone", "", ""].map(String::from));
    lines
}
