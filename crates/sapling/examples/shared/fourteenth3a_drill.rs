//! The fourteenth wave's group A drills (research/parity/FOURTEENTH-BRIEF.md): mob combat, on the flat
//! ground at midnight. `a3hurt`: a held snow golem throwing at a held skeleton, a creeper and a held
//! pillager; a survival actor hitting a held zombie that hunts a held villager, a player's iron golem
//! and a plain one. `a3fx`: a zombie hitting the survival watcher plain, with strength and with
//! weakness; speed and slowness on it; levitation, slow falling, and a jump onto a step with and
//! without jump boost. `a3hunt`: a piglin and a polar bear on peaceful; a piglin by an actor in a gold
//! helmet and then in none; a zombie walking into the still actor. `a3ender`: an angry enderman held
//! beside the actor, looked at and then not. `a3creeper`: a lit creeper whose prey steps behind a wall;
//! a forced creeper beside an arrow, an orb and falling sand (its summon misspelt: see `a3creeper2`).
//! The `2` and `3` scenes redo what went wrong in the first (each says how).

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn start(difficulty: &str) -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "time set 18000"]);
    out.extend([format!("difficulty {difficulty}"), "gamemode creative @a".into(), "effect @a clear".into(), "fill -8 -60 -8 8 -55 12 air".into(), "tp CmdWatcher 0.5 -60 -14.5".into(), "tp @s 0.5 -60 -12.5".into(), String::new()]);
    out
}

fn held(name: &str) -> String {
    format!("effect @e[name={name}] slowness 1000 255 true")
}

fn hurts() -> Vec<String> {
    let mut out = start("easy");
    for (kind, name, holds) in [("skeleton", "s", true), ("creeper", "c", false), ("pillager", "p", true)] {
        out.extend(["summon snow_golem g 0.5 -60 0.5".into(), held("g"), format!("summon {kind} {name} 4.5 -60 0.5")]);
        if holds {
            out.push(held(name));
        }
        wait(&mut out, 7);
        out.extend(owned(["kill @e[type=!player]", ""]));
    }
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 255 true", "tp @s 0.5 -60 -2 0 0", "summon villager v 0.5 -60 2", "summon zombie z 0.5 -60 0.5"]));
    out.extend([held("v"), held("z"), String::new(), String::new(), "!hit zombie".into()]);
    wait(&mut out, 5);
    out.extend(owned(["kill @e[type=!player]", "tp @s 0.5 -60 -2 0 0", "summon iron_golem 0.5 -60 1 0 0 minecraft:from_player i", ""]));
    out.extend([held("i"), "!hit iron_golem".into()]);
    wait(&mut out, 4);
    out.extend(owned(["kill @e[type=!player]", "tp @s 0.5 -60 -2 0 0", "summon iron_golem j 0.5 -60 1", ""]));
    out.extend([held("j"), "!hit iron_golem".into()]);
    wait(&mut out, 4);
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", ""]));
    out
}

fn effects() -> Vec<String> {
    let mut out = start("normal");
    out.extend(owned(["tp @s 0.5 -60 -20.5", "gamemode survival CmdWatcher", "tp CmdWatcher 0.5 -60 0.5 0 0", "summon zombie z 0.5 -60 2.5"]));
    for effect in ["", "effect @e[name=z] strength 30 0", "effect @e[name=z] weakness 30 0"] {
        out.push(effect.into());
        wait(&mut out, 4);
        out.extend(owned(["effect @e[name=z] clear", "effect CmdWatcher instant_health 1 4", ""]));
    }
    out.extend(owned(["tp CmdWatcher 0.5 -60 -14.5", "gamemode creative CmdWatcher", "effect @e[name=z] speed 10 1", "", "effect @e[name=z] slowness 10 1", "", "effect @e[name=z] clear", "", "kill @e[type=!player]", ""]));
    out.extend(owned(["summon zombie l 0.5 -60 0.5", "effect @e[name=l] slowness 1000 255 true", "effect @e[name=l] levitation 2 1"]));
    wait(&mut out, 6);
    out.extend(owned(["tp @e[name=l] 0.5 -50 0.5", "effect @e[name=l] slow_falling 10 0"]));
    wait(&mut out, 6);
    out.extend(owned(["kill @e[type=!player]", "fill -3 -60 4 3 -60 10 stone", "gamemode survival CmdWatcher", "tp CmdWatcher 0.5 -59 7.5", ""]));
    for boost in ["effect @e[name=b] jump_boost 30 2", ""] {
        out.extend(["summon zombie b 0.5 -60 0.5".to_string(), boost.to_string()]);
        wait(&mut out, 5);
        out.extend(owned(["kill @e[type=!player]", "effect CmdWatcher instant_health 1 4", "tp CmdWatcher 0.5 -59 7.5", ""]));
    }
    out.extend(owned(["fill -3 -60 4 3 -60 10 air", "gamemode creative CmdWatcher", "tp CmdWatcher 0.5 -60 -14.5", ""]));
    out
}

fn hunts() -> Vec<String> {
    let mut out = start("peaceful");
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 255 true", "tp @s 0.5 -60 -2 0 0", "summon piglin q 0.5 -60 2.5"]));
    wait(&mut out, 4);
    out.extend(owned(["kill @e[type=!player]", "tp @s 0.5 -60 -2 0 0", "summon polar_bear r 0.5 -60 0.5", "", "!hit polar_bear"]));
    wait(&mut out, 4);
    out.extend(owned(["kill @e[type=!player]", "difficulty easy", "replaceitem entity @s slot.armor.head 0 golden_helmet", "tp @s 0.5 -60 -2 0 0", "summon piglin q 0.5 -60 4.5"]));
    wait(&mut out, 5);
    out.extend(owned(["replaceitem entity @s slot.armor.head 0 air"]));
    wait(&mut out, 5);
    out.extend(owned(["kill @e[type=!player]", "tp @s 0.5 -60 -2 0 0", "summon zombie z 0.5 -60 4.5"]));
    wait(&mut out, 8);
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", ""]));
    out
}

fn enders() -> Vec<String> {
    let mut out = start("easy");
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 255 true", "tp @s 0.5 -60 0.5 0 0", "summon enderman e 0.5 -60 3", "effect @e[name=e] slowness 1000 255 true", "!look 0 2.6 2.5", "!hit enderman"]));
    for look in ["!look 0 2.6 2.5", "!look 0 1.6 -5"] {
        out.push(look.into());
        for _ in 0..20 {
            out.extend(owned(["tp @e[name=e] 0.5 -60 3", ""]));
        }
    }
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", ""]));
    out
}

fn creepers() -> Vec<String> {
    let mut out = start("easy");
    out.extend(owned(["fill -1 -60 1 1 -58 1 stone", "gamemode survival @s", "effect @s resistance 1000 255 true", "tp @s 2.5 -60 0.5", "summon creeper c 2.5 -60 2.8", "effect @e[name=c] slowness 1000 255 true", "tp @s 0.5 -60 -0.5"]));
    wait(&mut out, 5);
    out.extend(owned(["kill @e[type=!player]", "fill -1 -60 1 1 -58 1 air", "gamemode creative @s", "effect @s clear", "tp @s 0.5 -60 -12.5", ""]));
    out.extend(owned(["summon arrow 2.5 -60 0.5", "summon xp_orb 0.5 -60 2.5", "summon creeper f 0.5 -60 0.5 0 0 minecraft:start_exploding_forced", "setblock 1.5 -55 0.5 sand"]));
    wait(&mut out, 5);
    out.extend(owned(["kill @e[type=!player]", ""]));
    out
}

/// `a3hurt`'s zombie again, held before the villager comes; a zombie hit by the actor while it hunts
/// nothing (the actor creative first, then survival).
fn hurts_again() -> Vec<String> {
    let mut out = start("easy");
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 255 true", "tp @s 0.5 -60 -2 0 0", "summon zombie z 0.5 -60 0.5"]));
    out.extend([held("z"), "summon villager v 0.5 -60 2".into(), held("v"), String::new(), "!hit zombie".into()]);
    wait(&mut out, 5);
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "tp @s 0.5 -60 -2 0 0", "summon zombie y 0.5 -60 0.5"]));
    out.extend([held("y"), String::new(), "!hit zombie".into(), String::new(), "gamemode survival @s".into()]);
    wait(&mut out, 4);
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", ""]));
    out
}

/// A held zombie hitting the survival watcher beside it under strength II, weakness I and weakness II,
/// the watcher healed between; a zombie given slow falling and then put 10 up.
fn effects_again() -> Vec<String> {
    let mut out = start("normal");
    out.extend(owned(["tp @s 0.5 -60 -20.5", "gamemode survival CmdWatcher", "tp CmdWatcher 0.5 -60 0.5 0 0", "summon zombie z 0.5 -60 1.6"]));
    out.push(held("z"));
    for effect in ["strength 30 1", "weakness 30 0", "weakness 30 1", "strength 30 0"] {
        out.extend([format!("effect @e[name=z] {effect}"), "effect CmdWatcher instant_health 1 4".into()]);
        wait(&mut out, 2);
        out.extend(owned(["effect @e[name=z] clear", "effect @e[name=z] slowness 1000 255 true", "effect CmdWatcher instant_health 1 4"]));
    }
    out.extend(owned(["kill @e[type=!player]", "tp CmdWatcher 0.5 -60 -14.5", "gamemode creative CmdWatcher", "summon zombie l 0.5 -60 0.5", "effect @e[name=l] slowness 1000 255 true", "effect @e[name=l] slow_falling 30 0", "tp @e[name=l] 0.5 -50 0.5"]));
    wait(&mut out, 8);
    out.extend(owned(["kill @e[type=!player]", ""]));
    out
}

/// `a3fx2` again without the slowness, which stopped its zombie's hits: an unheld zombie hitting the
/// survival watcher under strength II, weakness I and weakness II, the watcher healed and put back.
fn effects_third() -> Vec<String> {
    let mut out = start("normal");
    out.extend(owned(["tp @s 0.5 -60 -20.5", "gamemode survival CmdWatcher", "tp CmdWatcher 0.5 -60 0.5 0 0", "summon zombie z 0.5 -60 2.5"]));
    for effect in ["strength 30 1", "weakness 30 0", "weakness 30 1"] {
        out.extend([format!("effect @e[name=z] {effect}"), "effect CmdWatcher instant_health 1 4".into(), "tp CmdWatcher 0.5 -60 0.5 0 0".into()]);
        wait(&mut out, 3);
        out.extend(owned(["effect @e[name=z] clear", "tp @e[name=z] 0.5 -60 2.5"]));
    }
    out.extend(owned(["kill @e[type=!player]", "effect CmdWatcher instant_health 1 4", "tp CmdWatcher 0.5 -60 -14.5", "gamemode creative CmdWatcher", ""]));
    out
}

/// A held creeper lit by the actor beside it, the actor then put 2.1 off behind a wall; a forced
/// creeper beside an arrow, an orb and two sands falling onto it.
fn creepers_again() -> Vec<String> {
    let mut out = start("easy");
    out.extend(owned(["fill -1 -60 1 1 -58 1 stone", "summon creeper c 0.5 -60 2.3", "effect @e[name=c] slowness 1000 255 true", "", "gamemode survival @s", "effect @s resistance 1000 255 true", "tp @s 2.4 -60 2.3", "", "tp @s 0.5 -60 0.2"]));
    wait(&mut out, 4);
    out.extend(owned(["tp @s 2.4 -60 2.3"]));
    wait(&mut out, 3);
    out.extend(owned(["kill @e[type=!player]", "fill -1 -60 1 1 -58 1 air", "gamemode creative @s", "effect @s clear", "tp @s 0.5 -60 -12.5", ""]));
    out.extend(owned(["summon arrow 2.5 -60 0.5", "summon xp_orb 0.5 -60 2.5", "", "setblock 1.5 -30 0.5 sand", "setblock -0.5 -20 0.5 sand", "summon creeper 0.5 -60 0.5 0 0 minecraft:start_exploding_forced f"]));
    wait(&mut out, 5);
    out.extend(owned(["kill @e[type=!player]", "fill -8 -60 -8 8 -55 12 air", ""]));
    out
}

/// `a3creeper2`'s wall again, its fuse having burnt out before the actor hid: the actor 2.1 off
/// behind it first, then lit beside it and put straight back.
fn creepers_third() -> Vec<String> {
    let mut out = start("easy");
    out.extend(owned(["fill -1 -60 1 1 -58 1 stone", "summon creeper c 0.5 -60 2.3", "effect @e[name=c] slowness 1000 255 true", "effect @s resistance 1000 255 true", "tp @s 0.5 -60 0.2", "gamemode survival @s"]));
    wait(&mut out, 3);
    out.extend(owned(["tp @s 2.4 -60 2.3", "tp @s 0.5 -60 0.2"]));
    wait(&mut out, 4);
    out.extend(owned(["kill @e[type=!player]", "fill -1 -60 1 1 -58 1 air", "gamemode creative @s", "effect @s clear", "tp @s 0.5 -60 -12.5", ""]));
    out
}

const SHOWN: &[&str] = &[
    "CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "EntityEvent", "SetEntityMotion", "LevelSoundEvent", "LevelEvent", "UpdateAttributes", "MobEffect",
    "AddItemEntity", "SpawnExperienceOrb", "UpdateBlock", "MovePlayer", "Animate",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "a3hurt" => (hurts(), SHOWN),
        "a3fx" => (effects(), SHOWN),
        "a3hunt" => (hunts(), SHOWN),
        "a3ender" => (enders(), SHOWN),
        "a3creeper" => (creepers(), SHOWN),
        "a3hurt2" => (hurts_again(), SHOWN),
        "a3fx2" => (effects_again(), SHOWN),
        "a3creeper2" => (creepers_again(), SHOWN),
        "a3fx3" => (effects_third(), SHOWN),
        "a3creeper3" => (creepers_third(), SHOWN),
        _ => return None,
    })
}
