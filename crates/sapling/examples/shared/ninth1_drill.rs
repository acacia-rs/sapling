//! The ninth wave's village drills (research/parity/NINTH-BRIEF.md "Group 1", villagers.md and
//! effects-raids.md "Not drilled"): when an armorer restocks after a trade (`n1restock`), when an
//! armorer whose blast furnace is broken drops its profession, traded or not (`n1drop`), whether a
//! pillager summoned by a raider joins the raid (`n1recruit`), and iron golems coming to a village
//! of 20 beds and 10 working armorers (`n1golems`) or 20 (`n1golems20`). On village_drill.rs's ground; drill sapling
//! with `SAPLING_DRILL_WORLD=lowflat`.

use crate::villages::{UNSKILLED, VILLAGE_SHOWN, every, village, wait};

const NEAR: &str = "testfor @e[family=monster,x=0,y=-60,z=6,r=24]";

fn lines<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// The actor beside the newest villager, then `!trade` of offer 0 `times` times.
fn trade(out: &mut Vec<String>, times: u32) {
    out.extend(["execute as @e[type=villager_v2,c=1,tag=buyer] at @s run tp CmdActor ~ ~ ~-1.5 facing ~ ~1 ~".to_owned(), String::new(), format!("!trade villager_v2 0 {times}"), String::new()]);
}

/// An armorer at its blast furnace traded once, then its screen opened every 5 s for 50 s: the
/// `uses` of its first offer tell when it restocked.
fn restock() -> Vec<String> {
    let mut out = village(&["setblock 6 -60 -4 blast_furnace", "summon villager 4.5 -60 -4.5 0 0 minecraft:become_armorer"]);
    out.extend(lines(["tag @e[type=villager_v2] add buyer", "give @s coal 64", "give @s coal 64"]));
    trade(&mut out, 1);
    for _ in 0..10 {
        wait(&mut out, 5);
        trade(&mut out, 0);
    }
    out.extend(lines(["clear @s", "gamemode creative @s", ""]));
    out
}

/// Two armorers at their blast furnaces, the second traded once; both furnaces broken, and their
/// `Variant` watched 90 s.
fn drop_profession() -> Vec<String> {
    let mut out = village(&[
        "setblock 8 -60 -4 blast_furnace", "summon villager 6.5 -60 -4.5 0 0 minecraft:become_armorer", "setblock -8 -60 -4 blast_furnace",
        "summon villager -6.5 -60 -4.5 0 0 minecraft:become_armorer",
    ]);
    out.extend(lines(["tag @e[type=villager_v2,x=-6.5,y=-60,z=-4.5,r=2] add buyer", "give @s coal 64"]));
    trade(&mut out, 1);
    out.extend(lines(["tp @s 0.5 -60 0.5", "setblock 8 -60 -4 air", "setblock -8 -60 -4 air"]));
    wait(&mut out, 90);
    out.extend(lines(["clear @s", "gamemode creative @s", ""]));
    out
}

/// The first wave of a raid, a pillager summoned two out from one of its pillagers, the wave
/// killed: whether the next wave waits for the newcomer.
fn recruit() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.extend(lines(["effect @e[type=villager_v2] resistance 1000 4 true", "effect @e[type=villager_v2] slowness 1000 255 true", "effect @s bad_omen 600 0"]));
    wait(&mut out, 52);
    out.extend(lines(["tag @e[family=monster] add wave", "execute at @e[tag=wave,type=pillager,c=1] run summon pillager ~2 ~ ~", NEAR]));
    wait(&mut out, 3);
    out.extend(lines(["kill @e[tag=wave]", NEAR]));
    every(&mut out, NEAR, 2, 20);
    out.extend(lines(["kill @e[family=monster]", NEAR]));
    every(&mut out, NEAR, 2, 15);
    out.extend(lines(["kill @e[family=monster]", "effect @s clear", "gamemode creative @s", ""]));
    out
}

/// Twenty beds and `armorers` armorers each at a blast furnace, mob spawning on: iron golems
/// counted every 5 s for 150 s. BDS's rule wants 20 villagers holding beds.
fn golems(armorers: i32) -> Vec<String> {
    let mut summon = Vec::new();
    for at in 0..20 {
        summon.push(format!("setblock {} -60 -14 bed [\"direction\"=0]", -19 + 2 * at));
    }
    for at in 0..armorers {
        let x = -19 + 2 * at;
        summon.push(format!("setblock {x} -60 16 blast_furnace"));
        summon.push(format!("summon villager {x}.5 -60 14.5 0 0 minecraft:become_armorer"));
    }
    let summon: Vec<&str> = summon.iter().map(String::as_str).collect();
    let mut out = village(&summon);
    out.extend(lines(["gamerule domobspawning true", "testfor @e[type=villager_v2]"]));
    every(&mut out, "testfor @e[type=iron_golem]", 5, 30);
    out.extend(lines(["gamerule domobspawning false", "kill @e[type=iron_golem]", "gamemode creative @s", ""]));
    out
}

/// The lines and shown packets of the drill `name`, if it is one of these.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "n1restock" => (restock(), VILLAGE_SHOWN),
        "n1drop" => (drop_profession(), VILLAGE_SHOWN),
        "n1recruit" => (recruit(), VILLAGE_SHOWN),
        "n1golems" => (golems(10), VILLAGE_SHOWN),
        "n1golems20" => (golems(20), VILLAGE_SHOWN),
        _ => return None,
    })
}
