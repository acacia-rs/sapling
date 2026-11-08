//! The fourteenth wave's group M drills (research/parity/FOURTEENTH-BRIEF.md), on the flat ground at
//! midnight, the actor at 0.5 -60 0.5. `m3pick`: creative picks of blocks whose item has another name,
//! and of a filled chest with and without its data. `m3slots`: `/replaceitem` on a player's offhand,
//! ender chest and mob-only slots. `m3stew`: suspicious stews 13 and 17 to 19 eaten. `m3torch`: a bare
//! `/setblock torch` with no floor and walls on given sides. `m3mace`: `/damage ... mace_smash` on the
//! survival actor (bare, armoured, with a source), a zombie, then two deaths. `m3breath`: the survival
//! actor under water and out. `m3bucket`: an empty bucket filled with a full inventory. `m3axe`: copper
//! scraped and unwaxed by plain and Unbreaking III axes. `m3throw`: 64 eggs and 32 bottles o'
//! enchanting summoned at once, snowballs on weakened blazes. `m3witch`: witches drinking in water seen
//! by an actor coming back from far off. `m3far`: a zombie walking at the actor, the watcher at
//! growing distances. `m3place`: `/place jigsaw` depth limits and `includeEntities`.

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn start() -> Vec<String> {
    owned([
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule falldamage false", "weather clear", "time set 18000", "difficulty easy",
        "gamemode creative @a", "effect @a clear", "clear @a", "fill -12 -60 -12 30 -54 12 air", "fill -12 -63 -12 30 -62 12 dirt", "fill -12 -61 -12 30 -61 12 grass",
        "tp @s 0.5 -60 0.5", "tp CmdWatcher 0.5 -60 -6.5", "",
    ])
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

/// Each block (with what it stands on or by) picked two east of the feet; a chest with and without data.
fn pick() -> Vec<String> {
    let mut out = start();
    let cases: &[(&str, &[&str])] = &[
        ("torch [\"torch_facing_direction\"=\"west\"]", &["setblock 3 -60 0 stone"]),
        ("redstone_torch [\"torch_facing_direction\"=\"west\"]", &["setblock 3 -60 0 stone"]),
        ("wall_sign [\"facing_direction\"=4]", &["setblock 3 -60 0 stone"]),
        ("cocoa [\"direction\"=1]", &["setblock 3 -60 0 jungle_log"]),
        ("wheat [\"growth\"=7]", &["setblock 2 -61 0 farmland"]),
        ("carrots", &["setblock 2 -61 0 farmland"]),
        ("potatoes", &["setblock 2 -61 0 farmland"]),
        ("beetroot", &["setblock 2 -61 0 farmland"]),
        ("pumpkin_stem", &["setblock 2 -61 0 farmland"]),
        ("standing_sign", &[]),
        ("redstone_wire", &[]),
        ("unpowered_repeater", &[]),
        ("powered_comparator", &[]),
        ("lit_furnace", &[]),
        ("lit_redstone_lamp", &[]),
        ("unlit_redstone_torch", &[]),
        ("trip_wire", &[]),
        ("cake", &[]),
        ("flower_pot", &[]),
        ("sweet_berry_bush", &[]),
        ("cauldron", &[]),
        ("brewing_stand", &[]),
        ("wooden_door", &[]),
        ("frame", &[]),
        ("bamboo_sapling", &[]),
        ("daylight_detector_inverted", &[]),
        ("piston_arm_collision", &[]),
        ("fire", &[]),
    ];
    for (block, under) in cases {
        out.extend(owned(["fill 2 -61 0 3 -59 0 air", "setblock 2 -61 0 grass", "clear @s"]));
        out.extend(under.iter().map(|line| line.to_string()));
        out.extend([format!("setblock 2 -60 0 {block}"), "!pick 2 0 0 0".into(), String::new()]);
    }
    out.extend(owned(["fill 2 -61 0 3 -59 0 air", "setblock 2 -61 0 grass", "clear @s", "setblock 2 -60 0 chest", "replaceitem block 2 -60 0 slot.container 0 diamond 3", ""]));
    out.extend(owned(["!pick 2 0 0 1", "", "clear @s", "!pick 2 0 0 0", "", "clear @s", "setblock 2 -60 0 air", ""]));
    out
}

fn slots() -> Vec<String> {
    let mut out = start();
    for line in [
        "slot.weapon.offhand 0 shield", "slot.weapon.offhand 1 apple", "slot.enderchest 0 apple 5", "slot.enderchest 26 bread 2", "slot.enderchest 27 apple", "slot.enderchest -1 apple",
        "slot.saddle 0 saddle", "slot.armor 0 iron_horse_armor", "slot.armor.body 0 iron_horse_armor", "slot.chest 0 apple", "slot.equippable 0 apple", "slot.hotbar 9 apple", "slot.inventory 27 apple",
    ] {
        out.extend([format!("replaceitem entity @s {line}"), String::new()]);
    }
    out.extend(owned(["setblock 2 -60 0 ender_chest", "", "!click 2 0 0", "", "", "!close", "", "replaceitem entity @s slot.enderchest 1 stone 2", "", "!click 2 0 0", "", "", "!close", ""]));
    out.extend(owned(["setblock 2 -60 0 air", "clear @s", ""]));
    out
}

fn stews() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["gamemode survival @s", "effect @s hunger 10 200 true", "", "", "", "", "effect @s clear", ""]));
    for data in [13, 17, 18, 19] {
        out.extend([hold(&format!("suspicious_stew 1 {data}")), String::new(), "!eat".into(), String::new(), String::new(), String::new()]);
    }
    out.extend(owned(["effect @s clear", "gamemode creative @s", "clear @s", ""]));
    out
}

/// A bare torch set at 2 -59 0 (air under it) with stone on the sides named.
fn torches() -> Vec<String> {
    let mut out = start();
    let side = |name: &str| match name {
        "east" => "3 -59 0",
        "west" => "1 -59 0",
        "north" => "2 -59 -1",
        _ => "2 -59 1",
    };
    let cases: &[&[&str]] = &[&["east"], &["west"], &["north"], &["south"], &["east", "north"], &["north", "west"], &["west", "south"], &["south", "east"], &["east", "west"], &["north", "south"], &["east", "west", "north", "south"]];
    for walls in cases {
        out.push("fill 1 -59 -1 3 -59 1 air".into());
        out.extend(walls.iter().map(|wall| format!("setblock {} stone", side(wall))));
        out.extend(owned(["setblock 2 -59 0 torch", ""]));
    }
    out.extend(owned(["fill 1 -59 -1 3 -59 1 air", "setblock 2 -59 0 torch", "", "fill 1 -59 -1 3 -59 1 air", ""]));
    out
}

fn mace() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["gamemode survival @s", "tp CmdWatcher 3.5 -60 0.5", "", "damage @s 4 mace_smash", "", "effect @s instant_health 1 4 true", "", "damage @s 4 mace_smash entity CmdWatcher", ""]));
    out.extend(owned(["effect @s instant_health 1 4 true", "replaceitem entity @s slot.armor.chest 0 diamond_chestplate", "", "damage @s 4 mace_smash", "", "effect @s instant_health 1 4 true", "", "damage @s 4 entity_attack", ""]));
    out.extend(owned(["effect @s instant_health 1 4 true", "summon zombie 0.5 -60 4.5", "effect @e[type=zombie] slowness 1000 255 true", "", "damage @e[type=zombie] 4 mace_smash entity @s", "", "damage @e[type=zombie] 2 mace_smash", ""]));
    out.extend(owned(["gamemode survival CmdWatcher", "damage CmdWatcher 100 mace_smash", "", "", "damage @s 100 mace_smash entity @e[type=zombie]", "", "", ""]));
    out
}

fn breath() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["fill 2 -63 -2 6 -59 2 glass", "fill 3 -62 -1 5 -59 1 water", "gamemode survival @s", "", "tp @s 4.5 -62 0.5", ""]));
    wait(&mut out, 5);
    out.extend(owned(["tp @s 4.5 -59.5 0.5", "", "", "tp @s 0.5 -60 0.5", "", "", "tp @s 4.5 -62 0.5", "", "", "tp @s 0.5 -60 0.5", "", "", "gamemode creative @s", "fill 2 -63 -2 6 -59 2 air", ""]));
    out
}

/// A full inventory (stone in every slot) with two empty buckets in hand, then one, then two with a free slot.
fn buckets() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["gamemode survival @s", "setblock 2 -61 0 glass", "!look 2 0.3 0", ""]));
    for (count, free) in [(2, false), (1, false), (2, true)] {
        out.extend(owned(["clear @s", "give @s stone 2304", "setblock 2 -60 0 water", ""]));
        out.push(hold(&format!("bucket {count}")));
        if free {
            out.push("replaceitem entity @s slot.inventory 5 air".into());
        }
        out.extend(owned(["", "!use", "", "", "kill @e[type=item]", ""]));
    }
    out.extend(owned(["clear @s", "gamemode creative @s", "setblock 2 -60 0 air", ""]));
    out
}

fn axes() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["gamemode survival @s", ""]));
    for (block, enchanted, times) in [("exposed_copper", false, 4), ("exposed_copper", true, 24), ("waxed_copper", true, 12)] {
        out.push(hold("iron_axe"));
        if enchanted {
            out.push("enchant @s unbreaking 3".into());
        }
        out.push(String::new());
        for _ in 0..times {
            out.extend([format!("setblock 2 -60 0 {block}"), "!click 2 0 0".into()]);
        }
    }
    out.extend(owned(["", "clear @s", "gamemode creative @s", "setblock 2 -60 0 air", ""]));
    out
}

fn throws() -> Vec<String> {
    let mut out = start();
    let many = |kind: &str| format!("execute as @a as @a as @a as @a as @a run summon {kind} 20.5 -57 0.5");
    out.extend([many("egg"), many("egg")]);
    wait(&mut out, 3);
    out.extend(owned(["kill @e[type=chicken]", "kill @e[type=item]", ""]));
    out.push(many("xp_bottle"));
    wait(&mut out, 3);
    out.extend(owned(["kill @e[type=xp_orb]", ""]));
    // Blazes left 3 and 4 of 20, then one left 3 hit by a thrown snowball.
    for left in [3, 4] {
        out.extend(["summon blaze 20.5 -60 8.5".into(), "effect @e[type=blaze] slowness 1000 255 true".into(), format!("damage @e[type=blaze] {}", 20 - left), String::new()]);
        out.extend(owned(["summon snowball 20.5 -56 8.5", "", "", "summon snowball 20.5 -56 8.5", "", "", "kill @e[type=blaze]", ""]));
    }
    out.extend(owned(["summon blaze 6.5 -60 0.5", "effect @e[type=blaze] slowness 1000 255 true", "damage @e[type=blaze] 17", "", "!look 6 0.9 0"]));
    out.extend([hold("snowball 4"), String::new(), "!use".into(), String::new(), String::new(), "kill @e[type=blaze]".into(), "clear @s".into(), String::new()]);
    out
}

fn witches() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["tp @s 0.5 -60 400.5", "fill 8 -63 -2 14 -58 2 glass", "fill 9 -62 -1 13 -59 1 water", ""]));
    for x in [9, 11, 13] {
        out.push(format!("summon witch {x}.5 -62 0.5"));
    }
    out.push(String::new());
    for _ in 0..6 {
        out.extend(owned(["tp @s 0.5 -60 0.5", "", "tp @s 0.5 -60 400.5", ""]));
    }
    out.extend(owned(["tp @s 0.5 -60 0.5", "kill @e[type=witch]", "fill 8 -63 -2 14 -58 2 air", ""]));
    out
}

/// The zombie from 16 west of the warded survival actor, the watcher this far north of its path.
fn far() -> Vec<String> {
    let mut out = start();
    out.extend(owned(["gamemode survival @s", "effect @s resistance 1000 255 true", "fill -40 -61 -100 40 -61 12 grass", "fill -40 -60 -100 40 -54 12 air", ""]));
    for off in [4, 12, 20, 28, 40, 56, 72, 96] {
        out.extend(["kill @e[type=zombie]".into(), format!("tp CmdWatcher -7.5 -60 {}", 0.5 - off as f32), "summon zombie -15.5 -60 0.5".into()]);
        wait(&mut out, 5);
    }
    out.extend(owned(["kill @e[type=zombie]", "gamemode creative @s", "effect @s clear", ""]));
    out
}

fn place() -> Vec<String> {
    let mut out = start();
    for (depth, x) in [(0, 200), (1, 240), (20, 280), (21, 320)] {
        out.extend([format!("place jigsaw minecraft:trail_ruins/tower minecraft:road_anchor {depth} {x} -60 200"), String::new()]);
    }
    for (entities, x) in [("false", 400), ("true", 480)] {
        out.extend([format!("place jigsaw minecraft:village/plains/town_centers minecraft:bottom 6 {x} -60 0 false {entities}"), String::new(), String::new()]);
    }
    out.extend(owned(["place structure minecraft:trail_ruins 560 -60 0 true false true", "", "place structure minecraft:trail_ruins 640 -60 0 false false false", ""]));
    out
}

const SHOWN: &[&str] = &[
    "CommandOutput", "Text", "AddEntity", "AddItemEntity", "RemoveEntity", "SetEntityData", "EntityEvent", "SetEntityMotion", "LevelSoundEvent", "LevelEvent", "UpdateAttributes", "MobEffect",
    "UpdateBlock", "UpdateSubchunkBlocks", "InventorySlot", "InventoryContent", "PlayerHotbar", "MobEquipment", "MobArmorEquipment", "ItemStackResponse", "ContainerOpen", "Animate", "SetHealth",
];

const MOVES: &[&str] = &["CommandOutput", "AddEntity", "RemoveEntity", "MoveEntityDelta", "MoveEntityAbsolute", "SetEntityMotion"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "m3pick" => (pick(), SHOWN),
        "m3slots" => (slots(), SHOWN),
        "m3stew" => (stews(), SHOWN),
        "m3torch" => (torches(), SHOWN),
        "m3mace" => (mace(), SHOWN),
        "m3breath" => (breath(), SHOWN),
        "m3bucket" => (buckets(), SHOWN),
        "m3axe" => (axes(), SHOWN),
        "m3throw" => (throws(), SHOWN),
        "m3witch" => (witches(), SHOWN),
        "m3far" => (far(), MOVES),
        "m3place" => (place(), SHOWN),
        _ => return None,
    })
}
