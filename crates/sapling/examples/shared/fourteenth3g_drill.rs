//! The fourteenth wave's group G drills (research/parity/FOURTEENTH-BRIEF.md): animal behaviour, on the
//! flat ground at midnight unless said. `g14evoker`: a creative actor 2 to 11 from a new evoker, then a
//! survival one 5 off. `g14wolf`: a wolf tamed by feeding; its owner hits a held zombie, then
//! a free zombie goes for the owner. `g14panda`: a lazy, a playful and a worried panda (a held cow 5 from
//! it) far from the players; a held worried panda with a held cow 4 to 16 off; three aggressive
//! bystanders while the actor hits a fourth. `g14fly`: a bat in a room 2 high; an allay given a stick
//! whose owner moves 22 and then 20 away. `g14frog`: a frog by a small slime and then a small magma cube,
//! watched on; two frogs fed slime balls by water; a frog in a glass pen 6 high. `g14bee`: by day,
//! bees over poppies with wheat (random ticks off) between them and a hive; by night a bee into each of
//! two hives, one broken with a silk touch axe, the other over a campfire broken by hand.

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn start(difficulty: &str) -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "weather clear", "time set 18000"]);
    out.extend([format!("difficulty {difficulty}"), "gamemode creative @a".into(), "effect @a clear".into(), "clear @s".into(), "fill -12 -60 -12 12 -54 12 air".into()]);
    out.extend(owned(["tp CmdWatcher 0.5 -60 -40.5", "tp @s 0.5 -60 -12.5", ""]));
    out
}

fn held(name: &str) -> String {
    format!("effect @e[name={name}] slowness 1000 255 true")
}

fn evokers() -> Vec<String> {
    let mut out = start("easy");
    for far in [2, 5, 7, 9, 11] {
        out.extend(["tp @s 0.5 -60 0.5 0 0".to_string(), format!("summon evocation_illager 0.5 -60 {far}.5 180 0")]);
        wait(&mut out, 5);
        out.extend(owned(["kill @e[type=!player]", ""]));
    }
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 255 true", "tp @s 0.5 -60 0.5 0 0", "summon evocation_illager 0.5 -60 5.5 180 0"]));
    wait(&mut out, 8);
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", ""]));
    out
}

fn wolves() -> Vec<String> {
    let mut out = start("easy");
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 255 true", "tp @s 0.5 -60 0.5 -90 0", "summon wolf 2.5 -60 -0.5", "effect @e[type=wolf] slowness 60 255 true", "give @s bone 64", ""]));
    for _ in 0..8 {
        out.extend(owned(["tp @s 0.5 -60 0.5 -90 0", "!feedall wolf", ""]));
    }
    out.extend(owned(["clear @s", "effect @e[type=wolf] clear", ""]));
    out.extend(["summon zombie z 2.5 -60 2.5".to_string(), held("z"), "effect @e[name=z] resistance 1000 3 true".into(), String::new(), "!hit zombie".into()]);
    wait(&mut out, 12);
    out.extend(owned(["kill @e[type=zombie]", "tp @s 0.5 -60 0.5 -90 0", "summon zombie y 0.5 -60 8.5", "effect @e[name=y] resistance 1000 3 true"]));
    wait(&mut out, 14);
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", ""]));
    out
}

fn pandas() -> Vec<String> {
    let mut out = start("easy");
    out.extend(owned(["tp @s 0.5 -60 -38.5", "summon cow c 12.5 -60 5.5"]));
    out.extend([held("c"), "summon panda -12.5 -60 0.5 0 0 minecraft:panda_lazy".into(), "summon panda 0.5 -60 0.5 0 0 minecraft:panda_playful".into(), "summon panda 12.5 -60 0.5 0 0 minecraft:panda_worried".into()]);
    wait(&mut out, 30);
    out.extend(owned(["kill @e[type=!player]", ""]));
    for far in [4, 7, 10, 13, 16] {
        out.extend(["summon cow c 0.5 -60 0.5".to_string(), held("c"), format!("summon panda 0.5 -60 {far}.5 0 0 minecraft:panda_worried w"), held("w")]);
        wait(&mut out, 3);
        out.extend(owned(["kill @e[type=!player]", ""]));
    }
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 255 true", "tp @s 0.5 -60 0.5 0 0"]));
    for at in ["8.5 -60 0.5", "8.5 -60 4.5", "-7.5 -60 4.5", "0.5 -60 2.5"] {
        out.push(format!("summon panda {at} 0 0 minecraft:panda_aggressive"));
    }
    out.extend(owned(["", "!hit panda"]));
    wait(&mut out, 12);
    out.extend(owned(["kill @e[type=!player]", "gamemode creative @s", "effect @s clear", ""]));
    out
}

fn flyers() -> Vec<String> {
    let mut out = start("easy");
    out.extend(owned(["fill -3 -60 -3 3 -57 3 stone hollow", "summon bat 0.5 -59 0.5"]));
    wait(&mut out, 40);
    out.extend(owned(["kill @e[type=!player]", "fill -3 -60 -3 3 -57 3 air", "tp @s 0.5 -60 0.5 0 0", "summon allay 0.5 -59 3.5", "replaceitem entity @s slot.weapon.mainhand 0 stick", "", "!interact allay", ""]));
    out.push("tp @s 0.5 -60 22.5".into());
    wait(&mut out, 8);
    out.push("tp @s 20.5 -60 22.5".into());
    wait(&mut out, 8);
    out.extend(owned(["kill @e[type=!player]", "clear @s", ""]));
    out
}

/// Four bats in a room 1 high (each always under its ceiling), the creative actor then the
/// survival actor standing on the ceiling over them, then the ceiling taken away.
fn bats() -> Vec<String> {
    let mut out = start("easy");
    out.extend(owned(["fill -4 -60 -4 4 -58 4 stone hollow"]));
    for at in ["-2.5 -59 -2.5", "2.5 -59 -2.5", "-2.5 -59 2.5", "2.5 -59 2.5"] {
        out.push(format!("summon bat {at}"));
    }
    wait(&mut out, 25);
    out.push("tp @s 0.5 -57 0.5".into());
    wait(&mut out, 4);
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 255 true"]));
    wait(&mut out, 4);
    out.extend(owned(["gamemode creative @s", "effect @s clear", "tp @s 0.5 -60 -12.5", "fill -4 -58 -4 4 -58 4 air"]));
    wait(&mut out, 4);
    out.extend(owned(["kill @e[type=!player]", "fill -4 -60 -4 4 -58 4 air", ""]));
    out
}

fn frogs() -> Vec<String> {
    let mut out = start("easy");
    out.extend(owned(["summon frog f 0.5 -60 0.5", "summon slime 4.5 -60 0.5 0 0 spawn_small"]));
    wait(&mut out, 8);
    out.push("summon magma_cube 0.5 -60 4.5 0 0 spawn_small".into());
    wait(&mut out, 30);
    out.extend(owned(["kill @e[type=!player]", "fill 3 -61 -2 7 -61 2 water", "summon frog a 0.5 -60 0.5", "summon frog b 1.5 -60 0.5"]));
    out.extend([held("a"), held("b"), "tp @s 0.5 -60 -1.5 0 0".into(), "give @s slime_ball 8".into(), String::new(), "!feedall frog".into(), String::new(), "effect @e[type=frog] clear".into(), "clear @s".into()]);
    wait(&mut out, 40);
    out.extend(owned(["kill @e[type=!player]", "fill 3 -61 -2 7 -61 2 grass", "tp @s 0.5 -60 -12.5", "fill -2 -60 -2 2 -55 2 glass", "fill -1 -60 -1 1 -55 1 air", "summon frog p 0.5 -60 0.5"]));
    wait(&mut out, 30);
    out.extend(owned(["kill @e[type=!player]", "fill -2 -60 -2 2 -55 2 air", ""]));
    out
}

fn bees() -> Vec<String> {
    let mut out = start("easy");
    out.extend(owned(["time set 6000", "gamerule randomtickspeed 0", "fill -2 -61 1 2 -61 7 farmland", "fill -2 -60 1 2 -60 7 wheat", "fill -2 -60 0 2 -60 0 poppy", "setblock 0 -60 9 beehive [\"direction\"=2]"]));
    out.extend(owned(["summon bee 0.5 -59 0.5", "summon bee -1.5 -59 0.5", "summon bee 2.5 -59 0.5"]));
    wait(&mut out, 70);
    out.extend(owned(["kill @e[type=!player]", "fill -2 -61 0 2 -61 9 grass", "fill -2 -60 0 2 -60 9 air", "gamerule randomtickspeed 1"]));
    out.extend(owned(["time set 18000", "setblock 4 -60 0 beehive [\"direction\"=1]", "summon bee 6.5 -59 0.5", "setblock 4 -61 4 campfire", "setblock 4 -60 4 beehive [\"direction\"=1]", "summon bee 6.5 -59 4.5"]));
    wait(&mut out, 15);
    out.extend(owned(["time set 6000", "gamemode survival @s", "effect @s resistance 1000 255 true", "tp @s 2.5 -60 0.5 -90 0", "replaceitem entity @s slot.weapon.mainhand 0 diamond_axe", "enchant @s silk_touch", "", "!break 2 0 0"]));
    wait(&mut out, 5);
    out.extend(owned(["replaceitem entity @s slot.weapon.mainhand 0 air", "tp @s 2.5 -60 4.5 -90 0", "", "!break 2 0 0"]));
    wait(&mut out, 10);
    out.extend(owned(["kill @e[type=!player]", "setblock 4 -61 4 grass", "fill 4 -60 0 4 -60 4 air", "gamemode creative @s", "effect @s clear", "clear @s", "time set 18000", ""]));
    out
}

const SHOWN: &[&str] = &[
    "CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityData", "EntityEvent", "SetEntityMotion", "LevelSoundEvent", "LevelEvent", "UpdateAttributes", "MobEffect",
    "AddItemEntity", "UpdateBlock", "Animate", "BlockActorData",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "g14evoker" => (evokers(), SHOWN),
        "g14wolf" => (wolves(), SHOWN),
        "g14panda" => (pandas(), SHOWN),
        "g14fly" => (flyers(), SHOWN),
        "g14bat" => (bats(), SHOWN),
        "g14frog" => (frogs(), SHOWN),
        "g14bee" => (bees(), SHOWN),
        _ => return None,
    })
}
