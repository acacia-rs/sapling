//! The fourteenth wave's group X drills (research/parity/FOURTEENTH-BRIEF.md), on the flat ground (stand at
//! y -60) unless said. `x2bat`: bats hung in a room 2 high, the actor standing in it 2 to 6 off, creative then
//! survival. `x2panda`: four lazy pandas watched a minute; a lazy and then a normal panda tempted from 8 off by
//! the survival actor's bamboo; two lazy pandas fed bamboo 6 apart beside eight bamboo stalks. `x2parrot`: three
//! wild parrots over the floor; one tamed by seeds, made to sit and stand by clicks while the actor walks off.
//! `x2bee`: by night four bees by a hive, a second hive 10 off; by night two bees in a hive that fire is set
//! beside. `x2wolf`: a tamed wolf biting a held, warded, healing zombie its owner hit, for a minute.
//! `x2bob`: two casts into a pool, each left 25 s.

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn start(time: &str) -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "weather clear", "difficulty easy"]);
    out.extend([format!("time set {time}"), "gamemode creative @a".into(), "effect @a clear".into(), "clear @s".into(), "fill -14 -60 -14 14 -54 14 air".into()]);
    out.extend(owned(["tp CmdWatcher 0.5 -60 -40.5", "tp @s 0.5 -60 -12.5", ""]));
    out
}

fn held(selector: &str) -> String {
    format!("effect @e[{selector}] slowness 1000 255 true")
}

fn survival() -> Vec<String> {
    owned(["gamemode survival @s", "effect @s resistance 1000 255 true"])
}

fn bats() -> Vec<String> {
    let mut out = start("18000");
    out.extend(owned(["fill -9 -60 -3 9 -57 3 stone hollow", "tp @s 0.5 -59 0.5 -90 0", ""]));
    for (mode, far) in [("creative", 2), ("survival", 6), ("survival", 5), ("survival", 4), ("survival", 3), ("survival", 2)] {
        out.extend([format!("gamemode {mode} @s"), "tp @s 0.5 -59 0.5 -90 0".into(), format!("summon bat {}.5 -58 0.5", far)]);
        wait(&mut out, 4);
        out.extend(owned(["kill @e[type=bat]", ""]));
    }
    out.extend(owned(["gamemode creative @s", "effect @s clear", "tp @s 0.5 -60 -12.5", "fill -9 -60 -3 9 -57 3 air", ""]));
    out
}

fn pandas() -> Vec<String> {
    let mut out = start("6000");
    for x in [-9, -3, 3, 9] {
        out.push(format!("summon panda {x}.5 -60 6.5 0 0 minecraft:panda_lazy"));
    }
    wait(&mut out, 60);
    out.extend(owned(["kill @e[type=!player]", ""]));
    out.extend(survival());
    for gene in ["lazy", "normal"] {
        out.extend(owned(["clear @s", "tp @s 0.5 -60 0.5 0 0"]));
        out.extend([format!("summon panda 0.5 -60 8.5 180 0 minecraft:panda_{gene}"), String::new(), "replaceitem entity @s slot.weapon.mainhand 0 bamboo 1".into()]);
        wait(&mut out, 10);
        out.extend(owned(["clear @s", "kill @e[type=panda]", ""]));
    }
    out.extend(owned(["fill -4 -60 4 4 -60 4 bamboo", "tp @s 0.5 -60 1.5 0 0", "summon panda -2.5 -60 2.5 0 0 minecraft:panda_lazy", "summon panda 3.5 -60 2.5 0 0 minecraft:panda_lazy"]));
    out.extend([held("type=panda"), "give @s bamboo 8".into(), String::new(), "!feedall panda".into(), String::new(), "effect @e[type=panda] clear".into(), "clear @s".into()]);
    wait(&mut out, 20);
    out.extend(owned(["kill @e[type=!player]", "fill -4 -60 4 4 -60 4 air", "gamemode creative @s", "effect @s clear", ""]));
    out
}

fn parrots() -> Vec<String> {
    let mut out = start("6000");
    out.extend(owned(["summon parrot -4.5 -58 6.5", "summon parrot 0.5 -58 6.5", "summon parrot 4.5 -58 6.5"]));
    wait(&mut out, 40);
    out.extend(owned(["kill @e[type=!player]", ""]));
    out.extend(survival());
    out.extend(owned(["tp @s 0.5 -60 0.5 -90 0", "summon parrot 2.5 -60 0.5", "effect @e[type=parrot] slowness 1000 255 true", "give @s wheat_seeds 32", ""]));
    for _ in 0..6 {
        out.extend(owned(["tp @s 0.5 -60 0.5 -90 0", "!feedall parrot", ""]));
    }
    out.extend(owned(["clear @s", "effect @e[type=parrot] clear", "tp @e[type=parrot] 2.5 -60 0.5", "tp @s 0.5 -60 0.5 -90 0", "", "!interact parrot"]));
    wait(&mut out, 4);
    out.push("tp @s 0.5 -60 10.5".into());
    wait(&mut out, 8);
    out.extend(owned(["tp @s 1.5 -60 2.5 180 0", "", "!interact parrot"]));
    wait(&mut out, 2);
    out.push("tp @s 0.5 -60 12.5".into());
    wait(&mut out, 12);
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", "clear @s", ""]));
    out
}

fn bees() -> Vec<String> {
    let mut out = start("18000");
    out.extend(owned(["setblock 0 -60 6 beehive [\"direction\"=2]", "setblock 10 -60 6 beehive [\"direction\"=2]"]));
    for x in ["-1.5", "0.5", "1.5", "2.5"] {
        out.push(format!("summon bee {x} -59 3.5"));
    }
    wait(&mut out, 45);
    out.extend(owned(["kill @e[type=!player]", "fill 0 -60 6 10 -60 6 air", ""]));
    out.extend(owned(["setblock 0 -60 6 beehive [\"direction\"=2]", "summon bee -0.5 -59 4.5", "summon bee 1.5 -59 4.5"]));
    wait(&mut out, 12);
    out.extend(owned(["setblock 1 -60 6 fire", "setblock 0 -59 6 planks"]));
    wait(&mut out, 40);
    out.extend(owned(["kill @e[type=!player]", "fill -2 -60 4 2 -58 8 air", ""]));
    out
}

fn wolves() -> Vec<String> {
    let mut out = start("18000");
    out.extend(survival());
    out.extend(owned(["tp @s 0.5 -60 0.5 -90 0", "summon wolf 2.5 -60 -0.5", "effect @e[type=wolf] slowness 60 255 true", "give @s bone 64", ""]));
    for _ in 0..8 {
        out.extend(owned(["tp @s 0.5 -60 0.5 -90 0", "!feedall wolf", ""]));
    }
    out.extend(owned(["clear @s", "effect @e[type=wolf] clear", "summon zombie z 2.5 -60 2.5", ""]));
    out.extend([held("name=z"), "effect @e[name=z] resistance 1000 3 true".into(), "effect @e[name=z] regeneration 1000 4 true".into(), "effect @e[name=z] fire_resistance 1000 0 true".into(), String::new(), "!hit zombie".into()]);
    wait(&mut out, 60);
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", ""]));
    out
}

fn bobs() -> Vec<String> {
    let mut out = start("6000");
    out.extend(owned(["fill 3 -63 -5 13 -61 5 water", "tp CmdWatcher -3.5 -60 4.5"]));
    out.extend(survival());
    out.extend(owned(["tp @s 0.5 -60 0.5 -90 0", "replaceitem entity @s slot.weapon.mainhand 0 fishing_rod", ""]));
    for aim in ["!look 6 -1 0", "!look 4 -1 2"] {
        out.extend(owned([aim, "", "!use"]));
        wait(&mut out, 25);
        out.extend(owned(["!use", "", "", "tp @s 0.5 -60 0.5 -90 0", ""]));
    }
    out.extend(owned(["kill @e[type=!player]", "fill 3 -63 -5 13 -62 5 dirt", "fill 3 -61 -5 13 -61 5 grass", "gamemode creative @s", "effect @s clear", "clear @s", ""]));
    out
}

const SHOWN: &[&str] = &[
    "CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "EntityEvent", "SetEntityMotion", "LevelSoundEvent", "LevelEvent", "UpdateAttributes", "MobEffect",
    "AddItemEntity", "UpdateBlock", "Animate", "BlockActorData", "SetEntityLink",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "x2bat" => (bats(), SHOWN),
        "x2panda" => (pandas(), SHOWN),
        "x2parrot" => (parrots(), SHOWN),
        "x2bee" => (bees(), SHOWN),
        "x2wolf" => (wolves(), SHOWN),
        "x2bob" => (bobs(), SHOWN),
        _ => return None,
    })
}
