//! The fourteenth wave's group C drills (research/parity/FOURTEENTH-BRIEF.md): BG's open lines. On the
//! flat ground at midnight. `f14csneak`: the actor sneaks and stands with empty hands, creative and
//! survival, then with a shield in its off hand. `f14cthrow`: thrown down at its feet beside a slowed
//! cow: a splash potion of invisibility, a lingering one, a snowball, an egg, a bottle o' enchanting and
//! a pearl. `f14cpick`: a zombie with a slowed villager and the survival watcher both in reach, each
//! layout three times: whom it takes first.

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn start() -> Vec<String> {
    owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 18000", "difficulty easy", "gamemode creative @a", "effect @a clear", "clear @a", "tp @s 0.5 -60 0.5", "tp CmdWatcher 3.5 -60 0.5", ""])
}

fn sneak() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["!sneak", "", "", "!unsneak", "", "", "!sneak", "", "!unsneak", "", "gamemode survival @s", "", "!sneak", "", "", "!unsneak", "", ""]));
    out.extend(owned(["replaceitem entity @s slot.weapon.offhand 0 shield", "", "!sneak", "", "", "!unsneak", "", "", "clear @s", "", "!sneak", "", "!unsneak", "", "gamemode creative @s", ""]));
    out
}

fn throw() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["summon cow 2 -60 0.5", "effect @e[type=cow] slowness 1000 255 true", "", "!look 0.01 -1 0", ""]));
    for item in ["splash_potion 1 7", "lingering_potion 1 7", "snowball", "egg", "experience_bottle", "ender_pearl"] {
        out.push(format!("replaceitem entity @s slot.weapon.mainhand 0 {item}"));
        out.extend(owned(["", "!use", "", ""]));
        wait(&mut out, 2);
        out.extend(owned(["effect @a clear", "effect @e[type=cow] invisibility 0", "kill @e[type=item]", "kill @e[type=xp_orb]", "kill @e[type=chicken]", "kill @e[type=area_effect_cloud]", "tp @s 0.5 -60 0.5", ""]));
    }
    out
}

/// The zombie at x 0.5 z 0.5; the villager and the watcher `near` and `far` blocks off, either way.
fn pick() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["tp @s 0.5 -60 -20.5", "gamemode survival CmdWatcher", "effect CmdWatcher resistance 1000 255 true", ""]));
    for (villager, watcher) in [(5.0, 15.0), (15.0, 5.0), (7.0, 18.0)] {
        for _ in 0..3 {
            out.push(format!("tp CmdWatcher {} -60 0.5", 0.5 - watcher));
            out.push(format!("summon villager v {} -60 0.5", 0.5 + villager));
            out.extend(owned(["effect @e[type=villager] slowness 1000 255 true", "summon zombie 0.5 -60 0.5", ""]));
            wait(&mut out, 3);
            out.extend(owned(["kill @e[type=zombie]", "kill @e[type=villager]", "kill @e[type=item]", ""]));
        }
    }
    out.extend(owned(["gamemode creative CmdWatcher", "effect @a clear", ""]));
    out
}

const SHOWN: &[&str] = &["CommandOutput", "SetEntityData", "MobEffect", "AddEntity", "AddItemEntity", "AddPlayer", "RemoveEntity", "LevelSoundEvent", "LevelEvent", "EntityEvent", "MobEquipment"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "f14csneak" => (sneak(), SHOWN),
        "f14cthrow" => (throw(), SHOWN),
        "f14cpick" => (pick(), SHOWN),
        _ => return None,
    })
}
