//! The tenth wave's villager and mob drills (research/parity/TENTH-BRIEF.md "Group F"), on
//! village_drill.rs's ground (stand at y -60; drill sapling with `SAPLING_DRILL_WORLD=lowflat`):
//! villagers by a hunting or a still zombie, in their village and alone (`f3fear`); villagers killed
//! by a zombie on hard and normal (`f3convert`); zombie villagers cured (`f3cure`); a village of 20
//! beds and 20 armorers spawning golems (`f3golem`); how far a villager takes a job site (`f3poi`);
//! villagers given bread (`f3breed`).

use crate::villages::{UNSKILLED, VILLAGE_SHOWN, every, village, wait};

fn lines<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// The villagers within 8 and within 4 of the zombie, every second for 15 s.
fn count_near_zombie(out: &mut Vec<String>) {
    for _ in 0..15 {
        out.extend(lines([
            "execute at @e[type=zombie,c=1] run testfor @e[type=villager_v2,r=8]", "execute at @e[type=zombie,c=1] run testfor @e[type=villager_v2,r=4]", "",
        ]));
    }
}

/// A zombie 3 and 5 from two unhurt villagers: hunting in their village, then 200 east (no beds,
/// no bell) held still and hunting.
fn fear() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.extend(lines(["effect @e[type=villager_v2] resistance 1000 4 true", "summon zombie 0.5 -60 4.5", "effect @e[type=zombie] fire_resistance 1000 0 true"]));
    count_near_zombie(&mut out);
    out.extend(lines([
        "kill @e[type=zombie]", "tp @a 200.5 -60 0.5", "fill 176 -61 -24 224 -61 24 grass_block", "fill 176 -60 -24 224 -52 24 air", "",
        "summon villager 196.5 -60 4.5 0 0 minecraft:become_unskilled", "summon villager 198.5 -60 4.5 0 0 minecraft:become_unskilled",
    ]));
    wait(&mut out, 10);
    out.extend(lines([
        "effect @e[type=villager_v2] resistance 1000 4 true", "summon zombie 201.5 -60 4.5", "effect @e[type=zombie] fire_resistance 1000 0 true",
        "effect @e[type=zombie] slowness 1000 255 true",
    ]));
    count_near_zombie(&mut out);
    out.extend(lines(["kill @e[type=zombie]", "tp @e[type=villager_v2] 197.5 -60 4.5", "", "summon zombie 201.5 -60 4.5", "effect @e[type=zombie] fire_resistance 1000 0 true"]));
    count_near_zombie(&mut out);
    out.extend(lines(["kill @e[type=!player]", "gamemode creative @s", ""]));
    out
}

/// A zombie summoned 60 off, held still or not, then put 4 from the villagers by `x`.
fn zombie_brought(out: &mut Vec<String>, x: i32, still: bool) {
    out.push(format!("summon zombie {}.5 -60 4.5", x + 60));
    out.push("effect @e[type=zombie] fire_resistance 1000 0 true".into());
    if still {
        out.push("effect @e[type=zombie] slowness 1000 255 true".into());
    }
    out.extend([String::new(), format!("tp @e[type=zombie] {}.5 -60 4.5", x + 4)]);
    count_near_zombie(out);
    out.extend(["kill @e[type=zombie]".into(), format!("tp @e[type=villager_v2] {x}.5 -60 4.5"), String::new()]);
}

/// [`fear`] with the zombie brought from afar, so a still one never moved near them: in the
/// village still and hunting, then 200 east, alone, the same.
fn fear_brought() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.push("effect @e[type=villager_v2] resistance 1000 4 true".into());
    zombie_brought(&mut out, -4, true);
    zombie_brought(&mut out, -4, false);
    out.extend(lines([
        "kill @e[type=!player]", "tp @a 200.5 -60 0.5", "fill 176 -61 -24 224 -61 24 grass_block", "fill 176 -60 -24 224 -52 24 air", "",
        "summon villager 196.5 -60 4.5 0 0 minecraft:become_unskilled", "summon villager 198.5 -60 4.5 0 0 minecraft:become_unskilled",
    ]));
    wait(&mut out, 10);
    out.push("effect @e[type=villager_v2] resistance 1000 4 true".into());
    zombie_brought(&mut out, 197, true);
    zombie_brought(&mut out, 197, false);
    out.extend(lines(["kill @e[type=!player]", "gamemode creative @s", ""]));
    out
}

/// Villagers killed by a zombie's hit: an armorer at work on hard, then six unskilled on normal.
fn convert() -> Vec<String> {
    let mut out = village(&["setblock 6 -60 4 blast_furnace", "summon villager 4.5 -60 4.5 0 0 minecraft:become_armorer"]);
    out.extend(lines([
        "difficulty hard", "summon zombie 12.5 -60 -12.5", "effect @e[type=zombie] slowness 1000 255 true", "effect @e[type=zombie] fire_resistance 1000 0 true",
        "damage @e[type=villager_v2] 100 entity_attack entity @e[type=zombie,c=1]",
    ]));
    wait(&mut out, 3);
    out.extend(lines(["testfor @e[type=zombie_villager_v2]", "kill @e[type=zombie_villager_v2]", "difficulty normal"]));
    for x in 0..6 {
        out.push(format!("summon villager {}.5 -60 -4.5 0 0 minecraft:become_unskilled", -6 + 2 * x));
    }
    wait(&mut out, 2);
    out.push("damage @e[type=villager_v2] 100 entity_attack entity @e[type=zombie,c=1]".into());
    wait(&mut out, 3);
    out.extend(lines(["testfor @e[type=zombie_villager_v2]", "kill @e[type=!player]", "difficulty easy", "gamemode creative @s", ""]));
    out
}

/// Three zombie villagers held still and weakened, each given a golden apple by the actor, the
/// third with a bed and iron bars beside it: when each becomes a villager, watched 250 s.
fn cure() -> Vec<String> {
    let mut out = village(&[]);
    out.extend(lines(["setblock -12 -60 -12 bed [\"direction\"=0]", "fill -10 -60 -14 -10 -59 -10 iron_bars"]));
    for (index, (x, z)) in [(10, -12), (0, -12), (-11, -12)].into_iter().enumerate() {
        // BDS's `/summon` names it `zombie_villager` (`zombie_villager_v2` is a syntax error).
        out.push(format!("summon zombie_villager {x}.5 -60 {z}.5"));
        out.extend(lines([
            "effect @e[type=zombie_villager_v2] slowness 1000 255 true", "effect @e[type=zombie_villager_v2] fire_resistance 1000 0 true",
            "effect @e[type=zombie_villager_v2] weakness 1000 0 true",
        ]));
        out.push(format!("replaceitem entity @s slot.weapon.mainhand 0 golden_apple {}", index + 1));
        out.push(format!("tp @s {x}.5 -60 {}.5 facing {x}.5 -59 {z}.5", z + 2));
        out.extend(lines(["", "!interact zombie_villager_v2", ""]));
    }
    out.push("tp @s 0.5 -60 0.5".into());
    every(&mut out, "testfor @e[type=villager_v2]", 10, 25);
    out.extend(lines(["kill @e[type=!player]", "clear @s", "gamemode creative @s", ""]));
    out
}

/// Twenty beds, each with an armorer and its blast furnace beside it, mob spawning on: iron golems
/// counted every 5 s for 150 s.
fn golems() -> Vec<String> {
    let mut summon = Vec::new();
    for at in 0..20 {
        let x = -19 + 2 * at;
        summon.push(format!("setblock {x} -60 -8 bed [\"direction\"=0]"));
        summon.push(format!("setblock {x} -60 -4 blast_furnace"));
        summon.push(format!("summon villager {x}.5 -60 -5.5 0 0 minecraft:become_armorer"));
    }
    let summon: Vec<&str> = summon.iter().map(String::as_str).collect();
    let mut out = village(&summon);
    out.extend(lines(["effect @e[type=villager_v2] resistance 1000 4 true", "gamerule domobspawning true", "testfor @e[type=villager_v2]"]));
    every(&mut out, "testfor @e[type=iron_golem]", 5, 30);
    out.extend(lines(["gamerule domobspawning false", "kill @e[type=!player]", "gamemode creative @s", ""]));
    out
}

/// An unskilled villager by a bell, a blast furnace `d` east of it, five places 200 apart: which
/// villagers take the job in 30 s.
fn poi() -> Vec<String> {
    let mut out = village(&[]);
    for (index, d) in [6, 10, 14, 18, 26].into_iter().enumerate() {
        let x = 400 + 200 * index as i32;
        out.extend([
            format!("tp @a {x}.5 -60 -6.5"), format!("fill {} -61 -32 {} -61 32 grass_block", x - 32, x + 32), format!("fill {} -60 -32 {} -52 32 air", x - 32, x + 32),
            String::new(), format!("setblock {x} -60 0 bell"), format!("setblock {} -60 0 blast_furnace", x + d), format!("summon villager {}.5 -60 1.5 0 0 minecraft:become_unskilled", x - 1),
            "effect @e[type=villager_v2] slowness 1000 255 true".into(),
        ]);
        wait(&mut out, 30);
        out.extend([format!("testfor @e[type=villager_v2,family=armorer,x={x},y=-60,z=0,r=40]"), format!("testfor @e[type=villager_v2,x={x},y=-60,z=0,r=40]")]);
    }
    out.extend(lines(["tp @a 0.5 -60 0.5", "kill @e[type=!player]", "gamemode creative @s", ""]));
    out
}

/// Two unskilled villagers in the three-bed village, 24 bread thrown at their feet: babies watched 90 s.
fn breed() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.extend(lines(["give @s bread 24", "tp @s -3.5 -60 2.5 facing -3.5 -60 4.5", "", "!drop 0 24", ""]));
    every(&mut out, "testfor @e[type=villager_v2]", 5, 18);
    out.extend(lines(["kill @e[type=!player]", "gamemode creative @s", ""]));
    out
}

/// The lines and shown packets of the drill `name`, if it is one of these.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "f3fear" => (fear(), VILLAGE_SHOWN),
        "f3fear2" => (fear_brought(), VILLAGE_SHOWN),
        "f3convert" => (convert(), VILLAGE_SHOWN),
        "f3cure" => (cure(), VILLAGE_SHOWN),
        "f3golem" => (golems(), VILLAGE_SHOWN),
        "f3poi" => (poi(), VILLAGE_SHOWN),
        "f3breed" => (breed(), VILLAGE_SHOWN),
        _ => return None,
    })
}
