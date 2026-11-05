//! The thirteenth wave's group B drills (research/parity/THIRTEENTH-BRIEF.md). `h3herds`, on the
//! flat world (sapling `SAPLING_DRILL_WORLD=open`), summons the kinds whose spawn rules' herds fire
//! events (horse coats, tropical fish looks, riders) with and without them (docs/research/husbandry.md
//! "Herds"). `h3keep`, on a default world of seed 20261003 (`SAPLING_DRILL_WORLD=generated
//! SAPLING_DRILL_SEED=20261003`), lets natural mobs come by the swamp hut at midnight and summons
//! one, so the world folder saved on stop shows their `Persistent` and `NaturalSpawn`
//! (docs/research/mob-spawning.md "Despawning").

use crate::creatures::waits;

const SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "SetEntityLink", "SetEntityData", "MobEquipment", "MobArmorEquipment", "CommandOutput"];
const SEEN: &[&str] = &["AddEntity", "RemoveEntity", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "h3herds" => (herds(), SHOWN),
        "h3keep" => (keep(), SEEN),
        _ => return None,
    })
}

fn herds() -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 5);
    lines.extend(["difficulty normal", "gamerule domobspawning false", "gamemode creative @s", "gamemode creative CmdWatcher", "time set 6000"].map(String::from));
    let summons = [
        ("horse", ""), ("horse", ""), ("horse", ""), ("horse", ""), ("horse", " 0 0 minecraft:make_black"), ("horse", " 0 0 minecraft:make_darkbrown"),
        ("tropicalfish", ""), ("tropicalfish", ""), ("tropicalfish", ""), ("tropicalfish", " 0 0 minecraft:become_clownfish"), ("tropicalfish", " 0 0 minecraft:become_red_snapper"),
        ("zombie_horse", " 0 0 minecraft:spawn_adult_with_rider"), ("camel_husk", " 0 0 minecraft:spawn_with_rider"), ("drowned", " 0 0 minecraft:as_rider"),
    ];
    for (n, (kind, event)) in summons.iter().enumerate() {
        lines.push(format!("summon {kind} {}.5 -60 6.5{event}", n as i32 * 3 - 20));
        waits(&mut lines, 1);
    }
    waits(&mut lines, 5);
    lines.extend(["testfor @e[type=zombie]", "testfor @e[type=husk]", "testfor @e[type=parched]", "testfor @e[type=zombie_nautilus]"].map(String::from));
    lines.push("kill @e[type=!player]".into());
    lines
}

fn keep() -> Vec<String> {
    let mut lines = Vec::new();
    waits(&mut lines, 5);
    lines.extend(["difficulty normal", "gamerule domobspawning true", "gamerule dodaylightcycle false", "time set 18000", "gamemode creative @s", "gamemode creative CmdWatcher"].map(String::from));
    for bot in ["@s", "CmdWatcher"] {
        lines.push(format!("tp {bot} 3843 91 -1795"));
    }
    for _ in 0..15 {
        lines.push("fill 3841 90 -1797 3845 90 -1793 barrier".into());
    }
    lines.push("summon zombie 3845 91 -1795".into());
    waits(&mut lines, 90);
    lines.push("testfor @e[type=!player]".into());
    lines
}
