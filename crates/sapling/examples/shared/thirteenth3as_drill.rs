//! The thirteenth wave's group AS drills (research/parity/THIRTEENTH-BRIEF.md): a happy ghast refused
//! unharnessed, harnessed by a click, ridden by both bots, steered and left (`as3ghast`); tamed parrots
//! finding the actor's shoulders and leaving them (`as3parrot`); an allay handed dirt, its world kept
//! (`as3allay`) and loaded again (`as3allay2`, KEEP_WORLD=1).

use crate::creatures::{floor, waits, with};

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "as3ghast" => (ghast(), SHOWN),
        "as3parrot" => (parrot(), SHOWN),
        "as3allay" => (allay(), SHOWN),
        "as3allay2" => (allay_kept(), SHOWN),
        _ => return None,
    })
}

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "TakeItemEntity", "SetEntityLink", "SetEntityData", "EntityEvent", "LevelSoundEvent", "MobEquipment", "MobArmorEquipment",
    "InventorySlot", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "MovePlayer", "CorrectPlayerMovePrediction", "UpdateAttributes", "CommandOutput",
];

fn ghast() -> Vec<String> {
    // The bot reaches a mob it has no hitbox for as a player-sized box at its feet: the ghast floats over the actor.
    const OVER: &str = "tp @e[type=happy_ghast] 0.5 -22 0.5";
    let mut lines = with(floor(), &["tp CmdWatcher 8.5 -26 -6.5", "tp @s 0.5 -26 0.5 0 0", "summon happy_ghast 0.5 -22 0.5 0 0 minecraft:spawn_adult", "effect @e[type=happy_ghast] slowness 60 255 true", OVER, ""]);
    lines.extend(["ride @s start_riding @e[type=happy_ghast,c=1] teleport_rider", "", "replaceitem entity @s slot.weapon.mainhand 0 white_harness", OVER, "", "!interact happy_ghast", "", ""].map(String::from));
    lines.extend(["!look 0 1.6 8", OVER, "", "!mount happy_ghast", "", "", "ride CmdWatcher start_riding @e[type=happy_ghast,c=1] teleport_rider", "", ""].map(String::from));
    lines.extend(["effect @e[type=happy_ghast] clear", "", "!walk", "", "", "!stop", "", "!jump", "", "!stop", "", ""].map(String::from));
    lines.extend(["ride CmdWatcher stop_riding", "", "", "!dismount", "", ""].map(String::from));
    waits(&mut lines, 4);
    lines.extend(["tp @s 0.5 -26 0.5", "", "", "replaceitem entity @s slot.weapon.mainhand 0 shears", "effect @e[type=happy_ghast] slowness 60 255 true", OVER, "", "!interact happy_ghast", "", ""].map(String::from));
    with(lines, &["kill @e[type=!player]", "clear @s", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

/// One parrot tamed by its event (no owner), then one tamed by the actor's seeds; each left to find
/// the actor; the actor then falls, goes into water and is hurt, each followed by a wait.
fn parrot() -> Vec<String> {
    let mut lines = with(floor(), &["time set noon", "tp CmdWatcher 8.5 -26 -6.5", "gamemode survival @s", "effect @s resistance 300 4 true", "tp @s 0.5 -26 0.5 0 0", "summon parrot 3.5 -26 0.5 0 0 minecraft:on_tame", ""]);
    waits(&mut lines, 10);
    lines.extend(["summon parrot -2.5 -26 0.5", "replaceitem entity @s slot.weapon.mainhand 0 wheat_seeds 64", ""].map(String::from));
    for _ in 0..8 {
        lines.extend(["!interact parrot".to_owned(), String::new()]);
    }
    waits(&mut lines, 10);
    lines.extend(["tp @s 0.5 -22 0.5", ""].map(String::from));
    waits(&mut lines, 10);
    lines.extend(["fill 5 -27 -2 7 -27 2 water", "tp @s 6.5 -26 0.5", "", "", "tp @s 0.5 -26 0.5", "fill 5 -27 -2 7 -27 2 stone"].map(String::from));
    waits(&mut lines, 10);
    lines.extend(["damage @s 1 entity_attack", "", "", "kill @e[type=parrot]", ""].map(String::from));
    with(lines, &["gamemode creative @s", "clear @s", "effect @s clear", ""])
}

/// An allay handed dirt by the survival actor, which then leaves and joins again; the world is kept.
fn allay() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 -6.5", "gamemode survival @s", "replaceitem entity @s slot.weapon.mainhand 0 dirt 4", "summon allay 1.5 -26 0.5", "effect @e[type=allay] slowness 120 255 true", ""]);
    lines.extend(["tp @e[type=allay] -0.5 -25.5 1.5", "", "!interact allay", "", ""].map(String::from));
    lines.extend(["!rejoin", "", "", ""].map(String::from));
    lines
}

/// After `as3allay`'s restart: the actor joins again and is shown the allay, then drops dirt by it.
fn allay_kept() -> Vec<String> {
    let mut lines: Vec<String> = ["!rejoin", "", "", "effect @e[type=allay] clear", "tp @s 4.5 -26 0.5", "replaceitem entity @s slot.hotbar 0 dirt 2", "", "!drop 0 1"].map(String::from).into();
    lines.extend(["tp @s 12.5 -26 0.5".to_owned()]);
    waits(&mut lines, 10);
    lines
}
