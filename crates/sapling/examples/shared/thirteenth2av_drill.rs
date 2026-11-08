//! The thirteenth wave's group AV drills (research/parity/THIRTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27). `av2stomp`: a zombie and one turtle egg on the open floor, then a three-egg block, each
//! watched till stomped (how long at the eggs); then one zombie among four eggs at known distances (which it
//! goes for, in what order). `av2cat`: a cat tamed by feeding beside a chest alone, then a bed alone, 40 s
//! each; then its owner sleeps in the bed and wakes (where the cat is put); each phase twice, see `twice`. `av2allay2`:
//! `as3allay`'s world loaded again (KEEP_WORLD=1), the allay's owner dropping dirt and watched 40 s.

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &[
    "AddEntity", "AddItemEntity", "TakeItemEntity", "RemoveEntity", "SetEntityData", "MoveEntityDelta", "MoveEntityAbsolute", "UpdateBlock", "LevelEvent", "LevelSoundEvent", "EntityEvent",
    "MobEquipment", "CommandOutput",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "av2stomp" => (stomping(), SHOWN),
        "av2cat" => (cats(), SHOWN),
        "av2allay2" => (allay_kept(), SHOWN),
        _ => return None,
    })
}

const NIGHT: [&str; 4] = ["time set 18000", "gamerule mobgriefing true", "tp CmdWatcher -7.5 -26 -7.5", "tp @s -6.5 -26 -7.5"];

fn stomping() -> Vec<String> {
    let mut lines = with(floor(), &NIGHT);
    for egg in ["turtle_egg", "turtle_egg [\"turtle_egg_count\"=\"three_egg\"]"] {
        lines.extend([format!("setblock 4 -26 0 {egg}"), "summon zombie -1.5 -26 0.5".into(), String::new()]);
        waits(&mut lines, 15);
        lines.extend(["kill @e[type=zombie]".to_owned(), "setblock 4 -26 0 air".into(), String::new()]);
    }
    let eggs = ["setblock 4 -26 0 turtle_egg", "setblock 0 -26 -6 turtle_egg", "setblock -5 -26 3 turtle_egg", "setblock 9 -26 5 turtle_egg"];
    lines.extend(eggs.map(String::from));
    lines.extend(["summon zombie 0.5 -26 0.5".to_owned(), String::new()]);
    waits(&mut lines, 30);
    with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -24 8 air", "time set 6000", ""])
}

fn tamed_cat() -> Vec<String> {
    let mut lines = with(floor(), &["time set 12000", "tp CmdWatcher -7.5 -26 -7.5", "gamemode survival @s", "tp @s 0.5 -26 0.5 -90 0", "clear @s", "give @s cod 64", ""]);
    lines.extend(["summon cat 2.5 -26 0.5 0 0 minecraft:spawn_wild_adult", "effect @e[type=cat] slowness 60 255 true", ""].map(String::from));
    for _ in 0..8 {
        lines.extend(["tp @s 0.5 -26 0.5 -90 0".to_owned(), "!feedall cat".into(), String::new()]);
    }
    let mut lines = with(lines, &["testfor @e[type=cat]", "clear @s", "effect @e[type=cat] clear", ""]);
    waits(&mut lines, 35);
    lines
}

/// Its owner's click toggles a tamed cat's sitting, and a feed while in love toggled it too (the first run), so
/// each phase runs twice with a click between: in one of the two the cat stands.
fn twice(lines: &mut Vec<String>, phase: &[&str], secs: usize) {
    for _ in 0..2 {
        lines.extend(["tp @s 0.5 -26 0.5 -90 0", "", "!interact cat", ""].map(String::from));
        lines.extend(phase.iter().map(|line| line.to_string()));
        waits(lines, secs);
    }
}

fn cats() -> Vec<String> {
    let mut lines = tamed_cat();
    twice(&mut lines, &["setblock 5 -26 3 chest", ""], 40);
    lines.extend(["setblock 5 -26 3 air", "setblock 5 -26 -3 bed [\"direction\"=0]", ""].map(String::from));
    twice(&mut lines, &[""], 40);
    lines.extend(["setblock 5 -26 -3 air", "setblock 5 -26 -2 air", "setblock -3 -26 3 bed [\"direction\"=0]", "gamerule playerssleepingpercentage 50", ""].map(String::from));
    twice(&mut lines, &["time set 13000", "tp @s -2.5 -26 1.5 0 0", "", "!sleep 0 0 2", ""], 20);
    with(lines, &["gamerule playerssleepingpercentage 100", "gamemode creative @s", "kill @e[type=!player]", "setblock -3 -26 3 air", "setblock -3 -26 4 air", "time set 6000", ""])
}

/// After `as3allay`'s restart: the actor joins again and drops dirt by the allay it gave dirt to, watched 40 s.
fn allay_kept() -> Vec<String> {
    let mut lines: Vec<String> = ["!rejoin", "", "", "effect @e[type=allay] clear", "tp @s 4.5 -26 0.5", "replaceitem entity @s slot.hotbar 0 dirt 2", "", "!drop 0 1"].map(String::from).into();
    lines.extend(["tp @s 12.5 -26 0.5".to_owned()]);
    waits(&mut lines, 40);
    lines
}
