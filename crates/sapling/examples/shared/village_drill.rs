//! The village and raid drills of bds_command_probe (docs/research/effects-raids.md "Raids",
//! villagers.md "Schedules", "Fear", "Standing"; structures-and-ticking.md "/tickingarea"):
//! a raid won with an armorer's prices before and after (`raid`), one lost (`raidlost`), one kept
//! across a restart (`raidkeep`, then `raidkeep2` with KEEP_WORLD=1), a village's golems and a
//! player's standing (`golems`, `golems2` kept), a night and a zombie (`village`), an ominous
//! bottle (`ominous`) and a command block in a far chunk (`spawnchunks`). Every place is absolute
//! on the flat ground (stand at y -60).

pub const VILLAGE_SHOWN: &[&str] = &[
    "MobEffect", "BossEvent", "AddEntity", "RemoveEntity", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "UpdateTrade", "MobArmorEquipment", "MobEquipment",
    "UpdateBlock", "SetEntityLink", "Text", "UpdateAttributes",
];

const UNSKILLED: [&str; 2] = ["summon villager -4.5 -60 4.5 0 0 minecraft:become_unskilled", "summon villager -2.5 -60 4.5 0 0 minecraft:become_unskilled"];
/// The armorer is summoned last, so `!trade villager_v2` takes it.
const ARMORER: [&str; 2] = ["setblock 6 -60 4 blast_furnace", "summon villager 4.5 -60 4.5 0 0 minecraft:become_armorer"];
const TRADE: [&str; 4] = ["tp @s 4.5 -60 2.5 facing 4.5 -59 4.5", "", "!trade villager_v2 0 0", ""];
const RAIDERS_NEAR: &str = "testfor @e[family=monster,x=0,y=-60,z=6,r=16]";

fn lines<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn wait(out: &mut Vec<String>, seconds: usize) {
    out.extend(std::iter::repeat_n(String::new(), seconds));
}

/// `line`, then `gap` seconds, `times` over.
fn every(out: &mut Vec<String>, line: &str, gap: usize, times: usize) {
    for _ in 0..times {
        out.push(line.into());
        wait(out, gap);
    }
}

/// A cleared square of grass about the origin with three beds and a bell, the actor in survival
/// beside it and unhurt by anything; then `summon`, and ten seconds for the villagers to claim.
fn village(summon: &[&str]) -> Vec<String> {
    let mut out = lines([
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule doweathercycle false", "weather clear", "time set 1000", "difficulty easy",
        "tp @a 0.5 -60 0.5", "fill -24 -61 -24 24 -61 24 grass_block", "fill -24 -60 -24 24 -52 24 air", "",
        "setblock -6 -60 8 bed [\"direction\"=0]", "setblock -4 -60 8 bed [\"direction\"=0]", "setblock -2 -60 8 bed [\"direction\"=0]", "setblock 4 -60 8 bell",
        "gamemode survival @s", "clear @s", "effect @s resistance 1000 4 true", "effect @s saturation 1000 0 true", "",
    ]);
    out.extend(summon.iter().map(|line| line.to_string()));
    wait(&mut out, 10);
    out
}

fn villagers_with_armorer() -> Vec<&'static str> {
    UNSKILLED.iter().chain(&ARMORER).copied().collect()
}

/// Bad omen II taken 200 out (no raid omen), then in the village; the raid on easy fought with
/// the raiders left 12 s near the village, then killed every 3 s until it is won; the watcher,
/// 80 out and in survival, shows whether a hero's reach takes it; the armorer's offers before and
/// after. `kill` is how the raiders die: by `/kill`, or (`raidhero`) by damage the actor deals.
fn raid(kill: &str) -> Vec<String> {
    let mut out = village(&villagers_with_armorer());
    out.extend(lines(["effect @e[type=villager_v2] resistance 1000 4 true", "effect @e[type=villager_v2] slowness 1000 255 true", "give @s coal 64", "give @s emerald 64"]));
    out.extend(lines(TRADE));
    out.extend(lines(["tp CmdWatcher 80.5 -60 0.5", "gamemode survival CmdWatcher", "effect CmdWatcher resistance 1000 4 true", "tp @s 200.5 -60 0.5", "", "effect @s bad_omen 600 1"]));
    wait(&mut out, 6);
    out.push("tp @s 0.5 -60 0.5".into());
    wait(&mut out, 40);
    every(&mut out, RAIDERS_NEAR, 1, 12);
    every(&mut out, kill, 2, 40);
    out.extend(lines(TRADE));
    out.extend(lines(["effect @s clear", "effect CmdWatcher clear", "gamemode creative @s", "gamemode creative CmdWatcher", ""]));
    out
}

/// Bad omen I in the village until the first wave comes and five seconds after; then the
/// villagers are killed and the raiders watched for 45 s.
fn raid_lost() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.push("effect @s bad_omen 600 0".into());
    wait(&mut out, 50);
    every(&mut out, RAIDERS_NEAR, 1, 5);
    out.push("kill @e[type=villager_v2]".into());
    every(&mut out, RAIDERS_NEAR, 2, 15);
    out.extend(lines(["kill @e[family=monster]", "effect @s clear", "gamemode creative @s", ""]));
    out
}

/// As `raidlost` up to the first wave; the server stops with the raid on.
fn raid_keep() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.push("effect @s bad_omen 600 0".into());
    wait(&mut out, 50);
    every(&mut out, RAIDERS_NEAR, 1, 5);
    out
}

/// On `raidkeep`'s world: what comes back, then the villagers killed.
fn raid_keep2() -> Vec<String> {
    let mut out = Vec::new();
    every(&mut out, RAIDERS_NEAR, 2, 8);
    out.push("kill @e[type=villager_v2]".into());
    every(&mut out, RAIDERS_NEAR, 2, 15);
    out.extend(lines(["kill @e[family=monster]", "effect @s clear", "gamemode creative @s", ""]));
    out
}

/// A village golem, a player's golem and a summoned one held still; a baby villager hit ten times,
/// then a grown one ten times: the hit after which each golem takes the actor (`TargetEid`).
fn golems() -> Vec<String> {
    let mut out = village(&UNSKILLED);
    out.extend(lines([
        "summon iron_golem 10.5 -60 -6.5 0 0 minecraft:from_village", "summon iron_golem -9.5 -60 -6.5 0 0 minecraft:from_player", "summon iron_golem 0.5 -60 -12.5",
        "effect @e[type=iron_golem] slowness 1000 255 true", "summon villager 2.5 -60 3.5 0 0 minecraft:entity_born", "effect @e[type=villager_v2] slowness 1000 255 true",
    ]));
    wait(&mut out, 8);
    // Each hit knocks the villager off, so the actor steps up to the tagged one again first.
    out.push("tag @e[type=villager_v2,x=2.5,y=-60,z=3.5,r=1] add hit".into());
    for _ in 0..10 {
        out.extend(lines(["execute at @e[tag=hit] run tp @s ~ ~ ~-1.5 facing ~ ~0.5 ~", "!hit villager_v2", ""]));
    }
    out.extend(lines(["tag @e[tag=hit] remove hit", "summon villager 10.5 -60 3.5 0 0 minecraft:become_unskilled", "effect @e[type=villager_v2] slowness 1000 255 true", "tag @e[type=villager_v2,x=10.5,y=-60,z=3.5,r=1] add hit"]));
    for _ in 0..10 {
        out.extend(lines(["execute at @e[tag=hit] run tp @s ~ ~ ~-1.5 facing ~ ~1 ~", "!hit villager_v2", ""]));
    }
    wait(&mut out, 5);
    out
}

/// On `golems`'s world: whether the golems still take the actor.
fn golems2() -> Vec<String> {
    let mut out = lines(["testfor @e[type=iron_golem]", "tp @s 0.5 -60 0.5"]);
    wait(&mut out, 15);
    out
}

/// Evening to night with the clock running: who walks home and sleeps (flags, beds'
/// `occupied_bit`), counted at the beds every 5 s; morning; then a zombie caged among them.
fn village_night() -> Vec<String> {
    let mut summon = villagers_with_armorer();
    summon.push("summon villager 0.5 -60 4.5 0 0 minecraft:entity_born");
    let mut out = village(&summon);
    out.extend(lines(["tp @s 10.5 -60 -10.5", "gamerule dodaylightcycle true", "time set 11400"]));
    every(&mut out, "testfor @e[type=villager_v2,x=-4,y=-60,z=9,r=3]", 4, 10);
    out.push("time set 23900".into());
    every(&mut out, "testfor @e[type=villager_v2,x=-4,y=-60,z=9,r=3]", 2, 8);
    out.extend(lines(["gamerule dodaylightcycle false", "time set 1000", ""]));
    every(&mut out, "testfor @e[type=villager_v2,x=0,y=-60,z=2,r=8]", 1, 3);
    out.extend(lines(["fill -1 -60 1 1 -58 3 glass", "fill 0 -60 2 0 -59 2 air", "summon zombie 0.5 -60 2.5"]));
    every(&mut out, "testfor @e[type=villager_v2,x=0,y=-60,z=2,r=8]", 1, 12);
    out.extend(lines(["kill @e[type=zombie]", "effect @s clear", "gamemode creative @s", ""]));
    out
}

/// An ominous bottle of data 2 drunk away from any village.
fn ominous() -> Vec<String> {
    let mut out = lines(["kill @e[type=!player]", "tp @s 0.5 -60 0.5", "gamemode survival @s", "effect @s clear", "clear @s", "give @s ominous_bottle 1 2", "", "!eat"]);
    wait(&mut out, 4);
    out.extend(lines(["effect @s clear", "gamemode creative @s", ""]));
    out
}

/// A repeating command block counting ticks at the origin, while both players stand by it and
/// while they are 3000 out, without and then with a ticking area over it.
fn spawn_chunks() -> Vec<String> {
    const COUNT: &str = "scoreboard players list clock";
    let mut out = lines([
        "kill @e[type=!player]", "gamerule commandblocksenabled true", "gamemode creative @s", "tp @a 0.5 -60 0.5", "fill -2 -60 -2 4 -58 4 air", "scoreboard objectives add t dummy",
        "setblock 2 -60 0 repeating_command_block", "", "!cmdblock 2 0 0 1 0 0 0 0 0 scoreboard players add clock t 1", "", "",
    ]);
    for kept in [false, true] {
        if kept {
            out.extend(lines(["tickingarea add circle 0 -60 0 1 kept", ""]));
        }
        out.extend(lines([COUNT, "", COUNT, "tp @a 3000.5 -60 0.5", "", COUNT]));
        every(&mut out, COUNT, 5, 3);
        out.extend(lines(["tp @a 0.5 -60 0.5", "", COUNT, "", COUNT]));
    }
    out.extend(lines(["tickingarea remove kept", "setblock 2 -60 0 air", "scoreboard objectives remove t", ""]));
    out
}

/// The lines and shown packets of the drill `name`, if it is one of these.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "raid" => (raid("kill @e[family=monster]"), VILLAGE_SHOWN),
        "raidhero" => (raid("damage @e[family=monster] 200 entity_attack entity CmdActor"), VILLAGE_SHOWN),
        "raidlost" => (raid_lost(), VILLAGE_SHOWN),
        "raidkeep" => (raid_keep(), VILLAGE_SHOWN),
        "raidkeep2" => (raid_keep2(), VILLAGE_SHOWN),
        "golems" => (golems(), VILLAGE_SHOWN),
        "golems2" => (golems2(), VILLAGE_SHOWN),
        "village" => (village_night(), VILLAGE_SHOWN),
        "ominous" => (ominous(), VILLAGE_SHOWN),
        "spawnchunks" => (spawn_chunks(), &["CommandOutput"]),
        _ => return None,
    })
}
