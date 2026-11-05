//! The thirteenth wave's group O drills (research/parity/THIRTEENTH-BRIEF.md), on warden_drill.rs's floor
//! (stone at y -27, the survival actor at the middle). `o3rod`: bolts summoned in a rod's cell, one and
//! two over a rod, twice each (its sparks). `o3wither`: a wither on normal hunting the warded actor (its
//! flight, bar and side heads). `o3wet`: blazes and snow golems in water, then in the rain. `o3stand`:
//! armour stands in fire and lava, one in gear, and one hit twice a second apart; `o3stand2`: their burn
//! out of lava, and second hits 2 to 14 ticks on (`!twohits`).

use super::creatures::{waits, with};
use super::warden::{healed, start};

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "AddItemEntity", "MoveEntityDelta", "SetEntityMotion", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "UpdateAttributes", "UpdateBlock", "BossEvent", "CommandOutput",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "o3rod" => (rod(), SHOWN),
        "o3wither" => (wither(), SHOWN),
        "o3wet" => (wet(), SHOWN),
        "o3stand" => (stand(), SHOWN),
        "o3stand2" => (stand_again(), SHOWN),
        _ => return None,
    })
}

/// `!twohits n kind`: hits the newest `kind` it knows, waits `n` ticks and hits it again. False for any
/// other line.
pub async fn act(actor: &mut acacia_bot::Bot, line: &str) -> bool {
    let Some((gap, kind)) = line.strip_prefix("!twohits ").and_then(|rest| rest.split_once(' ')) else { return false };
    let kind = format!("minecraft:{kind}");
    let Some(id) = actor.state().entities.iter().filter(|entity| entity.kind == kind).map(|entity| entity.runtime_id).max() else {
        println!("## no {kind} to hit");
        return true;
    };
    println!("## hit {id}: {:?}", actor.attack(id).await);
    let _ = actor.wait_ticks(gap.parse().unwrap_or(1)).await;
    println!("## hit {id}: {:?}", actor.attack(id).await);
    true
}

/// A stand in gear a second in lava, then out (its burn and what it drops), and one in diamond, which
/// outlasts its burn; then fresh stands hit twice 2 to 14 ticks apart.
fn stand_again() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamerule dofiretick false", "gamemode creative @s", "summon armor_stand 6.5 -26 -5.5", "summon armor_stand -5.5 -26 -5.5"]);
    lines.extend(["replaceitem entity @e[type=armor_stand,x=6.5,y=-26,z=-5.5,r=1] slot.armor.head 0 iron_helmet", "replaceitem entity @e[type=armor_stand,x=6.5,y=-26,z=-5.5,r=1] slot.weapon.mainhand 0 stone_sword"].map(String::from));
    for (slot, piece) in [("head", "helmet"), ("chest", "chestplate"), ("legs", "leggings"), ("feet", "boots")] {
        lines.push(format!("replaceitem entity @e[type=armor_stand,x=-5.5,y=-26,z=-5.5,r=1] slot.armor.{slot} 0 diamond_{piece}"));
    }
    lines.extend(["setblock 6 -26 -6 lava", "setblock -6 -26 -6 lava", "fill -6 -26 -6 6 -26 -6 air"].map(String::from));
    waits(&mut lines, 30);
    lines.extend(["testfor @e[type=armor_stand]", "kill @e[type=!player]", "gamemode survival @s", "tp @s 0.5 -26 0.5 0 0"].map(String::from));
    for gap in [2, 4, 6, 8, 10, 14] {
        lines.extend(["summon armor_stand 0.5 -26 2.5".into(), String::new(), format!("!twohits {gap} armor_stand"), String::new(), "testfor @e[type=armor_stand]".into(), "kill @e[type=!player]".into()]);
    }
    with(lines, &["gamemode creative @s", "gamerule dofiretick true"])
}

/// Rods at x -8, 0 and 8, z -6; each gets a bolt in its own cell, then one 1 over it, then 2 over it.
fn rod() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamerule dofiretick false", "gamemode creative @s"]);
    for x in [-8, 0, 8] {
        lines.push(format!("setblock {x} -26 -6 lightning_rod"));
    }
    waits(&mut lines, 1);
    for up in [0, 1, 2, 1, 0] {
        for x in [-8, 0, 8] {
            lines.push(format!("summon lightning_bolt {}.5 {} -5.5", x, -26 + up));
            waits(&mut lines, 1);
        }
        waits(&mut lines, 2);
    }
    with(lines, &["fill -9 -26 -6 9 -26 -6 air", "kill @e[type=!player]", "gamerule dofiretick true"])
}

fn wither() -> Vec<String> {
    let mut lines = with(start("noon"), &["difficulty normal", "gamerule mobgriefing false", "effect @s resistance 900 4 true", "summon wither 0.5 -26 -12.5"]);
    healed(&mut lines, 40);
    with(lines, &["kill @e[type=wither]", "gamemode creative @s", "gamerule mobgriefing true"])
}

/// Pools at x -8 and 8 (z -8) hold a blaze and a snow golem; two more stand dry at z 8 for the rain.
fn wet() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamemode creative @s", "fill -9 -26 -9 -7 -25 -7 glass", "fill 7 -26 -9 9 -25 -7 glass", "setblock -8 -25 -8 water", "setblock 8 -25 -8 water", "setblock -8 -26 -8 water", "setblock 8 -26 -8 water"]);
    lines.extend(["summon blaze -7.5 -26 -7.5", "summon snow_golem 8.5 -26 -7.5"].map(String::from));
    lines.extend(["tp @e[type=blaze] -7.5 -26 -7.5", "tp @e[type=snow_golem] 8.5 -26 -7.5", "effect @e[type=!player] slowness 120 255 true"].map(String::from));
    waits(&mut lines, 6);
    lines.extend(["kill @e[type=!player]", "fill -9 -26 -9 9 -25 -7 air", "summon blaze -4.5 -26 8.5", "summon snow_golem 4.5 -26 8.5", "effect @e[type=!player] slowness 120 255 true", "weather rain"].map(String::from));
    waits(&mut lines, 8);
    with(lines, &["weather clear", "kill @e[type=!player]"])
}

/// Stands at z -6: in fire at x -6, on lava at x 0, in fire with gear at x 6; then one hit twice.
fn stand() -> Vec<String> {
    let mut lines = with(start("noon"), &["gamerule dofiretick false", "gamemode creative @s"]);
    for x in [-6, 0, 6] {
        lines.push(format!("summon armor_stand {x}.5 -26 -5.5"));
    }
    lines.extend(["replaceitem entity @e[type=armor_stand,x=6.5,y=-26,z=-5.5,r=1] slot.armor.head 0 iron_helmet", "replaceitem entity @e[type=armor_stand,x=6.5,y=-26,z=-5.5,r=1] slot.weapon.mainhand 0 stone_sword"].map(String::from));
    lines.extend(["setblock -6 -26 -6 fire", "setblock 0 -27 -6 lava", "setblock 0 -26 -6 lava", "setblock 6 -26 -6 fire"].map(String::from));
    waits(&mut lines, 30);
    lines.extend(["testfor @e[type=armor_stand]", "fill -7 -27 -7 7 -26 -5 stone", "fill -7 -26 -7 7 -26 -5 air", "kill @e[type=!player]", "gamemode survival @s", "summon armor_stand 0.5 -26 2.5", "tp @s 0.5 -26 0.5 0 0", "", "!hit armor_stand", "!hit armor_stand", "", "testfor @e[type=armor_stand]"].map(String::from));
    waits(&mut lines, 2);
    with(lines, &["kill @e[type=!player]", "gamemode creative @s", "gamerule dofiretick true"])
}
