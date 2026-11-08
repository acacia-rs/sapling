//! The fourteenth wave's group K drills (research/parity/FOURTEENTH-BRIEF.md), on creature_drill.rs's floor
//! (stone at y -27, x -8..16, z -8..8). `k2babies`: 100 plain summons of each farm animal and the cat.
//! `k2mounts`: a tamed horse and donkey bred; a tamed donkey clicked with a chest twice and a saddle,
//! then opened; a tamed llama opened and given horse armour, then a carpet, into its slot 1.
//! `k2trader`: a wandering trader killed, then one 30 off sent `minecraft:start_despawn`.
//! `k2shot`: a free pillager, then a skeleton, shooting at the survival actor. `k2freeze`: mobs held in
//! powder snow, then the actor frozen full and killed. `k2bolt`: a bolt summoned onto each `Animal` kind.
//! `k2sniff`: sniffers sent `on_scenting_success`: one dirt block only, again and again; one held by
//! slowness with soil 10 off; three on a dirt field. `k2llama`: a tamed llama given a chest, opened
//! holding a stick, offered horse armour then a carpet in slot 1. `k2freeze2`: a zombie frozen then
//! let out of the snow, a magma cube and a strider frozen.

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &[
    "AddEntity", "RemoveEntity", "SetEntityData", "EntityEvent", "LevelSoundEvent", "LevelEvent", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion", "AddItemEntity", "SetEntityLink",
    "CommandOutput", "UpdateEquipment", "ItemStackResponse", "InventorySlot", "InventoryContent", "MobArmorEquipment", "UpdateAttributes", "MobEffect",
];
const ADDS: &[&str] = &["AddEntity", "CommandOutput"];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "k2babies" => (babies(), ADDS),
        "k2mounts" => (mounts(), SHOWN),
        "k2trader" => (trader(), SHOWN),
        "k2shot" => (shot(), SHOWN),
        "k2freeze" => (freeze(), SHOWN),
        "k2bolt" => (bolt(), SHOWN),
        "k2sniff" => (sniff(), SHOWN),
        "k2llama" => (llama(), SHOWN),
        "k2freeze2" => (freeze2(), SHOWN),
        _ => return None,
    })
}

const HOME: &str = "tp @s 0.5 -26 0.5 -90 0";

fn babies() -> Vec<String> {
    let mut lines = floor();
    lines.extend((-6..4).map(|x| format!("summon armor_stand {x}.5 -26 -6.5")));
    for kind in ["cow", "pig", "chicken", "sheep", "mooshroom", "cat"] {
        lines.extend((0..10).map(|_| format!("execute at @e[type=armor_stand] run summon {kind} ~ ~ ~")));
        lines.extend(["".to_owned(), format!("kill @e[type={kind}]"), "".into(), "kill @e[type=item]".into(), "".into()]);
    }
    lines
}

/// The actor, put back at its spot, clicks the newest `kind` holding `item` (sneaking if `sneak`).
fn click(kind: &str, item: &str, sneak: bool) -> Vec<String> {
    let mut lines = vec!["clear @s".to_owned(), format!("give @s {item}"), HOME.into(), String::new()];
    lines.extend(sneak.then(|| "!sneak".to_owned()));
    lines.extend([format!("!interact {kind}"), String::new()]);
    lines.extend(sneak.then(|| "!unsneak".to_owned()));
    lines
}

fn tamed(kind: &str) -> Vec<String> {
    vec![format!("summon {kind} 2.5 -26 0.5 0 0 minecraft:spawn_tame_adult"), format!("effect @e[type={kind}] slowness 200 255 true"), String::new()]
}

fn mounts() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5", "give @s golden_carrot 64", "summon horse 2.5 -26 -0.5 0 0 minecraft:spawn_tame_adult", "summon donkey 2.5 -26 1.5 0 0 minecraft:spawn_tame_adult", "", "!feedall horse", "!feedall donkey"]);
    waits(&mut lines, 12);
    lines = with(lines, &["kill @e[type=!player]", "", "kill @e[type=item]", ""]);
    lines.extend(tamed("donkey"));
    lines.extend(click("donkey", "chest", false));
    lines.extend(click("donkey", "chest", false));
    lines.extend(click("donkey", "saddle", false));
    lines.extend(click("donkey", "stick", true));
    lines = with(lines, &["!close", "", "kill @e[type=!player]", "", "kill @e[type=item]", ""]);
    lines.extend(tamed("llama"));
    lines = with(lines, &["clear @s", "give @s iron_horse_armor", "give @s white_carpet", HOME, "", "!sneak", "!interact llama", "", "!unsneak", "!put 0 1", "", "!put 1 1", "", "!close", ""]);
    lines.extend(click("llama", "chest", false));
    lines = with(lines, &["give @s iron_horse_armor", "", "!sneak", "!interact llama", "", "!unsneak", "!put 0 1", "", "!close", ""]);
    with(lines, &["kill @e[type=!player]", "", "kill @e[type=item]", "clear @s", ""])
}

fn trader() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5", "summon wandering_trader 6.5 -26 0.5"]);
    waits(&mut lines, 3);
    lines.push("kill @e[type=wandering_trader]".into());
    waits(&mut lines, 8);
    lines = with(lines, &["kill @e[type=!player]", "", "kill @e[type=item]", "fill 26 -27 -4 34 -27 4 stone", "fill 26 -26 -4 34 -20 4 air", "summon wandering_trader 30.5 -26 0.5"]);
    waits(&mut lines, 3);
    lines.push("event entity @e[type=wandering_trader] minecraft:start_despawn".into());
    waits(&mut lines, 10);
    with(lines, &["kill @e[type=!player]", "", "kill @e[type=item]", ""])
}

fn shot() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5", "gamemode survival @s", "effect @s resistance 1000 255 true", "summon pillager 14.5 -26 0.5"]);
    for _ in 0..5 {
        waits(&mut lines, 5);
        lines.push(HOME.into());
    }
    lines = with(lines, &["kill @e[type=!player]", "", "summon skeleton 14.5 -26 0.5"]);
    for _ in 0..4 {
        waits(&mut lines, 5);
        lines.push(HOME.into());
    }
    with(lines, &["kill @e[type=!player]", "", "kill @e[type=item]", "kill @e[type=arrow]", "gamemode creative @s", "effect @s clear", ""])
}

fn freeze() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5", "time set noon"]);
    for (kind, x, z) in [("zombie", 4, 4), ("cow", 8, 4), ("skeleton", 12, 4), ("stray", 4, -4), ("polar_bear", 8, -4), ("pig", 12, -4)] {
        lines.push(format!("fill {} -26 {} {} -24 {} powder_snow", x - 1, z - 1, x + 1, z + 1));
        lines.push(format!("summon {kind} {x}.5 -26 {z}.5"));
    }
    lines.push("effect @e[type=!player] slowness 1000 255 true".into());
    waits(&mut lines, 45);
    lines = with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "", "kill @e[type=item]", "gamemode survival @s", "fill 0 -26 0 0 -25 0 powder_snow"]);
    waits(&mut lines, 10);
    lines.push("kill @s".into());
    waits(&mut lines, 4);
    lines = with(lines, &["fill 0 -26 0 0 -25 0 air", "gamemode creative @s", HOME]);
    waits(&mut lines, 4);
    lines
}

fn llama() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5"]);
    lines.extend(tamed("llama"));
    lines.extend(click("llama", "chest", true));
    lines = with(lines, &["clear @s", "give @s stick", "give @s iron_horse_armor", "give @s white_carpet", HOME, "", "!sneak", "", "!interact llama", "", "!unsneak", "", "!put 1 1", "", "!put 2 1", "", "!close", ""]);
    with(lines, &["kill @e[type=!player]", "", "kill @e[type=item]", "clear @s", ""])
}

fn freeze2() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5", "time set noon"]);
    for (kind, x, z) in [("zombie", 4, 4), ("magma_cube", 12, 4), ("strider", 8, -4)] {
        lines.push(format!("fill {} -26 {} {} -24 {} powder_snow", x - 1, z - 1, x + 1, z + 1));
        lines.push(format!("summon {kind} {x}.5 -26 {z}.5"));
    }
    lines.push("effect @e[type=!player] slowness 1000 255 true".into());
    waits(&mut lines, 10);
    lines.push("fill 3 -26 3 5 -24 5 air".into());
    waits(&mut lines, 5);
    with(lines, &["kill @e[type=!player]", "fill -8 -26 -8 16 -16 8 air", "", "kill @e[type=item]", ""])
}

/// Every kind of BDS's `Animal` actor type (0x1300 set in its id).
const ANIMALS: [&str; 31] = [
    "chicken", "cow", "pig", "sheep", "wolf", "mooshroom", "rabbit", "ocelot", "horse", "polar_bear", "llama", "parrot", "donkey", "mule", "skeleton_horse", "zombie_horse", "turtle", "cat", "panda", "fox",
    "hoglin", "strider", "goat", "axolotl", "frog", "trader_llama", "camel", "sniffer", "armadillo", "happy_ghast", "camel_husk",
];

fn bolt() -> Vec<String> {
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5", "gamerule domobloot false"]);
    for kind in ANIMALS {
        lines = with(lines, &[&format!("summon {kind} 8.5 -26 0.5"), "", "summon lightning_bolt 8.5 -26 0.5", "", "", "kill @e[type=!player]", ""]);
    }
    with(lines, &["gamerule domobloot true", ""])
}

fn sniff() -> Vec<String> {
    const EVENT: &str = "event entity @e[type=sniffer] on_scenting_success";
    let mut lines = with(floor(), &["tp CmdWatcher -6.5 -26 6.5", "tp @s -6.5 -26 -6.5", "time set noon", "setblock 8 -27 4 dirt", "summon sniffer 3.5 -26 0.5 0 0 minecraft:spawn_adult", "", EVENT]);
    waits(&mut lines, 40);
    lines.push(EVENT.into());
    waits(&mut lines, 30);
    lines.push(EVENT.into());
    waits(&mut lines, 15);
    lines = with(lines, &["kill @e[type=!player]", "kill @e[type=item]", "setblock 8 -27 4 stone", "setblock 12 -27 -5 dirt", "summon sniffer 3.5 -26 0.5 0 0 minecraft:spawn_adult", "effect @e[type=sniffer] slowness 1000 255 true", "", EVENT]);
    waits(&mut lines, 70);
    lines = with(lines, &["kill @e[type=!player]", "kill @e[type=item]", "fill -8 -27 -8 16 -27 8 dirt"]);
    lines.extend([[-3, 0], [6, -5], [10, 4]].map(|[x, z]| format!("summon sniffer {x}.5 -26 {z}.5 0 0 minecraft:spawn_adult")));
    lines = with(lines, &["", EVENT]);
    waits(&mut lines, 35);
    lines.push(EVENT.into());
    waits(&mut lines, 35);
    with(lines, &["kill @e[type=!player]", "kill @e[type=item]", "fill -8 -27 -8 16 -27 8 stone", ""])
}
