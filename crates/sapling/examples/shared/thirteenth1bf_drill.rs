//! The thirteenth wave's group BF drills (research/parity/THIRTEENTH-BRIEF.md): mob invisibility. On the
//! flat ground at midnight: six held mobs over four chunks (a sheep, a zombie in iron holding a sword, a
//! skeleton with a helmet and bow, a calf, a farmer, a chicken) made invisible by `/effect` with particles,
//! left to run out; two hidden, one raised, taken off by `invisibility 0` and by `clear`; one seen by a
//! rejoined actor; the actor itself; a hidden farmer killed (`bf1give`). Splash and lingering potions of
//! invisibility thrown among two cows and a zombie (`bf1potion`). A zombie summoned 4, 9 and 15 from the
//! invisible actor, and from an invisible farmer, to see whether it comes (`bf1target`).

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn start() -> Vec<String> {
    owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 18000", "difficulty easy", "gamemode creative @a", "effect @a clear", "tp @a 0.5 -60 0.5", ""])
}

fn gives() -> Vec<String> {
    let mut out = start();
    let mobs = [
        ("sheep", "3.5 -60 0.5", "minecraft:entity_spawned", "a"), ("zombie", "20.5 -60 4.5", "minecraft:entity_spawned", "z"), ("skeleton", "-10.5 -60 3.5", "minecraft:entity_spawned", "s"),
        ("cow", "1.5 -60 -6.5", "minecraft:entity_born", "b"), ("villager", "-3.5 -60 9.5", "", "v"), ("chicken", "9.5 -60 -9.5", "minecraft:entity_spawned", "k"),
    ];
    out.extend(mobs.map(|(kind, at, event, name)| if kind == "villager" { format!("summon villager {name} {at}") } else { format!("summon {kind} {at} 0 0 {event} {name}") }));
    out.extend(owned([
        "effect @e[type=!player] slowness 1000 255 true",
        "replaceitem entity @e[name=z] slot.armor.head 0 iron_helmet", "replaceitem entity @e[name=z] slot.armor.chest 0 iron_chestplate", "replaceitem entity @e[name=z] slot.weapon.mainhand 0 iron_sword",
        "replaceitem entity @e[name=s] slot.armor.head 0 leather_helmet", "", "",
        "effect @e[type=!player] invisibility 4 0",
    ]));
    wait(&mut out, 6);
    out.extend(owned([
        "effect @e[name=a] invisibility 30 0 true", "effect @e[name=z] invisibility 30 0 true", "",
        "effect @e[name=a] invisibility 40 1 true", "",
        "effect @e[name=a] invisibility 0", "",
        "effect @e[name=z] clear", "",
        "effect @e[name=s] invisibility 60 0", "",
        "!rejoin", "", "", "",
        "effect @e[name=s] clear", "",
        "effect @s invisibility 3 0",
    ]));
    wait(&mut out, 5);
    out.extend(owned(["effect @s invisibility 30 0 true", "", "effect @s clear", "", "effect @e[name=v] invisibility 60 0 true", "", "kill @e[name=v]", "", "kill @e[type=!player]", ""]));
    wait(&mut out, 2);
    out
}

fn throw(out: &mut Vec<String>, potion: &str) {
    out.extend([format!("replaceitem entity @s slot.weapon.mainhand 0 {potion}"), "!look 0.01 -1 0".into(), "!use".into()]);
}

fn potions() -> Vec<String> {
    let mut out = start();
    out.extend(owned([
        "tp CmdWatcher 12.5 -60 12.5", "summon cow 2.5 -60 0.5 0 0 minecraft:entity_spawned c", "summon zombie 0.5 -60 3 0 0 minecraft:entity_spawned z", "summon cow -1.5 -60 0.5 0 0 minecraft:entity_spawned d",
        "effect @e[type=!player] slowness 1000 255 true", "replaceitem entity @e[name=z] slot.armor.head 0 iron_helmet", "", "",
    ]));
    throw(&mut out, "splash_potion 1 7");
    wait(&mut out, 3);
    out.extend(owned(["effect @e invisibility 0", "", ""]));
    throw(&mut out, "lingering_potion 1 7");
    wait(&mut out, 8);
    out.extend(owned(["effect @e invisibility 0", "", "kill @e[type=!player]", ""]));
    wait(&mut out, 2);
    out
}

fn targets() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["tp CmdWatcher 0.5 -60 -30.5", "gamemode survival @s", "effect @s resistance 1000 255 true", "effect @s invisibility 1000 0 true", ""]));
    for off in ["4.5", "9.5", "15.5"] {
        out.extend([format!("summon zombie 0.5 -60 {off} 0 0 minecraft:entity_spawned z"), "replaceitem entity @e[name=z] slot.armor.head 0 iron_helmet".into()]);
        wait(&mut out, 6);
        out.extend(owned(["kill @e[type=!player]", "tp @s 0.5 -60 0.5", ""]));
    }
    out.extend(owned(["gamemode creative @s", "effect @s clear", "tp @s 0.5 -60 -20.5", ""]));
    for off in ["4.5", "9.5", "15.5"] {
        out.extend([
            "summon villager v 0.5 -60 0.5".into(), "effect @e[name=v] slowness 1000 255 true".into(), "effect @e[name=v] invisibility 1000 0 true".into(),
            format!("summon zombie 0.5 -60 {off} 0 0 minecraft:entity_spawned z"), "replaceitem entity @e[name=z] slot.armor.head 0 iron_helmet".into(),
        ]);
        wait(&mut out, 6);
        out.extend(owned(["kill @e[type=!player]", ""]));
    }
    wait(&mut out, 2);
    out
}

/// As [`targets`], nearer: the zombie 1.5 and 2.5 from the invisible actor, then 9 from it seen.
fn nears() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["tp CmdWatcher 0.5 -60 -30.5", "gamemode survival @s", "effect @s resistance 1000 255 true", "effect @s invisibility 1000 0 true", ""]));
    for off in ["2", "3", "", "9.5"] {
        if off.is_empty() {
            out.push("effect @s invisibility 0".into());
            continue;
        }
        out.extend([format!("summon zombie 0.5 -60 {off} 0 0 minecraft:entity_spawned z"), "replaceitem entity @e[name=z] slot.armor.head 0 iron_helmet".into()]);
        wait(&mut out, 6);
        out.extend(owned(["kill @e[type=!player]", "tp @s 0.5 -60 0.5", ""]));
    }
    out.extend(owned(["gamemode creative @s", "effect @s clear", ""]));
    wait(&mut out, 2);
    out
}

/// The invisible actor in iron: four pieces with the zombie 15 off; two 9 and 15 off; one 9 off.
fn armours() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["tp CmdWatcher 0.5 -60 -30.5", "gamemode survival @s", "effect @s resistance 1000 255 true", "effect @s invisibility 1000 0 true"]));
    let worn = |out: &mut Vec<String>, pieces: &[&str]| {
        out.push("clear @s".into());
        out.extend(pieces.iter().map(|piece| format!("replaceitem entity @s slot.armor.{piece} 0 iron_{}", match *piece { "head" => "helmet", "chest" => "chestplate", "legs" => "leggings", _ => "boots" })));
    };
    for (pieces, off) in [(&["head", "chest", "legs", "feet"][..], "15.5"), (&["head", "chest"][..], "9.5"), (&["head", "chest"][..], "15.5"), (&["head"][..], "9.5")] {
        worn(&mut out, pieces);
        out.extend([format!("summon zombie 0.5 -60 {off} 0 0 minecraft:entity_spawned z"), "replaceitem entity @e[name=z] slot.armor.head 0 iron_helmet".into()]);
        wait(&mut out, 6);
        out.extend(owned(["kill @e[type=!player]", "tp @s 0.5 -60 0.5", ""]));
    }
    out.extend(owned(["clear @s", "gamemode creative @s", "effect @s clear", ""]));
    wait(&mut out, 2);
    out
}

const SHOWN: &[&str] = &["CommandOutput", "SetEntityData", "MobEffect", "AddEntity", "RemoveEntity", "EntityEvent", "LevelSoundEvent", "MobEquipment", "MobArmorEquipment", "AddPlayer"];
const TARGET_SHOWN: &[&str] = &["CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "EntityEvent"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "bf1give" => (gives(), SHOWN),
        "bf1potion" => (potions(), SHOWN),
        "bf1target" => (targets(), TARGET_SHOWN),
        "bf1near" => (nears(), TARGET_SHOWN),
        "bf1armour" => (armours(), TARGET_SHOWN),
        _ => return None,
    })
}
