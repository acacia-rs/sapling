//! The fourteenth wave's group E drills (research/parity/FOURTEENTH-BRIEF.md): water mobs, at midnight on
//! easy, in a stone-walled pool at x/z -6..6, y -26..-20 open to the air, with a walled stone tray of land
//! north of it (z 10..15) and a pool capped with stone south of it (z -15..-10, water to the cap).
//! `f14eguard`: a held guardian hit by the survival actor, then let go on its prey and on nobody.
//! `f14esquid`: a held squid and glow squid hit, each watched after; a squid on the tray.
//! `f14edolphin`: three dolphins, one hit and its prey then gone; one on the tray drying out; one under
//! the cap running out of breath. `f14efish`: a cod and a salmon on the tray; a frog in the pool.

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn start() -> Vec<String> {
    owned([
        "kill @e[type=!player]",
        "gamerule domobspawning false",
        "gamerule dodaylightcycle false",
        "gamerule falldamage false",
        "weather clear",
        "time set 18000",
        "difficulty easy",
        "gamemode creative @a",
        "effect @a clear",
        "fill -8 -27 -17 8 -14 17 air",
        "fill -7 -27 -7 7 -20 7 stone",
        "fill -6 -26 -6 6 -20 6 water",
        "fill -7 -20 9 7 -18 16 stone",
        "fill -6 -19 10 6 -18 15 air",
        "fill -7 -27 -16 7 -18 -9 stone",
        "fill -6 -26 -15 6 -19 -10 water",
        "tp CmdWatcher 7.5 -19 0.5",
        "tp @s 0.5 -26 0.5 0 0",
        "",
    ])
}

fn end(mut out: Vec<String>) -> Vec<String> {
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @a", "effect @a clear", "tp @s 7.5 -19 2.5", "fill -8 -27 -17 8 -14 17 air", ""]));
    out
}

fn prey() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["gamemode survival @s", "effect @s water_breathing 1000 0 true", "effect @s resistance 1000 255 true"]));
    out
}

fn held(name: &str) -> String {
    format!("effect @e[name={name}] slowness 1000 255 true")
}

/// Held and hit three times, the actor healed between; let go a while with the actor as its prey at
/// the pool's far side, then a while with the actor in creative.
fn guardian() -> Vec<String> {
    let mut out = prey();
    out.extend(["summon guardian g 0.5 -25 2.5".into(), held("g"), String::new(), "tp @s 0.5 -26 0.8 0 0".into()]);
    for _ in 0..3 {
        out.extend(owned(["!hit guardian", "effect @s instant_health 1 4 true", ""]));
    }
    out.extend(owned(["effect @e[name=g] clear", "tp @s 5.5 -26 5.5", "!hit guardian"]));
    for _ in 0..15 {
        out.extend(owned(["effect @s instant_health 1 4 true"]));
    }
    out.extend(owned(["tp @s -5.5 -26 -5.5"]));
    for _ in 0..10 {
        out.extend(owned(["effect @s instant_health 1 4 true"]));
    }
    out.extend(owned(["gamemode creative @s"]));
    wait(&mut out, 10);
    end(out)
}

/// `f14eguard`'s hit landed as the guardian moved, and the bot's hits here fell short of it: three
/// `/damage` hits as from the actor on one aiming at it, each after a second of aiming, then one on
/// it swimming free.
fn guardian_still() -> Vec<String> {
    let mut out = prey();
    out.extend(owned(["summon guardian g 0.5 -25 3.7", "tp @s 0.5 -26 0.5 0 0"]));
    for _ in 0..3 {
        out.extend(owned(["tp @e[name=g] 0.5 -25 3.7", "tp @s 0.5 -26 0.5 0 0", "", "damage @e[name=g] 1 entity_attack entity @s", "effect @s instant_health 1 4 true", "", "", "", ""]));
    }
    out.extend(owned(["effect @e[name=g] speed 1000 4 true", "tp @s 5.5 -26 5.5", "damage @e[name=g] 1 entity_attack entity @s", "", ""]));
    end(out)
}

/// A held squid hit, let go at once and watched; the same for a glow squid; a squid on the tray.
fn squid() -> Vec<String> {
    let mut out = prey();
    for (kind, name) in [("squid", "s"), ("glow_squid", "t")] {
        out.extend([format!("summon {kind} {name} 0.5 -24 2.5"), held(name), String::new(), "tp @s 0.5 -26 1 0 0".into(), format!("!hit {kind}"), format!("effect @e[name={name}] clear")]);
        wait(&mut out, 10);
        out.extend(owned(["kill @e[type=!player]", ""]));
    }
    out.extend(owned(["gamemode creative @s", "tp @s 7.5 -19 2.5", "summon squid o 0.5 -19 12.5"]));
    wait(&mut out, 15);
    end(out)
}

/// Three dolphins in the pool, the last held and hit, the actor healed; its prey goes creative and out
/// after 6 s. A dolphin on the tray all the while, and one under the cap.
fn dolphin() -> Vec<String> {
    let mut out = prey();
    out.extend(owned(["summon dolphin l 0.5 -19 12.5", "summon dolphin b 0.5 -22 -12.5", "summon dolphin p -4.5 -24 -4.5", "summon dolphin q 4.5 -24 -4.5", "summon dolphin d 0.5 -24 2.5"]));
    out.extend([held("d"), String::new(), "tp @s 0.5 -26 1 0 0".into(), "!hit dolphin".into(), "effect @e[name=d] clear".into()]);
    for _ in 0..6 {
        out.push("effect @s instant_health 1 4 true".into());
    }
    out.extend(owned(["gamemode creative @s", "effect @s clear", "tp @s 7.5 -19 2.5"]));
    wait(&mut out, 300);
    end(out)
}

/// `f14edolphin`'s hit missed (the dolphin swam off): three dolphins in the pool, the last hurt by
/// `/damage` as from the actor; the actor healed 10 s, then creative and out for 40 s.
fn dolphin_anger() -> Vec<String> {
    let mut out = prey();
    out.extend(owned(["summon dolphin p -4.5 -24 -4.5", "summon dolphin q 4.5 -24 -4.5", "summon dolphin d 0.5 -24 2.5", "", "damage @e[name=d] 1 entity_attack entity @s"]));
    for _ in 0..10 {
        out.push("effect @s instant_health 1 4 true".into());
    }
    out.extend(owned(["gamemode creative @s", "effect @s clear", "tp @s 7.5 -19 2.5"]));
    wait(&mut out, 40);
    end(out)
}

/// A cod and a salmon on the tray; a frog at the pool's floor.
fn fish() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["tp @s 7.5 -19 2.5", "summon cod c -2.5 -19 12.5", "summon salmon m 3.5 -19 12.5", "summon frog f 0.5 -26 0.5"]));
    wait(&mut out, 60);
    end(out)
}

const SHOWN: &[&str] = &[
    "CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "EntityEvent", "SetEntityMotion", "LevelSoundEvent", "LevelEvent", "UpdateAttributes", "MobEffect",
    "Animate", "SpawnParticleEffect",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "f14eguard" => (guardian(), SHOWN),
        "f14eguard2" => (guardian_still(), SHOWN),
        "f14esquid" => (squid(), SHOWN),
        "f14edolphin" => (dolphin(), SHOWN),
        "f14edolphin2" => (dolphin_anger(), SHOWN),
        "f14efish" => (fish(), SHOWN),
        _ => return None,
    })
}
