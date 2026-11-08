//! Fourteenth wave, group AC (drill2): `ac2wolf`: five wolves tamed east of the survival actor, then a
//! stone wall between them and a held zombie west of the actor, which the actor hits: do wolves that
//! cannot see it take up their owner's fight? `ac2panda`: lazy, then normal pandas fed bamboo by bamboo
//! rows: their courting pace.

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn wolves_behind_wall() -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "weather clear", "difficulty easy", "time set 18000"]);
    out.extend(owned(["gamemode creative @a", "effect @a clear", "clear @s", "fill -14 -60 -14 14 -54 14 air", "tp CmdWatcher 0.5 -60 -40.5", ""]));
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 255 true", "tp @s 1.5 -60 0.5 -90 0"]));
    for z in ["-1.5", "-0.5", "0.5", "1.5", "2.5"] {
        out.push(format!("summon wolf 3.5 -60 {z}"));
    }
    out.extend(owned(["effect @e[type=wolf] slowness 60 255 true", "give @s bone 64", ""]));
    for _ in 0..4 {
        out.extend(owned(["tp @s 1.5 -60 0.5 -90 0", "!feedall wolf", ""]));
    }
    out.extend(owned(["clear @s", "tp @s -0.5 -60 0.5 90 0", "fill 2 -60 -6 2 -58 6 stone", "effect @e[type=wolf] clear", "summon zombie z -1.5 -60 0.5", "effect @e[name=z] slowness 1000 255 true", ""]));
    out.extend(owned(["effect @e[name=z] resistance 1000 3 true", "effect @e[name=z] regeneration 1000 4 true", "", "!hit zombie"]));
    wait(&mut out, 30);
    out.extend(owned(["kill @e[type=!player]", "fill 2 -60 -6 2 -58 6 air", "gamemode creative @s", "effect @s clear", ""]));
    out
}

/// Two lazy pandas 4 apart fed bamboo between two bamboo rows, then two normal ones: do they court?
fn pandas_in_love() -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "weather clear", "difficulty easy", "time set 6000"]);
    out.extend(owned(["gamemode creative @a", "effect @a clear", "clear @s", "fill -14 -60 -14 14 -54 14 air", "tp CmdWatcher 0.5 -60 -40.5", ""]));
    out.extend(owned(["fill -6 -60 6 6 -60 6 bamboo", "fill -6 -60 -3 6 -60 -3 bamboo", "gamemode survival @s", "effect @s resistance 1000 255 true", ""]));
    for gene in ["lazy", "normal"] {
        out.extend(owned(["tp @s 0.5 -60 1.5 0 0"]));
        out.extend(["-1.5", "2.5"].map(|x| format!("summon panda {x} -60 2.5 0 0 minecraft:panda_{gene}")));
        out.extend(owned(["give @s bamboo 8", "", "!feedall panda", "", "clear @s", "tp @s 0.5 -60 -1.5 0 0"]));
        wait(&mut out, 30);
        out.extend(owned(["kill @e[type=panda]", "kill @e[type=item]", ""]));
    }
    out.extend(owned(["fill -6 -60 6 6 -60 6 air", "fill -6 -60 -3 6 -60 -3 air", "gamemode creative @s", "effect @s clear", ""]));
    out
}

/// bds-j3ride's spear rider on its zombie horse, after two horses summoned and killed out of the way, so
/// sapling's run rolls its horse another `movement` than the 0.209 j3ride's always got.
fn spear_rider() -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "weather clear", "time set 18000", "difficulty easy"]);
    out.extend(owned(["gamemode creative @a", "effect @a clear", "clear @s", "fill -12 -60 -12 12 -54 12 air", "fill -12 -63 -12 12 -62 12 dirt", "fill -12 -61 -12 12 -61 12 grass"]));
    out.extend(owned(["tp CmdWatcher 0.5 -60 -40.5", "tp @s 0.5 -60 -12.5", "", "gamemode survival @s", "effect @s resistance 1000 3 true", "effect @s instant_health 1 4 true"]));
    out.extend(owned(["summon zombie_horse 0.5 -60 -30.5", "summon zombie_horse 2.5 -60 -30.5", "kill @e[type=zombie_horse]", "tp @s 0.5 -60 0.5 0 0"]));
    out.push("summon zombie_horse 0.5 -60 8.5 180 0 minecraft:spawn_adult_with_rider".into());
    wait(&mut out, 12);
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", ""]));
    out
}

const SHOWN: &[&str] = &["CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "EntityEvent", "SetEntityMotion", "LevelSoundEvent"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ac2wolf" => (wolves_behind_wall(), SHOWN),
        "ac2panda" => (pandas_in_love(), SHOWN),
        "ac2ride" => (spear_rider(), SHOWN),
        _ => return None,
    })
}
