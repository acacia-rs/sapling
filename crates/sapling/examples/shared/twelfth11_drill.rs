//! The twelfth wave's group 11 drills (research/parity/TWELFTH-BRIEF.md), on village_drill.rs's
//! ground (drill sapling with `SAPLING_DRILL_WORLD=lowflat`): two unskilled villagers given bread in
//! their inventories, both 3 (`g11love`), one of them 3 (`g11love1`), both 2 (`g11love2`), both 6
//! or 12 (`g11love6`, `g11love12`), one of them 24 (`g11share`), both 3 with one bed left
//! (`g11nobed`), none watched 300 s (`g11unfed`); 24 bread dropped at one's feet (`g11breed`). Also four vexes
//! summoned by command by the actor (`g11vex`).

use crate::villages::{UNSKILLED, every, village};

pub const BREED_SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "TakeItemEntity", "SetEntityData", "EntityEvent", "LevelSoundEvent", "MoveEntityDelta", "UpdateBlock", "MobEquipment",
];

const FIRST: &str = "@e[type=villager_v2,c=1,x=-4.5,y=-60,z=4.5]";
const BOTH: &str = "@e[type=villager_v2]";

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// The village with its two villagers, `before` run, each named in `given` given `count` bread,
/// then the villagers counted every 6 s for 120 s.
fn love(before: &[&str], given: &str, count: u32) -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.extend(before.iter().map(|line| line.to_string()));
    out.push(format!("replaceitem entity {given} slot.inventory 0 bread {count}"));
    every(&mut out, "testfor @e[type=villager_v2]", 6, 20);
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", ""]));
    out
}

/// The village with its two villagers given nothing, counted every 6 s for 300 s.
fn unfed() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    every(&mut out, "testfor @e[type=villager_v2]", 6, 50);
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", ""]));
    out
}

fn breed() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.extend(owned(["give @s bread 24", "tp @s -3.5 -60 2.5 facing -3.5 -60 4.5", "", "!drop 0 24", ""]));
    every(&mut out, "testfor @e[type=villager_v2]", 5, 26);
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", ""]));
    out
}

/// Four vexes summoned 6 off the actor (in survival at midnight on a stone floor at y -20, healed
/// each second): whether a vex with no evoker charges, watched 30 s (bds-vex.txt had one).
fn vexes() -> Vec<String> {
    let mut out = owned([
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule falldamage false", "difficulty easy", "time set midnight", "tp @s 0.5 -19 0.5", "gamemode survival @s",
        "fill -20 -20 -20 20 -20 20 stone", "fill -20 -19 -20 20 -10 20 air", "fill -20 -9 -20 20 1 20 air", "",
        "summon vex 0.5 -18 -5.5", "summon vex 6.5 -18 0.5", "summon vex 0.5 -18 6.5", "summon vex -5.5 -18 0.5",
    ]);
    out.extend(std::iter::repeat_n("effect @s instant_health 1 4".to_owned(), 30));
    out.extend(owned(["kill @e[type=!player]", "effect @s clear", "gamerule falldamage true", "gamemode creative @s", ""]));
    out
}

const VEX_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "SetEntityData", "EntityEvent", "LevelSoundEvent", "MoveEntityDelta", "SetEntityMotion", "UpdateAttributes"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "g11love" => (love(&[], BOTH, 3), BREED_SHOWN),
        "g11love1" => (love(&[], FIRST, 3), BREED_SHOWN),
        "g11love2" => (love(&[], BOTH, 2), BREED_SHOWN),
        "g11love6" => (love(&[], BOTH, 6), BREED_SHOWN),
        "g11love12" => (love(&[], BOTH, 12), BREED_SHOWN),
        "g11share" => (love(&[], FIRST, 24), BREED_SHOWN),
        "g11nobed" => (love(&["setblock -6 -60 8 air", "setblock -2 -60 8 air", ""], BOTH, 3), BREED_SHOWN),
        "g11breed" => (breed(), BREED_SHOWN),
        "g11unfed" => (unfed(), BREED_SHOWN),
        "g11vex" => (vexes(), VEX_SHOWN),
        _ => return None,
    })
}
