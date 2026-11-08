//! The fourteenth wave's group L drills (research/parity/FOURTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27, the actor at 0.5 -26 0.5): a survival actor drinks water facing each way and up and down
//! (`l1drink`); hits an armour stand 40 times (`l1stand`); slides down a ladder and falls in the open onto a
//! held cow, hitting it (`l1crit`); dies at 1 health on a cactus, in a berry bush, on magma, under an anvil
//! and a stalactite, by a guardian's spikes and by a starred rocket under a glass roof (`l1death`); smashes
//! a golem with the survival watcher and its own wolf beside it, the watcher itself and a lifted golem with
//! a mace (`l1mace`).

use acacia_bot::Bot;

use super::creatures::{floor, waits, with};

const DRINK_SHOWN: &[&str] = &["LevelSoundEvent", "CompletedUsingItem", "EntityEvent", "MovePlayer"];
const STAND_SHOWN: &[&str] = &["UpdateAttributes", "LevelEvent", "SetEntityData", "AddItemEntity", "RemoveEntity", "CommandOutput"];
const CRIT_SHOWN: &[&str] = &["Animate", "EntityEvent", "LevelSoundEvent", "UpdateAttributes", "CommandOutput", "UpdateBlock"];
const DEATH_SHOWN: &[&str] = &[
    "Text", "DeathInfo", "EntityEvent", "SetHealth", "Respawn", "UpdateAttributes", "LevelSoundEvent", "AddEntity", "RemoveEntity", "CommandOutput", "ItemStackResponse", "UpdateBlock",
    "ContainerOpen", "AddItemEntity",
];
const MACE_SHOWN: &[&str] = &[
    "EntityEvent", "Animate", "LevelSoundEvent", "LevelEvent", "LevelEventGeneric", "SetEntityMotion", "UpdateAttributes", "SetEntityData", "MovePlayer", "CorrectPlayerMovePrediction",
    "SetEntityLink", "CommandOutput", "AddEntity",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "l1drink" => (drink(), DRINK_SHOWN),
        "l1stand" => (stand(), STAND_SHOWN),
        "l1crit" => (crit(), CRIT_SHOWN),
        "l1death" => (death(), DEATH_SHOWN),
        "l1mace" => (mace(), MACE_SHOWN),
        _ => return None,
    })
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

const HEAL: &str = "effect @s instant_health 1 4 true";

/// Water drunk facing south, west, north, east, south-east, and south looking down and up; honey facing west.
fn drink() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 6.5", "gamemode survival @s", ""]);
    let looks = ["0 1.62 2", "-2 1.62 0", "0 1.62 -2", "2 1.62 0", "1.4 1.62 1.4", "0 0 2", "0 3.2 2"];
    for look in looks {
        lines = with(lines, &[&hold("potion 1 0"), &format!("!look {look}"), "", "!eat"]);
        waits(&mut lines, 3);
    }
    lines = with(lines, &[&hold("honey_bottle 1"), "!look -2 1.62 0", "", "!eat"]);
    waits(&mut lines, 3);
    with(lines, &["gamemode creative @s", "clear @s", ""])
}

/// Forty hits a second apart on an armour stand east of the survival actor (each a lone hit, which only
/// wobbles it); then ten on a held cow.
fn stand() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 6.5", "gamemode survival @s", "summon armor_stand 2.5 -26 0.5", "!look 2 1 0", ""]);
    for _ in 0..40 {
        lines = with(lines, &["!hit armor_stand"]);
    }
    lines = with(lines, &["kill @e[type=!player]", "summon cow 2.5 -26 0.5 0 0 minecraft:entity_spawned", "effect @e[type=cow] slowness 100 255 true", "effect @e[type=cow] resistance 100 4 true", ""]);
    for _ in 0..10 {
        lines = with(lines, &["!hit cow"]);
    }
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", ""])
}

/// A ladder column on a wall west of the actor's cell: put 5 up in it, the actor slides down and hits a
/// cow penned 2 east, 1.2 over the floor; the same with no ladder (a plain fall), then with the ladder from 3 up.
fn crit() -> Vec<String> {
    let ladder = "fill 0 -26 0 0 -18 0 ladder [\"facing_direction\"=5]";
    let mut lines = with(floor(), &[
        "tp CmdWatcher 0.5 -26 6.5", "fill -1 -26 0 -1 -18 0 stone", ladder, "fill 1 -26 -1 3 -26 1 barrier", "fill 2 -26 0 2 -26 0 air", "summon cow 2.5 -26 0.5 0 0 minecraft:entity_spawned", "effect @e[type=cow] slowness 200 255 true",
        "effect @e[type=cow] resistance 200 4 true", "gamemode survival @s", "effect @s resistance 200 4 true", "tp @s 0.5 -26 0.5 -90 0", "",
    ]);
    for (laddered, up) in [(true, 5), (false, 5), (true, 3), (false, 3)] {
        lines.push(if laddered { ladder.into() } else { "fill 0 -26 0 0 -18 0 air".into() });
        lines = with(lines, &["tp @s 0.5 -26 0.5 -90 0", "", &format!("!smash cow {up} 1.2"), "", ""]);
    }
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", "fill -1 -26 0 0 -18 0 air", ""])
}

/// Each way of dying with the actor first hurt to 1 health; told by Text and DeathInfo.
fn death() -> Vec<String> {
    let low = |lines: Vec<String>, at: &str| with(lines, &[&format!("tp @s {at}"), "gamemode survival @s", HEAL, "", "damage @s 19", ""]);
    let mut lines = with(floor(), &["tp CmdWatcher 0.5 -26 7.5", "gamerule showdeathmessages true", "gamerule doimmediaterespawn true", "setblock 2 -27 -4 sand", "setblock 2 -26 -4 cactus", ""]);
    lines = with(low(lines, "4.5 -26 -3.5"), &["tp @s 2.5 -25 -3.5", "", ""]);
    lines = with(low(lines, "4.5 -26 -1.5"), &["setblock 5 -26 -1 sweet_berry_bush [\"growth\"=3]", "tp @s 5.5 -26 -0.5 -90 0", "!walk", "", "!stop", ""]);
    lines = with(low(lines, "8.5 -26 -3.5"), &["setblock 7 -27 -4 magma", "tp @s 7.5 -26 -3.5", "", ""]);
    lines = with(low(lines, "10.5 -26 -3.5"), &["setblock 10 -20 -4 anvil", "", ""]);
    lines = with(low(lines, "12.5 -26 -3.5"), &["setblock 12 -17 -4 stone", "setblock 12 -18 -4 pointed_dripstone [\"hanging\"=true,\"dripstone_thickness\"=\"tip\"]", "", "setblock 12 -17 -4 air", "", ""]);
    // Spikes: BDS took `/damage ... entity @s` on a held guardian as the actor's hit (fourteenth2e).
    lines = with(lines, &["fill 13 -27 2 15 -24 4 glass", "fill 14 -26 3 14 -24 3 water", "summon guardian g 14.5 -26 3.5", "effect @e[name=g] slowness 100 255 true"]);
    lines = with(low(lines, "12.5 -26 3.5 -90 0"), &["damage @e[name=g] 1 entity_attack entity @s", "", "", "kill @e[name=g]"]);
    // A starred rocket under a glass roof 3 over the floor, crafted in a table: gunpowder and dye, then paper, gunpowder and the star.
    lines = with(lines, &[
        "tp @s 0.5 -26 0.5 0 0", "setblock 0 -26 2 crafting_table", "clear @s", "replaceitem entity @s slot.hotbar 1 gunpowder 2", "replaceitem entity @s slot.hotbar 2 red_dye 1",
        "replaceitem entity @s slot.hotbar 3 paper 1", "", "!click 0 0 2", "", "!grid! fireworks 1 1 2", "", "!grid! fireworks 3 3 1 0", "", "!close", "",
        // The bot's own view of its hotbar misses the crafts (crafting_requests.rs): a game mode change sends it all.
        "gamemode creative @s", "",
        "fill -3 -23 -3 3 -23 -1 glass", "tp @s 0.5 -26 -1.5 0 0",
    ]);
    lines = with(low(lines, "0.5 -26 -1.5 0 0"), &["!look 0 0 1.5", "", "!click 0 -1 1"]);
    waits(&mut lines, 4);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "clear @s", "tp @s 0.5 -26 0.5 -90 0", "fill -8 -26 -8 16 -16 8 air", "fill -8 -27 -8 16 -27 8 stone", ""])
}

/// A mace in hand, the survival actor facing east: smashes a golem penned 2 east from 4 up with the
/// survival watcher 2 off it and the actor's own wolf sat 2.2 off; then the watcher penned there; then a
/// held golem lifted by levitation.
fn mace() -> Vec<String> {
    // One high: the bot does not hit through a barrier (bds-l1mace.txt's first run).
    let pen = ["fill 1 -26 -1 3 -26 1 barrier", "fill 2 -26 0 2 -26 0 air"];
    let mut lines = with(floor(), &["gamerule pvp true", "gamemode survival @s", "gamemode survival CmdWatcher", "effect CmdWatcher resistance 200 3 true", "tp CmdWatcher 2.5 -26 2.5"]);
    lines = with(lines, &["summon wolf 1.5 -26 -1.5", "effect @e[type=wolf] slowness 200 255 true", &hold("bone 16"), "!look 1 0.5 -2", ""]);
    for _ in 0..6 {
        lines = with(lines, &["!interact wolf", ""]);
    }
    lines = with(lines, &[&hold("mace"), "", "!interact wolf", ""]);
    lines = with(lines, &pen);
    lines = with(lines, &["summon iron_golem 2.5 -26 0.5", HEAL, "", "!smash iron_golem 4 2.2", "", "", ""]);
    lines = with(lines, &["kill @e[type=iron_golem]", "effect CmdWatcher instant_health 1 4 true", "tp CmdWatcher 2.5 -26 0.5", "tp @s 0.5 -26 0.5 -90 0", HEAL, "", "!smash player 4 2.2", "", "", ""]);
    lines = with(lines, &["tp CmdWatcher 0.5 -26 6.5", "effect CmdWatcher instant_health 1 4 true", "kill @e[type=wolf]", "tp @s 0.5 -26 0.5 -90 0", HEAL]);
    lines = with(lines, &pen);
    lines = with(lines, &["summon iron_golem 2.5 -26 0.5", "effect @e[type=iron_golem] slowness 10 255 true", "effect @e[type=iron_golem] levitation 10 0 true", "", "!smash iron_golem 4 2.2", "", "", ""]);
    with(lines, &["kill @e[type=!player]", "gamemode creative @a", "effect @a clear", "clear @s", "fill -8 -26 -8 16 -16 8 air", ""])
}

/// `!hotbar n` selects hotbar slot `n`. False for any other line.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let Some(slot) = line.strip_prefix("!hotbar ").and_then(|slot| slot.parse().ok()) else { return false };
    println!("## hotbar {slot}: {:?}", actor.select_hotbar(slot));
    true
}
