//! The thirteenth wave's group BA drills (research/parity/THIRTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27), both run with BDS_GDB watchpoints (docs/research/worldgen-bds-debugger.md "Linux").
//! `ba2boat`: boats summoned on the floor at z 4.5 and at (2, 2). `ba2ghast`: the actor rides a harnessed
//! happy ghast, holds jump, walks, jumps, then holds walk, sprint and jump together (`!leap`). The bot reports
//! jump while riding only with a patched acacia (vehicles.md "Happy ghast").

use super::creatures::{floor, with};

const SHOWN: &[&str] = &["AddEntity", "MoveEntityDelta", "MovePlayer", "SetEntityData", "SetEntityLink", "CommandOutput", "CorrectPlayerMovePrediction"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ba2boat" => (boats(), SHOWN),
        "ba2ghast" => (ghast(), SHOWN),
        _ => return None,
    })
}

fn boats() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -7.5 -26 -7.5", "tp @s -6.5 -26 -7.5"]);
    for at in ["4.5 -26 4.5", "9.5 -26 4.5", "-0.5 -26 4.5", "2 -26 2"] {
        lines.extend([format!("summon boat {at}"), String::new(), String::new(), "kill @e[type=boat]".into()]);
    }
    with(lines, &["tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}

fn ghast() -> Vec<String> {
    const OVER: &str = "tp @e[type=happy_ghast] 0.5 -22 0.5";
    let mut lines = with(floor(), &["tp CmdWatcher 8.5 -26 -6.5", "tp @s 0.5 -26 0.5 0 0", "summon happy_ghast 0.5 -22 0.5 0 0 minecraft:spawn_adult", "effect @e[type=happy_ghast] slowness 60 255 true", OVER, ""]);
    lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 white_harness", OVER, "", "!interact happy_ghast", "", "", "!look 0 1.6 8", OVER, "", "!mount happy_ghast", "", ""].map(String::from));
    lines.extend(["effect @e[type=happy_ghast] clear", "", "!jump", "", "", "", "!stop", "", "!walk", "!jump", "", "", "!stop", "", "!leap", "", "", "!stop", "", "!dismount", "", ""].map(String::from));
    with(lines, &["kill @e[type=!player]", "clear @s", "tp @s 0.5 -26 0.5", "tp CmdWatcher 0.5 -26 0.5", ""])
}
