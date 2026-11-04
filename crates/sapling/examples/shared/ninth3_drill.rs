//! The ninth wave's mount and small mob rule drills (research/parity/NINTH-BRIEF.md "Group 3"), on
//! harm_drill.rs's floor (stone at y -27): lightning on pigs and villagers (`n3struck`), breath
//! coming back and armour stands burning (`n3breath`), how deep water is before a mob swims up
//! (`n3float`), mobs on turtle eggs and clutches laid (`n3eggs`), a pregnant turtle taken from home
//! (`n3home`), shots' knockback (`n3knock`), wild mounts, the carrot stick and leads (`n3ride`),
//! exit spots (`n3exit`), foals' strides (`n3foals`), frogs' waits (`n3frogs`) and the carrot stick penned
//! and in the air (`n3stick`).

use super::harm::{mid, pen, start, summon, wait};

const SHOWN: &[&str] = &[
    "UpdateAttributes", "EntityEvent", "SetEntityData", "LevelSoundEvent", "LevelEvent", "MovePlayer", "SetEntityMotion", "RemoveEntity", "AddEntity", "AddItemEntity", "MoveEntityDelta",
    "MoveEntityAbsolute", "CorrectPlayerMovePrediction", "MobEquipment", "UpdateBlock", "UpdateSubchunkBlocks", "InventorySlot", "InventoryContent", "SetEntityLink", "Text", "CommandOutput",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "n3struck" => struck(),
            "n3breath" => breath(),
            "n3float" => float(),
            "n3eggs" => eggs(),
            "n3home" => home(),
            "n3knock" => knock(),
            "n3ride" => ride(),
            "n3foals" => foals(),
            "n3frogs" => frogs(),
            "n3stick" => stick(),
            "n3exit" => exits(),
            _ => return None,
        },
        SHOWN,
    ))
}

/// A bolt on a pig, a villager, a mooshroom and a creeper, a second apart, each in its pen.
fn struck() -> Vec<String> {
    let mut lines = start("noon", 0);
    lines.push("gamerule dofiretick false".into());
    let mobs = [(4, "pig"), (7, "villager"), (-4, "mooshroom"), (-7, "creeper")];
    for (x, kind) in mobs {
        pen(&mut lines, x, 4, 3);
        lines.push(summon(kind, mid(x), -26.0, 4.5));
    }
    wait(&mut lines, 2);
    for (x, _) in mobs {
        lines.extend([format!("summon lightning_bolt {} -26 4.5", mid(x)), String::new(), String::new()]);
    }
    wait(&mut lines, 2);
    lines.extend(["kill @e[type=!player]".into(), "fill -9 -26 2 9 -16 6 air".into(), String::new()]);
    lines
}

/// A cow kept under water 8 s and let out; armour stands in lava and in fire.
fn breath() -> Vec<String> {
    let mut lines = start("noon", 0);
    lines.push("gamerule dofiretick false".into());
    pen(&mut lines, -4, 4, 3);
    lines.extend(["setblock -4 -26 4 lava".into(), summon("armor_stand", -3.5, -26.0, 4.5)]);
    pen(&mut lines, -7, 4, 3);
    lines.extend(["setblock -7 -27 4 netherrack".into(), "setblock -7 -26 4 fire".into(), summon("armor_stand", -6.5, -26.0, 4.5)]);
    pen(&mut lines, 4, 4, 4);
    lines.extend(["fill 4 -26 4 4 -25 4 water".into(), "setblock 4 -24 4 stone".into(), summon("cow", 4.5, -26.0, 4.5)]);
    wait(&mut lines, 8);
    lines.push("fill 4 -26 4 4 -24 4 air".into());
    wait(&mut lines, 16);
    lines.extend(["kill @e[type=!player]".into(), "fill -9 -27 2 9 -16 6 air".into(), "fill -9 -27 2 9 -27 6 stone".into(), String::new()]);
    lines
}

/// Cows (z > 0) and chickens (z < 0) held still in water `level` 1 to 6 cells from its source,
/// each lane walled one wide so the water stands.
fn float() -> Vec<String> {
    let mut lines = start("noon", 0);
    lines.push("tp @s -3.5 -26 0.5".into());
    let lanes: Vec<(i32, i32, &str)> = (1..=6).flat_map(|level| [(level, 2 * level, "cow"), (level, -2 * level, "chicken")]).collect();
    for &(_, z, _) in &lanes {
        lines.push(format!("fill -9 -26 {} 0 -23 {} stone", z - 1, z + 1));
    }
    for &(level, z, _) in &lanes {
        lines.extend([format!("fill -8 -26 {z} {} -23 {z} air", -8 + level), format!("setblock -8 -26 {z} water")]);
    }
    wait(&mut lines, 3);
    for &(level, z, kind) in &lanes {
        lines.push(summon(&format!("{kind} 0 0 minecraft:spawn_adult"), mid(-8 + level), -26.0, mid(z)));
    }
    lines.extend(["effect @e[type=cow] slowness 60 255 true".into(), "effect @e[type=chicken] slowness 60 255 true".into()]);
    wait(&mut lines, 10);
    lines.extend(["kill @e[type=!player]".into(), "fill -9 -26 -13 0 -23 13 air".into(), String::new()]);
    lines
}

/// Mobs on single eggs with griefing off (how high they stand), then eight turtles wanting to lay,
/// each in a pen on sand held up by stone (the clutch sizes), and one in a lane with sand 2 west
/// and 3 east.
fn eggs() -> Vec<String> {
    let mut lines = start("noon", -4);
    lines.push("gamerule mobgriefing false".into());
    let standing = [(4, "cow"), (7, "pig"), (-4, "zombie"), (-7, "chicken")];
    for (x, kind) in standing {
        pen(&mut lines, x, -8, 3);
        lines.extend([format!("setblock {x} -26 -8 turtle_egg"), summon(kind, mid(x), -25.0, -7.5)]);
    }
    wait(&mut lines, 4);
    lines.extend(["kill @e[type=!player]".into(), "fill -9 -26 -10 9 -16 -6 air".into(), "gamerule mobgriefing true".into(), String::new()]);
    let pens: Vec<(i32, i32)> = [-6, -2, 2, 6].into_iter().flat_map(|x| [(x, 4), (x, 8)]).collect();
    for &(x, z) in &pens {
        lines.push(format!("fill {} -26 {} {} -24 {} stone", x - 2, z - 2, x + 2, z + 2));
    }
    for &(x, z) in &pens {
        lines.extend([format!("fill {} -28 {} {} -28 {} stone", x - 1, z - 1, x + 1, z + 1), format!("fill {} -27 {} {} -27 {} sand", x - 1, z - 1, x + 1, z + 1), format!("fill {} -26 {} {} -24 {} air", x - 1, z - 1, x + 1, z + 1)]);
    }
    lines.extend(["fill -6 -26 -13 6 -24 -11 stone", "fill -5 -26 -12 5 -24 -12 air", "fill -5 -28 -12 5 -28 -12 stone", "setblock -2 -27 -12 sand", "setblock 3 -27 -12 sand"].map(String::from));
    for &(x, z) in &pens {
        lines.push(summon("turtle 0 0 minecraft:go_lay_egg", mid(x), -26.0, mid(z)));
    }
    lines.push(summon("turtle 0 0 minecraft:go_lay_egg", 0.5, -26.0, -11.5));
    wait(&mut lines, 50);
    lines.extend(["kill @e[type=!player]".into(), "fill -9 -27 -14 9 -16 11 air".into(), "fill -9 -27 -14 9 -27 11 stone".into(), String::new()]);
    lines
}

/// Four pregnant turtles each taken 10 east of where they came the second after, on stone with
/// sand at home; where they walk in 100 s.
fn home() -> Vec<String> {
    let mut lines = start("noon", 0);
    lines.push("gamerule mobgriefing true".into());
    for z in [-9, -3, 3, 9] {
        lines.extend([format!("fill -8 -28 {} -6 -28 {} stone", z - 1, z + 1), format!("fill -8 -27 {} -6 -27 {} sand", z - 1, z + 1), summon("turtle 0 0 minecraft:become_pregnant", -6.5, -26.0, mid(z))]);
    }
    lines.push(String::new());
    for z in [-9, -3, 3, 9] {
        lines.push(format!("tp @e[type=turtle,x=-7,y=-26,z={z},r=2] 3.5 -26 {}", mid(z)));
    }
    wait(&mut lines, 100);
    lines.extend(["kill @e[type=!player]".into(), "fill -9 -27 -11 9 -27 11 stone".into(), "fill -20 -26 -20 20 -16 20 air".into(), String::new()]);
    lines
}

/// Dispensers shooting an arrow and a snowball at cows 3 east, and an arrow at the actor.
fn knock() -> Vec<String> {
    let mut lines = start("noon", -8);
    lines.extend(["gamemode survival @s", "effect @s resistance 60 4 true"].map(String::from));
    for (z, shot) in [(2, "arrow"), (5, "snowball"), (-8, "arrow")] {
        lines.extend([format!("setblock -3 -26 {z} dispenser [\"facing_direction\"=5]"), format!("replaceitem block -3 -26 {z} slot.container 0 {shot} 1")]);
    }
    lines.extend([summon("cow 0 0 minecraft:spawn_adult", 0.5, -26.0, 2.5), summon("cow 0 0 minecraft:spawn_adult", 0.5, -26.0, 5.5), "effect @e[type=cow] slowness 30 255 true".into(), String::new()]);
    for z in [2, 5, -8] {
        if z > 0 {
            lines.push(format!("tp @e[type=cow,x=0,y=-26,z={z},c=1] 0.5 -26 {}", mid(z)));
        }
        lines.extend([format!("setblock -3 -25 {z} redstone_block"), String::new(), String::new()]);
    }
    wait(&mut lines, 2);
    lines.extend(["kill @e[type=!player]".into(), "fill -4 -26 -9 1 -24 6 air".into(), "gamemode creative @s".into(), "effect @s clear".into(), String::new()]);
    lines
}

/// A ride by a fresh saddled pig: `block` set on the cell north of it (or the floor there for `floor`) after
/// getting on, then off by command: where the rider is put.
fn exit_trial(lines: &mut Vec<String>, x: i32, block: &str) {
    let (y, block) = block.strip_prefix("floor ").map_or((-26, block), |block| (-27, block));
    lines.extend([
        "kill @e[type=pig]".into(),
        summon("pig 0 0 minecraft:on_saddled", mid(x), -26.0, -11.5),
        "effect @e[type=pig] slowness 30 255 true".into(),
        "ride @s start_riding @e[type=pig,c=1] teleport_rider".into(),
        format!("setblock {x} {y} -13 {block}"),
        "ride @s stop_riding".into(),
        String::new(),
    ]);
}

/// A wild horse ridden 12 s, three times; a saddled pig boosted by a nearly worn stick, on the
/// ground and falling; a cow led 3 more blocks off each 2 s, then 15.
fn ride() -> Vec<String> {
    let mut lines = start("noon", 0);
    lines.push("gamerule dofiretick false".into());
    for _ in 0..3 {
        lines.extend(["kill @e[type=!player]".into(), "tp @s 0.5 -26 0.5".into(), summon("horse 0 0 minecraft:spawn_adult", 2.5, -26.0, 0.5), String::new(), "!interact horse".into()]);
        wait(&mut lines, 12);
    }
    lines.extend(["ride @s stop_riding", "kill @e[type=!player]", "tp @s 0.5 -26 0.5", "gamemode survival @s", "effect @s resistance 120 4 true"].map(String::from));
    lines.extend([summon("pig 0 0 minecraft:on_saddled", 2.5, -26.0, 0.5), "ride @s start_riding @e[type=pig,c=1] teleport_rider".into(), "replaceitem entity @s slot.hotbar 0 carrot_on_a_stick 1 21".into(), String::new()]);
    lines.extend(["!use", "", "", "", "", "!use", "", "", "", "", "!use", "", "", ""].map(String::from));
    lines.extend(["ride @s stop_riding", "kill @e[type=pig]", "replaceitem entity @s slot.hotbar 0 carrot_on_a_stick 1 0"].map(String::from));
    lines.extend([summon("pig 0 0 minecraft:on_saddled", 2.5, -6.0, 0.5), "ride @s start_riding @e[type=pig,c=1] teleport_rider".into(), "!use".into(), String::new(), String::new(), String::new()]);
    lines.extend(["ride @s stop_riding", "kill @e[type=!player]", "gamemode creative @s", "effect @s clear", "tp @s -12.5 -26 0.5", "replaceitem entity @s slot.hotbar 0 lead 1"].map(String::from));
    lines.extend([summon("cow 0 0 minecraft:spawn_adult", -10.5, -26.0, 0.5), String::new(), "!interact cow".into(), String::new(), String::new()]);
    for x in [-8, -5, -2, 1, 4, 7] {
        lines.extend([format!("tp @s {} -26 0.5", mid(x)), String::new(), String::new()]);
    }
    lines.extend(["tp @s 19.5 -26 0.5", "", "", "", "kill @e[type=!player]", "tp @s 0.5 -26 0.5", "clear @s"].map(String::from));
    lines
}

/// Where a rider is put off a saddled pig with a block of each of twelve kinds north of it, and off
/// a harnessed happy ghast.
fn exits() -> Vec<String> {
    let mut lines = start("noon", 0);
    lines.extend(["gamerule dofiretick false", "tp @s 0.5 -26 -8.5"].map(String::from));
    let kinds = ["stone", "water", "lava", "fire", "web", "sweet_berry_bush [\"growth\"=3]", "powder_snow", "floor magma", "wither_rose", "ladder [\"facing_direction\"=3]", "campfire", "floor air"];
    for (at, block) in kinds.iter().enumerate() {
        exit_trial(&mut lines, -11 + 2 * at as i32, block);
    }
    lines.extend(["kill @e[type=!player]".into(), "fill -12 -27 -14 12 -16 -10 air".into(), "fill -12 -27 -14 12 -27 -10 stone".into(), String::new()]);
    lines.extend([summon("happy_ghast 0 0 minecraft:spawn_adult", 0.5, -22.0, 8.5), "event entity @e[type=happy_ghast] minecraft:on_harnessed".into(), "effect @e[type=happy_ghast] slowness 30 255 true".into()]);
    lines.extend(["ride @s start_riding @e[type=happy_ghast,c=1] teleport_rider", "", "ride @s stop_riding", "", "kill @e[type=!player]", "tp @s 0.5 -26 0.5", ""].map(String::from));
    lines
}

/// Eight frogs on the open floor for 90 s: the waits from each landing to the next leap.
fn frogs() -> Vec<String> {
    let mut lines = start("noon", 0);
    for (x, z) in [(-6, -6), (-6, 6), (6, -6), (6, 6), (-3, 0), (3, 0), (0, -3), (0, 3)] {
        lines.push(summon("frog 0 0 spawn_temperate", mid(x), -26.0, mid(z)));
    }
    wait(&mut lines, 90);
    lines.extend(["kill @e[type=!player]".into(), String::new()]);
    lines
}

/// Six pairs of tamed horses fed golden carrots one after the other; each pair's foal.
fn foals() -> Vec<String> {
    let mut lines = start("noon", 0);
    lines.push("replaceitem entity @s slot.hotbar 0 golden_carrot 64".into());
    for _ in 0..6 {
        lines.extend(["kill @e[type=horse]".into(), String::new(), summon("horse 0 0 minecraft:spawn_tame_adult", 2.5, -26.0, 2.5), String::new(), "!interact horse".into()]);
        lines.extend([summon("horse 0 0 minecraft:spawn_tame_adult", 2.5, -26.0, -1.5), String::new(), "!interact horse".into()]);
        wait(&mut lines, 7);
    }
    lines.extend(["kill @e[type=!player]".into(), "clear @s".into(), String::new()]);
    lines
}

/// A saddled pig penned and boosted three times by a stick two uses from worn out; then one
/// falling from y 60, boosted a second after it is mounted.
fn stick() -> Vec<String> {
    let mut lines = start("noon", 0);
    lines.extend(["gamemode survival @s", "effect @s resistance 60 4 true"].map(String::from));
    pen(&mut lines, 3, 3, 2);
    lines.extend([summon("pig 0 0 minecraft:on_saddled", 3.5, -26.0, 3.5), "ride @s start_riding @e[type=pig,c=1] teleport_rider".into(), "replaceitem entity @s slot.hotbar 0 carrot_on_a_stick 1 21".into(), String::new()]);
    for _ in 0..3 {
        lines.push("!use".into());
        wait(&mut lines, 4);
    }
    lines.extend(["ride @s stop_riding", "kill @e[type=pig]", "replaceitem entity @s slot.hotbar 0 carrot_on_a_stick 1 0"].map(String::from));
    lines.extend([summon("pig 0 0 minecraft:on_saddled", -3.5, 60.0, -3.5), "ride @s start_riding @e[type=pig,c=1] teleport_rider".into(), "!use".into()]);
    wait(&mut lines, 4);
    lines.extend(["ride @s stop_riding", "kill @e[type=!player]", "tp @s 0.5 -26 0.5", "gamemode creative @s", "effect @s clear", "clear @s", "fill 2 -26 2 4 -24 4 air", ""].map(String::from));
    lines
}
