//! The fourteenth wave's group T drills (research/parity/FOURTEENTH-BRIEF.md). On the flat ground at
//! midnight: `tt1ghast`, a held ghast's fireballs ending on the warded survival actor with nothing
//! behind it, from four spots; `tt1warden`, fresh wardens held 10 off while commands set, fill and
//! kill near them and a sculk sensor; `tt1patrol`, the creative actor standing at night with mob
//! spawning on (a world past `pillager_patrol`'s age, set before the run). On creature_drill.rs's
//! floor: `tt1piglin`, bare piglins offered one armour piece each; `tt1worn`, piglins that took a
//! diamond helmet over an iron one, hurt and killed; `tt1shulker`, shulkers hurt below half.

use super::creatures::{floor, waits, with};

const GHAST_SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "SetEntityData", "EntityEvent", "LevelEvent", "LevelSoundEvent", "MovePlayer", "CommandOutput",
];
const WARDEN_SHOWN: &[&str] = &["AddEntity", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "LevelEventGeneric", "CommandOutput", "UpdateBlock", "RemoveEntity"];
const PIGLIN_SHOWN: &[&str] = &[
    "AddEntity", "AddItemEntity", "RemoveEntity", "TakeItemEntity", "MobEquipment", "MobArmorEquipment", "SetEntityData", "LevelSoundEvent", "EntityEvent", "CommandOutput",
];
const SHULKER_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "CommandOutput"];
const PATROL_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "CommandOutput", "MobArmorEquipment", "MobEquipment"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "tt1ghast" => (ghast(), GHAST_SHOWN),
        "tt1warden" => (warden(), WARDEN_SHOWN),
        "tt1piglin" => (piglins(), PIGLIN_SHOWN),
        "tt1worn" => (worn(), PIGLIN_SHOWN),
        "tt1shulker" => (shulkers(), SHULKER_SHOWN),
        "tt1patrol" => (patrol(), PATROL_SHOWN),
        _ => return None,
    })
}

fn owned(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

/// The flat ground cleared about the spawn at `time`, both players creative, the watcher far south.
fn ground(time: u32) -> Vec<String> {
    let mut out = owned(&["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "gamerule mobgriefing false", "weather clear", "difficulty easy"]);
    out.extend([format!("time set {time}"), "gamemode creative @a".into(), "effect @a clear".into(), "clear @s".into()]);
    out.extend(owned(&["fill -12 -60 -12 12 -50 16 air", "tp CmdWatcher 0.5 -60 -14.5", "tp @s 0.5 -60 0.5 0 0", ""]));
    out
}

fn end() -> Vec<String> {
    owned(&["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", "clear @s", "gamerule mobgriefing true", "tp @s 0.5 -60 0.5 0 0", ""])
}

/// A held ghast's shots end on the warded survival actor, open ground behind it: three from each
/// of four spots (ahead, aside, high ahead, low aside).
fn ghast() -> Vec<String> {
    let mut out = ground(18000);
    out.extend(owned(&["gamemode survival @s", "effect @s resistance 1000 255 true", "effect @s fire_resistance 1000 0 true", "tp @s 0.5 -60 0.5 0 0"]));
    out.extend(owned(&["summon ghast 0.5 -59 14.5", "effect @e[type=ghast] slowness 1000 255 true"]));
    for at in ["0.5 -59 14.5", "9.5 -59 11.5", "0.5 -53 12.5", "-8.5 -58 9.5"] {
        out.push(format!("tp @e[type=ghast] {at}"));
        for _ in 0..3 {
            waits(&mut out, 4);
            out.push("tp @s 0.5 -60 0.5 0 0".into());
        }
    }
    waits(&mut out, 2);
    out.extend(end());
    out
}

const FRESH: [&str; 3] = ["kill @e[type=warden]", "summon warden 0.5 -60 10.5", "effect @e[type=warden] slowness 1000 255 true"];

/// For each disturbance a fresh warden held 10 off, a sculk sensor 4 off it: blocks set, replaced,
/// broken and filled by command, a door's lower half taken away, and pigs `/kill`ed, killed by
/// `/damage` and killed after being heard hurt.
fn warden() -> Vec<String> {
    let mut out = ground(18000);
    out.extend(owned(&["setblock 4 -60 6 sculk_sensor", "setblock -3 -60 6 wooden_door", ""]));
    let parts: [&[&str]; 9] = [
        &["setblock 2 -60 6 stone"],
        &["setblock 2 -60 6 air"],
        &["setblock 2 -60 6 stone", "", "", "", "", "setblock 2 -60 6 air destroy"],
        &["fill 1 -60 7 2 -60 7 stone"],
        &["fill 1 -60 7 2 -60 7 air"],
        &["setblock -3 -60 6 air"],
        &["summon pig 0.5 -60 6.5", "effect @e[type=pig] slowness 1000 255 true", "", "", "", "", "kill @e[type=pig]"],
        &["summon pig 0.5 -60 6.5", "effect @e[type=pig] slowness 1000 255 true", "", "", "", "", "damage @e[type=pig] 100"],
        &["summon pig 0.5 -60 6.5", "effect @e[type=pig] slowness 1000 255 true", "damage @e[type=pig] 1", "", "", "", "damage @e[type=pig] 1", "", "", "", "kill @e[type=pig]"],
    ];
    for part in parts {
        out.extend(owned(&FRESH));
        waits(&mut out, 4);
        out.extend(owned(part));
        waits(&mut out, 5);
    }
    out.extend(owned(&["setblock 4 -60 6 air", "fill 1 -60 6 2 -60 7 air", "kill @e[type=item]"]));
    out.extend(end());
    out
}

/// A piglin 3 east of the creative actor with its hands and every armour slot bare, killed after `offer` is
/// dropped towards it and 6 s pass.
fn offered(mut lines: Vec<String>, head: &str, offer: &str) -> Vec<String> {
    lines = with(lines, &["kill @e[type=item]", "summon piglin 3.5 -26 0.5 90 0 minecraft:spawn_adult"]);
    for slot in ["slot.armor.chest", "slot.armor.legs", "slot.armor.feet", "slot.weapon.offhand", "slot.weapon.mainhand"] {
        lines.push(format!("replaceitem entity @e[type=piglin] {slot} 0 air"));
    }
    lines.push(format!("replaceitem entity @e[type=piglin] slot.armor.head 0 {head}"));
    with(lines, &[&format!("replaceitem entity @s slot.hotbar 0 {offer} 1"), "", "!drop 0 1"])
}

/// Each armour piece offered to a bare piglin, the iron helmet four times.
fn piglins() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 7.5"]);
    let offers = [
        "iron_helmet", "iron_chestplate", "iron_helmet", "iron_leggings", "iron_boots", "iron_helmet", "chainmail_helmet", "diamond_helmet", "leather_helmet", "netherite_helmet", "copper_helmet",
        "turtle_helmet", "iron_helmet", "diamond_boots",
    ];
    for offer in offers {
        lines = offered(lines, "air", offer);
        waits(&mut lines, 6);
        lines = with(lines, &["kill @e[type=piglin]", "", ""]);
    }
    with(lines, &["kill @e[type=!player]", "clear @s", ""])
}

/// A piglin in an iron helmet offered a diamond one, which it takes; then hurt `hits` times by 4
/// as an attack with no attacker, healed between, and killed, so the picked-up helmet falls.
fn worn() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 7.5"]);
    for hits in [0, 5, 10] {
        lines = offered(lines, "iron_helmet", "diamond_helmet");
        waits(&mut lines, 4);
        lines.push("effect @e[type=piglin] slowness 1000 255 true".into());
        for _ in 0..hits {
            lines = with(lines, &["damage @e[type=piglin] 4 entity_attack", "effect @e[type=piglin] instant_health 1 1 true"]);
        }
        lines = with(lines, &["", "kill @e[type=piglin]", "", ""]);
    }
    with(lines, &["kill @e[type=!player]", "clear @s", ""])
}

/// Four shulkers on the floor hurt by 3 a second, ten times, with no attacker.
fn shulkers() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 7.5", "tp @s 0.5 -26 -6.5"]);
    for at in ["4.5 -26 3.5", "8.5 -26 -3.5", "12.5 -26 3.5", "4.5 -26 -3.5"] {
        lines.push(format!("summon shulker {at}"));
    }
    waits(&mut lines, 3);
    for _ in 0..10 {
        lines.push("damage @e[type=shulker] 3".into());
    }
    waits(&mut lines, 6);
    with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", ""])
}

/// The creative actor stands 8 minutes at midnight on easy with mob spawning on, the watcher at the
/// spawn too; pillagers that come are named in `AddEntity` with their gear.
fn patrol() -> Vec<String> {
    let mut out = owned(&["kill @e[type=!player]", "gamerule domobspawning true", "gamerule dodaylightcycle false", "weather clear", "difficulty easy", "time set 18000"]);
    out.extend(owned(&["gamemode creative @a", "tp CmdWatcher 0.5 -60 0.5", "tp @s 0.5 -60 0.5", "mobevent minecraft:pillager_patrols_event true", "mobevent events_enabled true", ""]));
    waits(&mut out, 480);
    out.extend(owned(&["gamerule domobspawning false", "kill @e[type=!player]", ""]));
    out
}
