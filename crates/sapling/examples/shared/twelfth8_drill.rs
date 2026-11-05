//! The twelfth wave's group 8 drills (research/parity/TWELFTH-BRIEF.md): the byte a turn is told in,
//! armour stands teleported to yaws either side of zero and past a turn (`g8rot`); an armorer's
//! offers before it is turned into a zombie villager and after it is cured, and a neighbour's
//! (`g8cured`, village_drill.rs's ground: drill sapling with `SAPLING_DRILL_WORLD=lowflat`).

use super::creatures::{floor, with};
use crate::villages::{VILLAGE_SHOWN, every, village};

const ROT: &[&str] = &["AddEntity", "MoveEntityDelta", "MoveEntityAbsolute", "CommandOutput"];

/// The yaws each stand is teleported to, one stand a yaw.
const YAWS: [&str; 12] = ["-29.5", "-31", "330.5", "329", "-200", "200", "29.5", "-0.7", "-359.3", "539.3", "-180.5", "~-60"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "g8rot" => (rot(), ROT),
        "g8cured" => (cured(), VILLAGE_SHOWN),
        "g8wither" => (wither(), WITHER),
        _ => return None,
    })
}

const WITHER: &[&str] = &["AddEntity", "UpdateBlock"];

/// A wither's T lying on creature_drill.rs's floor, its skull row across (4, -26, 0) and its top
/// toward each way in turn, the last skull put by the actor; then an upright one.
fn wither() -> Vec<String> {
    let skull = "wither_skeleton_skull [\"facing_direction\"=1]";
    let mut lines = with(floor(), &["difficulty easy", "replaceitem entity @s slot.weapon.mainhand 0 wither_skeleton_skull 64"]);
    // (stem, arm row's middle, along the rows toward the set skull, the actor's stand and look, its
    // click from its feet); the actor puts the row's other end (the floor at (5, -27, 0) refused it).
    let lying: [([i32; 2], [i32; 2], [i32; 2], &str, &str); 4] = [
        ([2, 0], [3, 0], [0, -1], "4.5 -26 3.5 180 45", "0 -1 -2"),
        ([6, 0], [5, 0], [0, -1], "4.5 -26 3.5 180 45", "0 -1 -2"),
        ([4, -2], [4, -1], [1, 0], "1.5 -26 0.5 -90 45", "2 -1 0"),
        ([4, 2], [4, 1], [1, 0], "1.5 -26 0.5 -90 45", "2 -1 0"),
    ];
    for ([sx, sz], [ax, az], [rx, rz], stand, click) in lying {
        // The second run's bot missed one teleport and placed at its old spot: told twice, a second apart.
        lines.extend([format!("tp @s {stand}"), String::new(), format!("tp @s {stand}")]);
        lines.push(format!("setblock {sx} -26 {sz} soul_sand"));
        lines.push(format!("fill {} -26 {} {} -26 {} soul_sand", ax - rx, az - rz, ax + rx, az + rz));
        lines.extend([format!("setblock {} -26 {} {skull}", 4 + rx, rz), format!("setblock 4 -26 0 {skull}"), format!("!place {click}"), String::new(), String::new()]);
        lines.extend(["kill @e[type=wither]", "fill 1 -26 -3 7 -24 3 air", ""].map(String::from));
    }
    lines.extend(["setblock 6 -24 1 stone", "tp @s 6.5 -23 1.5 90 30", "setblock 3 -26 0 soul_sand", "fill 3 -25 -1 3 -25 1 soul_sand"].map(String::from));
    lines.extend([format!("setblock 3 -24 -1 {skull}"), format!("setblock 3 -24 0 {skull}"), "!place -3 -2 0".into(), String::new(), String::new()]);
    with(lines, &["kill @e[type=wither]", "fill 1 -26 -3 7 -23 3 air", "clear @s", ""])
}

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn rot() -> Vec<String> {
    let lines = ["kill @e[type=!player]", "gamerule domobspawning false", "gamemode creative @a", "tp @s 12.5 -60 2.5", "tp CmdWatcher 13.5 -60 2.5", "fill 0 -61 0 26 -61 10 stone", "fill 0 -60 0 26 -55 10 air", ""];
    let mut lines = owned(lines);
    for (stand, _) in YAWS.iter().enumerate() {
        lines.push(format!("summon armor_stand {}.5 -60 6.5 30 0", stand * 2));
    }
    lines.push(String::new());
    for (stand, yaw) in YAWS.iter().enumerate() {
        let x = stand * 2;
        lines.push(format!("tp @e[type=armor_stand,x={x}.5,y=-60,z=6.5,r=0.5] {x}.5 -60 6.5 {yaw} 0"));
    }
    with(lines, &["", "", "kill @e[type=armor_stand]", ""])
}

/// A neighbour armorer at (4.5, 4.5) and the one to be cured at (8.5, 4.5), each one's offers read;
/// the second killed by a zombie on hard, its zombie villager given a golden apple beside a bed and
/// iron bars; once cured (watched 250 s), its offers, then the neighbour's again.
fn cured() -> Vec<String> {
    let mut out = village(&[]);
    out.extend(owned([
        "setblock 6 -60 2 blast_furnace", "setblock 10 -60 2 blast_furnace", "setblock 10 -60 7 bed [\"direction\"=0]", "fill 7 -60 7 7 -59 7 iron_bars",
        "summon villager 4.5 -60 4.5 0 0 minecraft:become_armorer", "effect @e[type=villager_v2] slowness 1000 255 true", "tp @s 4.5 -60 2.5 facing 4.5 -59 4.5", "", "!trade villager_v2 0 0", "",
        "summon villager 8.5 -60 4.5 0 0 minecraft:become_armorer", "effect @e[type=villager_v2] slowness 1000 255 true", "tp @s 8.5 -60 2.5 facing 8.5 -59 4.5", "", "!trade villager_v2 0 0", "",
        "difficulty hard", "summon zombie 14.5 -60 -10.5", "effect @e[type=zombie] slowness 1000 255 true", "effect @e[type=zombie] fire_resistance 1000 0 true",
        // Held at once: the first run's zombie villager walked off and hit the neighbour.
        // r=3: sapling's held villager crept 1.5 off (slowness 255 does not stop it there).
        "damage @e[type=villager_v2,x=8.5,y=-60,z=4.5,r=3] 100 entity_attack entity @e[type=zombie,c=1]", "kill @e[type=zombie]",
        "effect @e[type=zombie_villager_v2] slowness 1000 255 true", "effect @e[type=zombie_villager_v2] fire_resistance 1000 0 true", "effect @e[type=zombie_villager_v2] weakness 1000 0 true",
        "difficulty easy", "replaceitem entity @s slot.weapon.mainhand 0 golden_apple 1", "execute at @e[type=zombie_villager_v2] run tp @s ~ ~ ~-1.5", "", "!interact zombie_villager_v2", "",
    ]));
    every(&mut out, "testfor @e[type=villager_v2]", 10, 25);
    out.extend(owned([
        "effect @e[type=villager_v2] slowness 1000 255 true", "execute at @e[type=villager_v2,x=8.5,y=-60,z=4.5,r=3] run tp @s ~ ~ ~-1.5", "", "!trade villager_v2 0 0", "",
        "kill @e[type=villager_v2,x=8.5,y=-60,z=4.5,r=3]", "execute at @e[type=villager_v2,x=4.5,y=-60,z=4.5,r=3] run tp @s ~ ~ ~-1.5", "", "", "!trade villager_v2 0 0", "",
        "kill @e[type=!player]", "clear @s", "gamemode creative @s", "",
    ]));
    out
}
