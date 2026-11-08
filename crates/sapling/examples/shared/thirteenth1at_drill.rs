//! The thirteenth wave's group AT drills (research/parity/THIRTEENTH-BRIEF.md). On the flat ground
//! (stand at y -60; drill sapling with `SAPLING_DRILL_WORLD=lowflat`): blocks with several drops
//! broken by `setblock ... destroy`, by the actor's hand and asked of `/loot ... mine` (`at1drops`);
//! ten minutes of mob spawning on grass by day, the slimes counted (`at1slime`). On a default world
//! of seed 20261003 (BDS: server.properties `level-type=DEFAULT`, `level-seed=20261003`; sapling:
//! `SAPLING_DRILL_WORLD=generated SAPLING_DRILL_SEED=20261003`): a cartographer on a pad at y 150
//! traded up to its second tier, its explorer maps bought (`at1maps`).

use crate::villages::{every, wait};

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// Blocks broken at (10, -60, 0) over what they stand on, each so many times.
const DESTROYED: [(&str, &str, usize); 11] = [
    ("wheat [\"growth\"=7]", "farmland", 8),
    ("wheat [\"growth\"=3]", "farmland", 2),
    ("potatoes [\"growth\"=7]", "farmland", 4),
    ("carrots [\"growth\"=7]", "farmland", 4),
    ("beetroot [\"growth\"=7]", "farmland", 4),
    ("nether_wart [\"age\"=3]", "soul_sand", 4),
    ("sweet_berry_bush [\"growth\"=3]", "grass_block", 3),
    ("glowstone", "grass_block", 3),
    ("melon_block", "grass_block", 2),
    ("campfire", "grass_block", 1),
    ("bookshelf", "grass_block", 1),
];

fn drops() -> Vec<String> {
    let mut out = owned([
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 1000", "tp @a 0.5 -60 0.5",
        "fill -8 -61 -8 16 -61 8 grass_block", "fill -8 -60 -8 16 -55 8 air", "",
    ]);
    for (block, under, times) in DESTROYED {
        for _ in 0..times {
            out.extend([format!("setblock 10 -61 0 {under}"), format!("setblock 10 -60 0 {block}"), "setblock 10 -60 0 air destroy".into(), String::new()]);
        }
        out.push("kill @e[type=item]".into());
    }
    out.extend(owned(["gamemode survival @s", "clear @s", "effect @s resistance 1000 4 true", ""]));
    for _ in 0..4 {
        out.extend(owned(["setblock 1 -61 0 farmland", "setblock 1 -60 0 wheat [\"growth\"=7]", "!break 1 0 0", ""]));
    }
    out.extend(owned(["kill @e[type=item]", "gamemode creative @s", "effect @s clear", "setblock 12 -60 0 glowstone"]));
    for _ in 0..3 {
        out.push("loot spawn 12 -59 3 mine 12 -60 0".into());
    }
    out.extend(owned(["", "kill @e[type=item]", "setblock 12 -60 0 air", ""]));
    out
}

/// Grass at y -61 by day, mob spawning on, the slimes counted every 30 s for 600 s.
fn slime() -> Vec<String> {
    let mut out = owned([
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule doweathercycle false", "weather clear", "time set 1000", "difficulty easy",
        "tp @a 0.5 -60 0.5", "fill -24 -61 -24 24 -61 24 grass_block", "fill -24 -60 -24 24 -52 24 air",
        "gamemode survival @s", "clear @s", "effect @s resistance 1000 4 true", "effect @s saturation 1000 0 true", "gamerule domobspawning true", "",
    ]);
    every(&mut out, "testfor @e[type=slime]", 30, 20);
    out.extend(owned(["gamerule domobspawning false", "kill @e[type=!player]", "effect @s clear", "gamemode creative @s", ""]));
    out
}

/// A cartographer on a barrier pad at y 150 by the origin, traded paper to its second tier, its
/// four explorer maps bought; `/locate` of their structures first.
fn maps() -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 1000", "difficulty easy", "tp @a 0.5 151 0.5"]);
    wait(&mut out, 10);
    out.extend(owned([
        "fill -4 150 -4 4 150 4 barrier", "tp @a 0.5 151 0.5", "locate structure village", "locate structure monument", "locate structure trial_chambers", "locate structure mansion", "",
    ]));
    // Sapling generates the pad's chunks late; trades before they load time out.
    wait(&mut out, 20);
    out.extend(owned([
        "summon villager 0.5 151 2.5 0 0 minecraft:become_cartographer", "effect @e[type=villager_v2] slowness 1000 255 true", "gamemode survival @s", "clear @s",
        "effect @s resistance 1000 4 true", "give @s paper 64", "give @s paper 64", "give @s emerald 64", "give @s compass 8", "tp @s 0.5 151 0.5 facing 0.5 152 2.5", "",
        "!trade villager_v2 0 0", "", "!trade villager_v2 0 5",
    ]));
    wait(&mut out, 8);
    out.extend(owned(["!trade villager_v2 0 0", "", "!trade villager_v2 2 1", "", "!trade villager_v2 3 1", "", "!trade villager_v2 5 1", "", "!trade villager_v2 6 1", ""]));
    wait(&mut out, 5);
    out.extend(owned(["kill @e[type=!player]", "clear @s", "effect @s clear", "gamemode creative @s", ""]));
    out
}

const DROPS_SHOWN: &[&str] = &["AddItemEntity", "CommandOutput"];
const SLIME_SHOWN: &[&str] = &["AddEntity", "RemoveEntity"];
const MAPS_SHOWN: &[&str] = &["UpdateTrade", "InventorySlot", "InventoryContent", "ItemStackResponse", "ClientboundMapItemData", "CommandOutput", "ContainerOpen", "ContainerClose"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "at1drops" => (drops(), DROPS_SHOWN),
        "at1slime" => (slime(), SLIME_SHOWN),
        "at1maps" => (maps(), MAPS_SHOWN),
        _ => return None,
    })
}
