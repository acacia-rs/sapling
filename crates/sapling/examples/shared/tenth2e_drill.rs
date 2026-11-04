//! The tenth wave's item and player drills (research/parity/TENTH-BRIEF.md "Group E"), on
//! creature_drill.rs's floor (stone at y -27): armour and a spear worn to breaking (`t2ewear`),
//! Soul Speed boots on soul sand (`t2esoles`), stews past data 14 and absorption under hits
//! (`t2eitems`), instant potions broken near players (`t2esplash`), bows shot standing and walking
//! (`t2ebow`), casts walking and at a wall (`t2efish`), the captain's banner and a loom's seventh
//! pattern (`t2ebanner`), heads put on from the hand (`t2edon`), raw book edits (`t2ebook`),
//! spears mended in the grid (`t2espear`), a merge and a tipped arrow (`t2epack`). The world
//! drills are in tenth2e_world.rs.

#[path = "tenth2e_world.rs"]
mod world;

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "TakeItemEntity", "SetEntityMotion", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "MobEffect", "UpdateAttributes",
    "SetHealth", "MobEquipment", "MobArmorEquipment", "PlayerArmorDamage", "InventorySlot", "InventoryContent", "CompletedUsingItem", "HurtAnimation", "UpdateBlock", "BlockEntityData",
    "SpawnParticleEffect", "ContainerOpen", "ContainerClose", "ItemStackResponse",
];
const MOVES: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta", "SetEntityMotion", "SetEntityData", "LevelSoundEvent", "EntityEvent", "MovePlayer"];

/// The drill called `name` of this file or the world drills'.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "t2ewear" => (wear(), SHOWN),
        "t2esoles" => (soles(), SHOWN),
        "t2eitems" => (items(), SHOWN),
        "t2esplash" => (splash(), SHOWN),
        "t2ebow" => (bow(), MOVES),
        "t2efish" => (fish(), MOVES),
        "t2ebanner" => (banner(), SHOWN),
        "t2edon" => (don(), SHOWN),
        "t2ebook" => (book(), SHOWN),
        "t2espear" => (spear(), SHOWN),
        "t2epack" => (pack(), SHOWN),
        _ => return world::drill(name),
    })
}

fn held(kind: &str) -> String {
    format!("effect @e[type={kind}] slowness 120 255 true")
}

const HEAL: &str = "effect @s instant_health 1 9 true";

/// The survival actor on the floor, healed.
fn survival() -> Vec<String> {
    with(floor(), &["gamemode survival @s", "effect @s health_boost 300 4 true", HEAL])
}

/// Each material's four pieces two short of the Java game's uses, hit four times by 4; an iron
/// spear and an iron sword three short, hitting a cow held still four times.
fn wear() -> Vec<String> {
    let mut lines = survival();
    let lasts = [("leather", 5), ("chainmail", 15), ("iron", 15), ("golden", 7), ("diamond", 33), ("netherite", 37)];
    for (material, base) in lasts {
        for (part, piece, share) in [("head", "helmet", 11), ("chest", "chestplate", 16), ("legs", "leggings", 15), ("feet", "boots", 13)] {
            lines.push(format!("replaceitem entity @s slot.armor.{part} 0 {material}_{piece} 1 {}", base * share - 2));
        }
        lines.push(String::new());
        for _ in 0..4 {
            lines.extend(["damage @s 4 entity_attack".to_owned(), HEAL.to_owned()]);
        }
    }
    lines.extend(["replaceitem entity @s slot.armor.head 0 air", "replaceitem entity @s slot.armor.chest 0 air", "replaceitem entity @s slot.armor.legs 0 air", "replaceitem entity @s slot.armor.feet 0 air"].map(String::from));
    lines.extend(["summon cow 2.0 -26 0.5", &held("cow"), "effect @e[type=cow] health_boost 120 9 true", "effect @e[type=cow] regeneration 120 255 true"].map(String::from));
    for weapon in ["iron_spear 1 247", "iron_sword 1 247", "golden_spear 1 27"] {
        lines.extend([format!("replaceitem entity @s slot.weapon.mainhand 0 {weapon}"), String::new()]);
        for _ in 0..4 {
            lines.extend(["tp @e[type=cow] 2.0 -26 0.5".to_owned(), "!hit cow".to_owned()]);
        }
    }
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "clear @s"])
}

/// Soul Speed III boots worn across a strip of soul sand three times, 5 s each.
fn soles() -> Vec<String> {
    let mut lines = with(survival(), &["fill -8 -27 6 16 -27 6 soul_sand", "replaceitem entity @s slot.weapon.mainhand 0 diamond_boots", "enchant @s soul_speed 3", "!use", ""]);
    for _ in 0..3 {
        lines.extend(["tp @s -7.5 -26 6.5 -90 0", "", "!walk"].map(String::from));
        waits(&mut lines, 5);
        lines.push("!stop".into());
    }
    with(lines, &["fill -8 -27 6 16 -27 6 stone", "gamemode creative @s", "clear @s"])
}

/// A hungry actor eats stews of data 0, 15, 16 and 20; then, given absorption 8, takes 3, 3 by
/// magic, 4, and a fall of 2.
fn items() -> Vec<String> {
    let mut lines = with(survival(), &["effect @s hunger 10 200 true", "", "", "", "effect @s clear", HEAL]);
    for data in [0, 15, 16, 20] {
        lines.extend([format!("replaceitem entity @s slot.weapon.mainhand 0 suspicious_stew 1 {data}"), String::new(), "!eat".into(), String::new(), String::new(), String::new()]);
    }
    lines.extend(["effect @s clear", "effect @s health_boost 300 4 true", HEAL, "", "effect @s absorption 30 1 true", ""].map(String::from));
    for hit in ["3 entity_attack", "3 magic", "4 entity_attack", "2 fall"] {
        lines.extend([format!("damage @s {hit}"), String::new()]);
    }
    with(lines, &["effect @s clear", "gamemode creative @s", "clear @s"])
}

/// A dispenser breaks harming, then healing on the hurt watcher, on the actor, the watcher beside
/// it 2.1 south, 2.1 east and 1 north (leftover_drill.rs's `splashshare`, instant potions).
fn splash() -> Vec<String> {
    let mut lines = with(floor(), &["setblock 0 -26 4 dispenser [\"facing_direction\"=5]", "gamemode survival @s", "gamemode survival CmdWatcher", "tp @s 3.5 -26 4.5 -90 0"]);
    lines.extend(["effect @a health_boost 300 4 true", "effect @a instant_health 1 9 true", ""].map(String::from));
    for (potion, hurt) in [("23", false), ("21", true)] {
        for watcher in ["3.5 -26 6.6", "5.6 -26 4.5", "3.5 -26 3"] {
            lines.push(format!("tp CmdWatcher {watcher}"));
            if hurt {
                lines.extend(["damage @a 12 magic".to_owned(), String::new()]);
            }
            lines.extend([format!("replaceitem block 0 -26 4 slot.container 0 splash_potion 1 {potion}"), "setblock -1 -26 4 redstone_block".into(), String::new(), String::new()]);
            lines.extend(["setblock -1 -26 4 air", "effect @a instant_health 1 9 true", ""].map(String::from));
        }
    }
    with(lines, &["setblock 0 -26 4 air", "effect @a clear", "gamemode creative @s", "gamemode creative CmdWatcher"])
}

/// Six full draws of a bow looking east standing, then two while walking east.
fn bow() -> Vec<String> {
    let mut lines = with(survival(), &["replaceitem entity @s slot.weapon.mainhand 0 bow", "replaceitem entity @s slot.inventory 0 arrow 64", "tp @s -7.5 -26 0.5 -90 0", ""]);
    for _ in 0..6 {
        lines.extend(["!use", "", "!release", ""].map(String::from));
    }
    for _ in 0..2 {
        lines.extend(["tp @s -7.5 -26 0.5 -90 0", "", "!walk", "!use", "", "!release", "!stop", ""].map(String::from));
    }
    with(lines, &["kill @e[type=arrow]", "gamemode creative @s", "clear @s"])
}

/// Two stones dropped one by one to merge where they lie; a tipped arrow (poison, data 26) shot.
fn pack() -> Vec<String> {
    let mut lines = with(survival(), &["replaceitem entity @s slot.hotbar 0 stone 2", "", "!drop 0 1", "", "!drop 0 1", "", "", "", "kill @e[type=item]"]);
    lines.extend(["replaceitem entity @s slot.hotbar 0 bow", "replaceitem entity @s slot.inventory 0 arrow 1 26", "", "!use", "", "!release", "", ""].map(String::from));
    with(lines, &["kill @e[type=arrow]", "gamemode creative @s", "clear @s"])
}

/// Casts looking east standing, walking, and at a glass wall 2.5 ahead.
fn fish() -> Vec<String> {
    let mut lines = with(survival(), &["replaceitem entity @s slot.weapon.mainhand 0 fishing_rod", "tp @s -7.5 -26 0.5 -90 0", "", "!fish 3", ""]);
    lines.extend(["tp @s -7.5 -26 0.5 -90 0", "", "!walk", "!fish 3", "!stop", ""].map(String::from));
    lines.extend(["fill -5 -26 -2 -5 -20 3 glass", "tp @s -7.5 -26 0.5 -90 0", "", "!fish 3", ""].map(String::from));
    with(lines, &["fill -5 -26 -2 -5 -20 3 air", "gamemode creative @s", "clear @s"])
}

/// Heads and a carved pumpkin put on by a click in the air, each over the one before; then a helmet.
fn don() -> Vec<String> {
    let mut lines = survival();
    for item in ["carved_pumpkin", "zombie_head", "skeleton_skull", "dragon_head", "player_head", "iron_helmet"] {
        lines.extend([format!("replaceitem entity @s slot.weapon.mainhand 0 {item}"), String::new(), "!use".into(), String::new()]);
    }
    with(lines, &["gamemode creative @s", "clear @s"])
}

/// Two iron spears worn 150 and 130, the first enchanted, mended in a crafting table's grid; then
/// the other way round.
fn spear() -> Vec<String> {
    let mut lines = with(floor(), &["setblock ~2 ~ ~ crafting_table", ""]);
    for (first, second) in [(150, 130), (130, 150)] {
        let given = [format!("replaceitem entity @s slot.hotbar 0 iron_spear 1 {first}"), String::new(), "enchant @s sharpness 2".into(), format!("replaceitem entity @s slot.hotbar 1 iron_spear 1 {second}"), String::new(), "!inv".into()];
        lines.extend(["clear @s".into(), "!forget".into(), String::new()]);
        lines.extend(given);
        lines.extend(["!click 2 0 0", "", "!grid repair 1 0 1", "", "!close", "", "!inv"].map(String::from));
    }
    lines
}

/// A book of one page, its page then sent raw as 256, 257 and 1000 letters, then signed raw with
/// titles of 40, 17 and 16, the stack listed after each.
fn book() -> Vec<String> {
    let mut lines = with(survival(), &["replaceitem entity @s slot.weapon.mainhand 0 writable_book", "", "!write x", "", "!inv"]);
    // A 1000-letter page got the actor kicked (bds-t2ebook1.txt), so the longest goes last.
    for (verb, letter, length) in [("!rawpage", "a", 256), ("!rawpage", "b", 257), ("!rawsign", "u", 17), ("!rawsign", "t", 40), ("!rawpage", "c", 512)] {
        lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 writable_book".into(), "!write x".into(), String::new(), format!("{verb} {}", letter.repeat(length)), String::new(), "!inv".into()]);
    }
    with(lines, &["gamemode creative @s", "clear @s"])
}

/// A raid captain killed for its banner, which the actor takes and sets down; then a plain banner
/// given seven patterns at a loom, the inventory listed after each.
fn banner() -> Vec<String> {
    let mut lines = with(survival(), &["summon pillager 3.5 -26 0.5 0 0 minecraft:spawn_as_illager_captain", "", "kill @e[type=pillager]", "", "tp @s 3.5 -26 0.5 -90 0", "", "", "!inv", "kill @e[type=item]"]);
    lines.extend(["tp @s 0.5 -26 0.5 -90 0", "", "!place 2 -1 0", "", ""].map(String::from));
    lines.extend(["setblock 2 -26 0 loom", "clear @s", "replaceitem entity @s slot.hotbar 0 banner 1 15", "replaceitem entity @s slot.hotbar 1 red_dye 64", ""].map(String::from));
    for pattern in ["bo", "ls", "rs", "ts", "bs", "cs", "ms"] {
        lines.extend([format!("!loom 2 0 0 {pattern}"), String::new(), "!inv".into()]);
    }
    with(lines, &["!close", "setblock 2 -26 0 air", "setblock 2 -26 1 air", "kill @e[type=item]", "gamemode creative @s", "clear @s"])
}
