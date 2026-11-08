//! The thirteenth wave's group BD drills (research/parity/THIRTEENTH-BRIEF.md). On the flat ground (stand at
//! y -60): seven named sheep about the actor, one killed and replaced, hit by `/event` and `/effect` with every
//! sheep, the nearest three, and changes told at the tick's end, to see what order they are told in; then
//! each omen effect given by `/effect` and by event to the actor and to a sheep, for their particles and
//! sounds (`bd2event`); sheep over three chunks, one moved between them, and a villager's omen sound
//! (`bd2order`). On the drill floor at midnight: a zombie on a shelf three up, the actor below, five
//! times 8 s, to see whether BDS's always comes off its spot towards the actor (`bd2shelf`).

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// Where each named sheep stands, in the order they are summoned (`c` killed after `f`, `g` after that).
const SHEEP: [(&str, &str); 6] = [("a", "3.5 -60 0.5"), ("b", "20.5 -60 4.5"), ("c", "2.5 -60 6.5"), ("d", "-10.5 -60 3.5"), ("e", "1.5 -60 2.5"), ("f", "9.5 -60 9.5")];

fn events() -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 1000", "tp @a 0.5 -60 0.5", ""]);
    out.extend(SHEEP.map(|(name, at)| format!("summon sheep {at} 0 0 minecraft:entity_spawned {name}")));
    out.extend(owned([
        "effect @e[type=sheep] slowness 1000 255 true", "kill @e[name=c]", "summon sheep 5.5 -60 -3.5 0 0 minecraft:entity_spawned g", "effect @e[type=sheep] slowness 1000 255 true", "", "",
        "event entity @e[type=sheep] wololo", "",
        "event entity @e[type=sheep] minecraft:on_sheared", "",
        "event entity @e[type=sheep,c=3] minecraft:on_eat_block", "",
        "event entity @e[type=sheep] minecraft:on_eat_block", "",
        "effect @e[type=sheep] speed 5 0", "",
        "effect @e[type=sheep] invisibility 5 0 true", "", "",
        "effect @s bad_omen 30 0", "",
        "effect @s bad_omen 30 1", "",
        "effect @s clear", "",
        "effect @s raid_omen 30 0", "", "effect @s clear", "",
        "effect @s trial_omen 30 0", "", "effect @s clear", "",
        "effect @s hero_of_the_village 30 0", "", "effect @s clear", "",
        "effect @s bad_omen 30 0 true", "",
        "event entity @s minecraft:gain_raid_omen", "", "",
        "effect @s clear", "",
        "effect @e[name=a] bad_omen 30 0", "", "effect @e[name=a] raid_omen 30 0", "", "effect @e[name=a] trial_omen 30 0", "", "effect @e[name=a] clear", "",
        "kill @e[type=!player]", "",
    ]));
    wait(&mut out, 2);
    out
}

/// Five sheep over three chunks (summoned in chunks A, B, A, C, B), told of at the tick's end before and
/// after `s1` is moved into C; `@e` with the watcher 30 off; a held villager's omen sound.
fn orders() -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 1000", "tp @a 0.5 -60 0.5", ""]);
    let sheep = [("s1", "2.5 -60 2.5"), ("s2", "20.5 -60 2.5"), ("s3", "4.5 -60 5.5"), ("s4", "2.5 -60 -6.5"), ("s5", "22.5 -60 6.5")];
    out.extend(sheep.map(|(name, at)| format!("summon sheep {at} 0 0 minecraft:entity_spawned {name}")));
    out.extend(owned([
        "effect @e[type=sheep] slowness 1000 255 true", "", "",
        "event entity @e[type=sheep] wololo", "",
        "event entity @e[type=sheep] minecraft:on_sheared", "",
        "tp @e[name=s1] 2.5 -60 -9.5", "", "",
        "event entity @e[type=sheep] minecraft:on_eat_block", "",
        "event entity @e[type=sheep] minecraft:on_sheared", "",
        "tp CmdWatcher 30.5 -60 0.5", "", "",
        "effect @e speed 3 0 true", "", "",
        "summon villager_v2 -3.5 -60 0.5 0 0 minecraft:spawn_farmer v", "effect @e[name=v] slowness 1000 255 true", "", "",
        "effect @e[name=v] bad_omen 30 0", "", "effect @e[name=v] trial_omen 30 0", "",
        "kill @e[type=!player]", "",
    ]));
    wait(&mut out, 2);
    out
}

/// Where a mob's omen is heard: a villager, a zombie, a cow, a baby cow, a pig and a chicken, held, each
/// given bad omen, their moves shown.
fn mouths() -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 1000", "tp @a 0.5 -60 0.5", ""]);
    let mobs = [
        ("villager", "-3.5 -60 0.5", "minecraft:spawn_farmer", "v"), ("zombie", "4.5 -60 0.5", "minecraft:entity_spawned", "z"), ("cow", "0.5 -60 4.5", "minecraft:entity_spawned", "c"),
        ("cow", "0.5 -60 -3.5", "minecraft:entity_born", "b"), ("pig", "4.5 -60 4.5", "minecraft:entity_spawned", "p"), ("chicken", "-3.5 -60 -3.5", "minecraft:entity_spawned", "k"),
    ];
    for (kind, at, event, name) in mobs {
        out.extend([format!("summon {kind} {at} 0 0 {event} {name}"), format!("effect @e[name={name}] slowness 1000 255 true")]);
    }
    out.extend(owned(["", ""]));
    for name in ["v", "z", "c", "b", "p", "k"] {
        out.extend([format!("effect @e[name={name}] bad_omen 30 0"), String::new()]);
    }
    out.extend(owned(["kill @e[type=!player]", ""]));
    wait(&mut out, 2);
    out
}

fn shelves() -> Vec<String> {
    let mut out = super::warden::start("midnight");
    let reset = ["kill @e[type=!player]", "fill -20 -26 -20 20 -16 0 air", "fill -20 -26 0 20 -16 20 air", "fill -20 -26 -20 20 -24 -6 stone", "tp @s 0.5 -26 0.5"];
    for _ in 0..5 {
        out.extend(reset.map(String::from));
        out.push("summon zombie 0.5 -23 -11.5".into());
        super::warden::healed(&mut out, 8);
    }
    out.extend(owned(["kill @e[type=!player]", "fill -20 -26 -20 20 -16 0 air", "tp @s 0.5 -26 0.5", "gamemode creative @s", "effect @s clear"]));
    out
}

const EVENT_SHOWN: &[&str] = &["CommandOutput", "SetEntityData", "MobEffect", "AddEntity", "RemoveEntity", "EntityEvent", "LevelEvent", "LevelSoundEvent"];
const MOUTH_SHOWN: &[&str] = &["CommandOutput", "AddEntity", "MoveEntityDelta", "MoveEntityAbsolute", "LevelSoundEvent"];
const SHELF_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "SetEntityData", "CommandOutput", "EntityEvent"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "bd2event" => (events(), EVENT_SHOWN),
        "bd2shelf" => (shelves(), SHELF_SHOWN),
        "bd2order" => (orders(), EVENT_SHOWN),
        "bd2mouth" => (mouths(), MOUTH_SHOWN),
        _ => return None,
    })
}
