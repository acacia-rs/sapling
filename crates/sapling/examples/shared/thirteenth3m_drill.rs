//! The thirteenth wave's group M drills (research/parity/THIRTEENTH-BRIEF.md), on warden_drill.rs's floor
//! (stone at y -27, the survival actor at the middle). `m3wither`: a wither on normal hurt 10 at a time
//! every 3 s down past half health, then shown a held cow to dash at; the world is kept for its saved
//! fields. `m3wither0`: a wither on normal left unhurt 20 s (its first saved fields). `m3struck`: items
//! and an orb struck by a summoned bolt with fire ticks off. `m3reflect`: an arrow shot at a summoned
//! dragon fireball. `m3rod`: a Channeling trident thrown at a lightning rod in thunder (open sky).
//! `m3inblock`: stone set at slowed cows' feet, some walled in. `m3rideview`: a watcher comes back to
//! the actor riding a boat, then a saddled pig.

use super::creatures::{waits, with};
use super::warden::{healed, start};

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "MoveEntityDelta", "SetEntityMotion", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "UpdateAttributes", "UpdateBlock", "TakeItemEntity", "CommandOutput",
];

const VIEWED: &[&str] = &["AddPlayer", "AddEntity", "RemoveEntity", "SetEntityLink", "SetEntityData", "MobEquipment", "MobArmorEquipment", "UpdateAttributes", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "m3wither" => (wither(), SHOWN),
        "m3wither0" => (wither_unhurt(), SHOWN),
        "m3struck" => (struck(), SHOWN),
        "m3reflect" => (reflect(), SHOWN),
        "m3rod" => (rod(), SHOWN),
        "m3inblock" => (in_block(), SHOWN),
        "m3rideview" => (ride_view(), VIEWED),
        _ => return None,
    })
}

fn normal(time: &str) -> Vec<String> {
    with(start(time), &["difficulty normal", "gamerule mobgriefing false", "effect @s resistance 900 4 true"])
}

fn wither() -> Vec<String> {
    let mut lines = normal("noon");
    lines.push("summon wither 0.5 -26 -12.5".into());
    healed(&mut lines, 12);
    for _ in 0..18 {
        lines.push("damage @e[type=wither] 10".into());
        healed(&mut lines, 3);
    }
    lines.extend(["gamemode creative @s", "summon cow 0.5 -26 6.5", "effect @e[type=cow] slowness 120 255 true", "effect @e[type=cow] health_boost 120 9 true", "effect @e[type=cow] instant_health 1 9 true"].map(String::from));
    waits(&mut lines, 30);
    with(lines, &["kill @e[type=cow]", "gamerule mobgriefing true"])
}

fn wither_unhurt() -> Vec<String> {
    let mut lines = normal("noon");
    lines.push("summon wither 0.5 -26 -12.5".into());
    healed(&mut lines, 20);
    with(lines, &["gamemode creative @s", "gamerule mobgriefing true"])
}

fn struck() -> Vec<String> {
    let mut lines = with(normal("noon"), &["gamerule dofiretick false", "gamemode creative @s"]);
    for item in ["stick 4", "netherite_ingot 1", "nether_star 1", "cobblestone 8"] {
        let count = item.split(' ').nth(1).unwrap_or("1");
        lines.extend([format!("replaceitem entity @s slot.hotbar 0 {item}"), String::new(), format!("!drop 0 {count}"), String::new()]);
    }
    waits(&mut lines, 2);
    lines.extend(["tp @e[type=item] 0.5 -26 -4.5", "summon xp_orb 0.5 -26 -4.5", "tp @s 0.5 -26 6.5"].map(String::from));
    waits(&mut lines, 2);
    lines.push("summon lightning_bolt 0.5 -26 -4.5".into());
    waits(&mut lines, 4);
    with(lines, &["testfor @e[type=item]", "testfor @e[type=xp_orb]", "kill @e[type=!player]", "gamerule dofiretick true"])
}

fn reflect() -> Vec<String> {
    let mut lines = with(normal("noon"), &["replaceitem entity @s slot.hotbar 0 bow", "replaceitem entity @s slot.inventory 0 arrow 1 16"]);
    lines.extend(["summon dragon_fireball 0.5 -24.4 -8.5", "tp @s 0.5 -26 0.5 facing 0.5 -24.4 -8.5", "", "!use", "", "!release"].map(String::from));
    waits(&mut lines, 6);
    with(lines, &["testfor @e[type=dragon_fireball]", "kill @e[type=!player]"])
}

/// Stone set in the feet cell of slowed cows standing at cell middles (head clear): one in the open, one
/// with a wall cell south (+z), one walled south and north, one walled all round but east, one off the
/// middle at x .8; each 6 apart along x at z 4.
fn in_block() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s"]);
    let walls: [&[(i32, i32)]; 5] = [&[], &[(0, 1)], &[(0, 1), (0, -1)], &[(0, 1), (0, -1), (-1, 0)], &[]];
    for (i, sides) in walls.iter().enumerate() {
        let x = -12 + 6 * i as i32;
        for (dx, dz) in sides.iter() {
            lines.push(format!("setblock {} -26 {} stone", x + dx, 4 + dz));
        }
        let off = if i == 4 { 0.8 } else { 0.5 };
        lines.push(format!("summon cow {} -26 4.5", x as f32 + off));
    }
    lines.push("effect @e[type=cow] slowness 60 255 true".into());
    waits(&mut lines, 2);
    for i in 0..5 {
        lines.push(format!("setblock {} -26 4 stone", -12 + 6 * i));
    }
    waits(&mut lines, 5);
    with(lines, &["kill @e[type=!player]", "fill -14 -26 2 14 -24 6 air"])
}

/// The actor rides a boat, then a saddled pig, while the watcher goes 3000 off and comes back (a late viewer).
fn ride_view() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "summon boat 0.5 -26 3.5", "", "ride @s start_riding @e[type=boat]"]);
    waits(&mut lines, 2);
    let away = ["tp CmdWatcher 3000 -26 3000", "", "", "", "tp CmdWatcher 14.5 -26 14.5", "", "", ""];
    lines.extend(away.map(String::from));
    lines.extend(["ride @s stop_riding", "kill @e[type=boat]", "summon pig 4.5 -26 3.5", "effect @e[type=pig] slowness 60 255 true", "replaceitem entity @e[type=pig] slot.saddle 0 saddle", "", "ride @s start_riding @e[type=pig]", "", ""].map(String::from));
    lines.extend(away.map(String::from));
    with(lines, &["ride @s stop_riding", "kill @e[type=!player]"])
}

fn rod() -> Vec<String> {
    let mut lines = with(normal("noon"), &["gamerule dofiretick false", "setblock 0 -26 -6 lightning_rod", "weather thunder", "replaceitem entity @s slot.hotbar 0 trident", "enchant @s channeling 1"]);
    waits(&mut lines, 6);
    lines.extend(["tp @s 0.5 -26 0.5", "", "!look 0 0.6 -6", "!use", "", "", "!release"].map(String::from));
    waits(&mut lines, 5);
    with(lines, &["testfor @e[type=lightning_bolt]", "weather clear", "setblock 0 -26 -6 air", "kill @e[type=!player]", "gamerule dofiretick true"])
}
