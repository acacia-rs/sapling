//! The fourteenth wave's group J drills (research/parity/FOURTEENTH-BRIEF.md), on the flat ground at
//! midnight, the actor creative 13 off unless said. `j3knock`: a zombie at a held villager, an iron
//! golem at a held zombie, a wolf set on a held zombie by `/damage`, then a zombie at the survival actor.
//! `j3slime`: a medium slime in a lava pool, a large slime by a held, weakened iron golem. `j3omen`: a
//! wind-charged zombie dies among held cows and dropped items 2 to 8 off; oozing medium and large slimes
//! and an oozing zombie die. `j3creak`: spawning on, the survival actor hits a natural heart's creaking
//! three times, then the creaking is put 20, 30, 40 and 50 from its heart. `j3ride`: a zombie horse
//! with its rider and a baby zombie put on a chicken near the survival actor; a pig with a zombie on it
//! and a pig alone dropped into water. `j3sheep`: 64 summoned sheep; held lambs on stone ringed by
//! grass blocks, short grass beside two.

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn start(difficulty: &str) -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "weather clear", "time set 18000"]);
    out.extend([format!("difficulty {difficulty}"), "gamemode creative @a".into(), "effect @a clear".into(), "clear @s".into(), "fill -12 -60 -12 12 -54 12 air".into(), "fill -12 -63 -12 12 -62 12 dirt".into(), "fill -12 -61 -12 12 -61 12 grass".into()]);
    out.extend(owned(["tp CmdWatcher 0.5 -60 -40.5", "tp @s 0.5 -60 -12.5", ""]));
    out
}

fn held(name: &str) -> String {
    format!("effect @e[name={name}] slowness 1000 255 true")
}

fn survival(out: &mut Vec<String>) {
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 3 true", "effect @s instant_health 1 4 true", "tp @s 0.5 -60 0.5 0 0"]));
}

fn creative(out: &mut Vec<String>) {
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", "tp @s 0.5 -60 -12.5", ""]));
}

fn knocks() -> Vec<String> {
    let mut out = start("easy");
    out.extend(["summon villager v 0.5 -60 2.5".to_string(), held("v"), "effect @e[name=v] resistance 1000 3 true".into(), "summon zombie 0.5 -60 6.5 180 0".into()]);
    wait(&mut out, 8);
    out.extend(owned(["kill @e[type=!player]", ""]));
    out.extend(["summon zombie z 0.5 -60 2.5".to_string(), held("z"), "effect @e[name=z] resistance 1000 3 true".into(), "summon iron_golem 0.5 -60 6.5 180 0".into()]);
    wait(&mut out, 8);
    out.extend(owned(["kill @e[type=!player]", ""]));
    out.extend(["summon zombie z 0.5 -60 2.5".to_string(), held("z"), "effect @e[name=z] resistance 1000 3 true".into(), "summon wolf 0.5 -60 5.5 180 0".into(), String::new()]);
    out.push("damage @e[type=wolf] 1 entity_attack entity @e[name=z]".into());
    wait(&mut out, 8);
    out.extend(owned(["kill @e[type=!player]", ""]));
    survival(&mut out);
    out.push("summon zombie 0.5 -60 4.5 180 0".into());
    wait(&mut out, 6);
    creative(&mut out);
    out
}

fn slimes() -> Vec<String> {
    let mut out = start("easy");
    out.extend(owned(["fill 3 -63 -3 9 -61 3 stone", "fill 4 -62 -2 8 -61 2 lava", "summon slime 6.5 -61 0.5 0 0 spawn_medium", "effect @e[type=slime] resistance 1000 255 true", "effect @e[type=slime] fire_resistance 1000 0 true"]));
    wait(&mut out, 15);
    out.extend(owned(["kill @e[type=!player]", "fill 3 -63 -3 9 -62 3 dirt", "fill 3 -61 -3 9 -61 3 grass", ""]));
    out.extend(["summon iron_golem g -4.5 -60 0.5".to_string(), held("g"), "effect @e[name=g] weakness 1000 255 true".into(), "summon slime -4.5 -60 4.5 180 0 spawn_large".into(), "effect @e[type=slime] resistance 1000 255 true".into()]);
    wait(&mut out, 20);
    out.extend(owned(["kill @e[type=!player]", ""]));
    out
}

fn omens() -> Vec<String> {
    let mut out = start("easy");
    for (name, at) in [("a", "3.5 -60 0.5"), ("b", "-3.5 -60 0.5"), ("c", "0.5 -60 5.5"), ("d", "0.5 -60 -5.5"), ("e", "5.5 -60 5.5"), ("f", "-6.5 -60 -6.5"), ("g", "8.5 -60 -4.5")] {
        out.extend([format!("summon cow {name} {at}"), held(name)]);
    }
    out.extend(owned(["loot spawn 2.5 -60 2.5 loot \"entities/chicken\"", "loot spawn -1.5 -60 -3.5 loot \"entities/chicken\"", "loot spawn 4.5 -60 -2.5 loot \"entities/chicken\""]));
    out.extend(["summon zombie z 0.5 -60 0.5".to_string(), held("z"), "effect @e[name=z] wind_charged 100 0 true".into()]);
    wait(&mut out, 3);
    out.push("kill @e[name=z]".into());
    wait(&mut out, 4);
    out.extend(owned(["kill @e[type=!player]", ""]));
    for (event, at) in [("spawn_medium", "-4.5 -60 4.5"), ("spawn_large", "4.5 -60 4.5")] {
        out.extend([format!("summon slime {at} 0 0 {event}"), "effect @e[type=slime] slowness 1000 255 true".into(), "effect @e[type=slime] oozing 100 0 true".into(), "kill @e[type=slime]".into()]);
        wait(&mut out, 3);
        out.extend(owned(["kill @e[type=!player]", ""]));
    }
    out.extend(["summon zombie o 0.5 -60 4.5".to_string(), held("o"), "effect @e[name=o] oozing 100 0 true".into(), "kill @e[name=o]".into()]);
    wait(&mut out, 3);
    out.extend(owned(["kill @e[type=!player]", ""]));
    out
}

/// What spawns besides the creaking goes each second, the actor kept alive.
fn guarded(out: &mut Vec<String>, seconds: usize) {
    for _ in 0..seconds {
        out.extend(owned(["effect @s instant_health 1 4 true", "kill @e[type=!player,type=!creaking,type=!item]"]));
    }
}

fn creakings() -> Vec<String> {
    let mut out = start("easy");
    out.extend(owned(["gamerule domobspawning true", "setblock 6 -60 0 pale_oak_log", "setblock 6 -59 0 creaking_heart [\"natural\"=true]", "setblock 6 -58 0 pale_oak_log"]));
    out.extend(owned(["setblock 6 -60 1 pale_oak_log", "setblock 6 -59 1 pale_oak_log", "setblock 5 -59 0 pale_oak_log", "setblock 7 -59 0 pale_oak_log"]));
    survival(&mut out);
    guarded(&mut out, 20);
    for _ in 0..3 {
        out.extend(owned(["execute at @e[type=creaking,c=1] run tp @s ~2 ~ ~ facing @e[type=creaking,c=1]", "!hit creaking"]));
        guarded(&mut out, 2);
    }
    out.extend(owned(["gamemode creative @s", "effect @s clear", "tp @s 0.5 -60 -12.5"]));
    for far in [20, 30, 40, 50] {
        out.push(format!("tp @e[type=creaking] 6.5 -60 {far}.5"));
        guarded(&mut out, 4);
    }
    out.extend(owned(["gamerule domobspawning false", "kill @e[type=!player]", "fill 5 -60 0 7 -58 1 air", ""]));
    out
}

fn riders() -> Vec<String> {
    let mut out = start("easy");
    survival(&mut out);
    out.push("summon zombie_horse 0.5 -60 8.5 180 0 minecraft:spawn_adult_with_rider".into());
    wait(&mut out, 12);
    out.extend(owned(["kill @e[type=!player]", "tp @s 0.5 -60 0.5 0 0", "summon chicken c 0.5 -60 8.5", "summon zombie 0.5 -60 8.5 180 0 minecraft:as_baby", "ride @e[type=zombie] start_riding @e[name=c]"]));
    wait(&mut out, 12);
    creative(&mut out);
    out.extend(owned(["fill 3 -63 -3 9 -61 3 water", "summon zombie_horse 6.5 -55 -1.5 0 0 minecraft:spawn_adult_with_rider"]));
    wait(&mut out, 4);
    out.extend(owned(["summon chicken c 6.5 -55 2.5", "summon zombie 6.5 -55 2.5 0 0 minecraft:as_baby", "ride @e[type=zombie,r=2,x=6.5,y=-55,z=2.5] start_riding @e[name=c]"]));
    wait(&mut out, 4);
    out.push("summon zombie_horse 4.5 -55 0.5".into());
    wait(&mut out, 4);
    out.extend(owned(["kill @e[type=!player]", "fill 3 -63 -3 9 -62 3 dirt", "fill 3 -61 -3 9 -61 3 grass", ""]));
    out
}

fn sheep() -> Vec<String> {
    let mut out = start("easy");
    out.push("time set 6000".into());
    for x in 0..8 {
        for z in 0..8 {
            out.push(format!("summon sheep {} -60 {}", x * 3 - 10, z * 3 - 10));
        }
    }
    wait(&mut out, 2);
    out.extend(owned(["kill @e[type=!player]", ""]));
    for (x, z, yaw) in [(-6, 0, 0), (-2, 0, 90), (2, 0, 180), (6, 0, -90), (-6, 6, 0), (2, 6, 0)] {
        out.extend([format!("setblock {x} -61 {z} stone"), format!("summon sheep {x}.5 -60 {z}.5 {yaw} 0 spawn_baby")]);
    }
    out.extend(owned(["effect @e[type=sheep] slowness 1000 255 true", "setblock -6 -60 7 short_grass", "setblock 3 -60 6 short_grass"]));
    wait(&mut out, 40);
    out.extend(owned(["kill @e[type=!player]", "fill -12 -61 -12 12 -61 12 grass", "fill -12 -60 -12 12 -60 12 air", "time set 18000", ""]));
    out
}

const SHOWN: &[&str] = &[
    "CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "EntityEvent", "SetEntityMotion", "LevelSoundEvent", "LevelEvent", "UpdateAttributes", "MobEffect",
    "AddItemEntity", "UpdateBlock", "Animate", "SetEntityLink", "UpdateSubchunkBlocks",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "j3knock" => (knocks(), SHOWN),
        "j3slime" => (slimes(), SHOWN),
        "j3omen" => (omens(), SHOWN),
        "j3creak" => (creakings(), SHOWN),
        "j3ride" => (riders(), SHOWN),
        "j3sheep" => (sheep(), SHOWN),
        _ => return None,
    })
}
