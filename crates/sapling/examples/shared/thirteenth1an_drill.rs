//! The thirteenth wave's group AN drills (research/parity/THIRTEENTH-BRIEF.md), on the flat ground
//! (stand at y -60; drill sapling with `SAPLING_DRILL_WORLD=lowflat`): still zombies at set
//! distances from free villagers away from any village (`an1fear`); a village of eight beds and
//! eight villagers with a baby, mob spawning on, watched 21 minutes for cats and the baby growing
//! up (`an1grow`); a farmer by ripe wheat (`an1farm`); a cartographer traded up to its second tier
//! (`an1maps`).

use crate::villages::{every, village, wait};

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// The villager's distance from its zombie, per pair.
const DISTANCES: [f32; 6] = [2.5, 3.0, 3.5, 4.0, 5.0, 6.0];
const PAIRS: usize = 12;

/// Pair `i`'s zombie cell: three columns 16 apart from x 300, four rows 16 apart from z 0.
fn zombie_at(i: usize) -> [i32; 2] {
    [300 + 16 * (i % 3) as i32, 16 * (i / 3) as i32]
}

/// Whether each pair's villager is still within its distance and 1.5 of its zombie, each second for 12 s.
fn count_pairs(out: &mut Vec<String>) {
    for _ in 0..12 {
        for i in 0..PAIRS {
            let [x, z] = zombie_at(i);
            let r = DISTANCES[i % DISTANCES.len()] + 1.5;
            out.push(format!("execute at @e[type=zombie,x={x}.5,y=-60,z={z}.5,r=1] run testfor @e[type=villager_v2,r={r}]"));
        }
        out.push(String::new());
    }
}

/// Twelve still zombies, each with an unhurt villager `DISTANCES` east of it, twice over.
fn fear() -> Vec<String> {
    let mut out = owned([
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule doweathercycle false", "weather clear", "time set 1000", "difficulty easy",
        "tp @a 316.5 -60 -8.5", "fill 292 -61 -12 340 -61 60 grass_block", "fill 292 -60 -12 340 -56 60 air", "",
    ]);
    for i in 0..PAIRS {
        let [x, z] = zombie_at(i);
        out.push(format!("summon zombie {x}.5 -60 {z}.5"));
    }
    out.extend(owned(["effect @e[type=zombie] slowness 1000 255 true", "effect @e[type=zombie] fire_resistance 1000 0 true", ""]));
    for _ in 0..2 {
        for i in 0..PAIRS {
            let [x, z] = zombie_at(i);
            out.push(format!("summon villager {} -60 {z}.5 0 0 minecraft:become_unskilled", x as f32 + 0.5 + DISTANCES[i % DISTANCES.len()]));
        }
        out.push("effect @e[type=villager_v2] resistance 1000 4 true".into());
        count_pairs(&mut out);
        out.extend(owned(["kill @e[type=villager_v2]", ""]));
    }
    out.extend(owned(["kill @e[type=!player]", ""]));
    out
}

/// Eight beds and a bell on cleared grass, eight unskilled villagers and a baby, mob spawning on;
/// the cats and villagers counted every 30 s for 21 minutes.
fn grow() -> Vec<String> {
    let mut out = owned([
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "gamerule doweathercycle false", "weather clear", "time set 1000", "difficulty easy",
        "tp @a 0.5 -60 0.5", "fill -24 -61 -24 24 -61 24 grass_block", "fill -24 -60 -24 24 -52 24 air", "",
    ]);
    for i in 0..8 {
        out.push(format!("setblock {} -60 8 bed [\"direction\"=0]", -14 + 2 * i));
    }
    out.extend(owned(["setblock 4 -60 8 bell", "gamemode survival @s", "clear @s", "effect @s resistance 1000 4 true", "effect @s saturation 1000 0 true", ""]));
    for i in 0..8 {
        out.push(format!("summon villager {}.5 -60 4.5 0 0 minecraft:become_unskilled", -14 + 2 * i));
    }
    out.push("summon villager 0.5 -60 -4.5 0 0 minecraft:entity_born".into());
    wait(&mut out, 10);
    out.extend(owned(["effect @e[type=villager_v2] resistance 1000 4 true", "gamerule domobspawning true", ""]));
    for _ in 0..42 {
        out.extend(owned(["testfor @e[type=cat]", "testfor @e[type=villager_v2]"]));
        wait(&mut out, 30);
    }
    out.extend(owned(["gamerule domobspawning false", "kill @e[type=!player]", "effect @s clear", "gamemode creative @s", ""]));
    out
}

/// A farmer by its composter and a 7 by 7 field of ripe wheat round a water cell, given seeds,
/// watched 150 s.
fn farm() -> Vec<String> {
    let mut out = village(&["setblock 6 -60 4 composter", "summon villager 4.5 -60 2.5 0 0 minecraft:become_farmer"]);
    out.extend(owned([
        "fill 4 -61 -10 10 -61 -4 farmland [\"moisturized_amount\"=7]", "setblock 7 -61 -7 water", "fill 4 -60 -10 10 -60 -4 wheat [\"growth\"=7]", "setblock 7 -60 -7 air",
        "replaceitem entity @e[type=villager_v2] slot.inventory 0 wheat_seeds 16", "",
    ]));
    every(&mut out, "testfor @e[type=villager_v2,x=7,y=-60,z=-7,r=5]", 10, 15);
    out.extend(owned(["kill @e[type=!player]", "effect @s clear", "gamemode creative @s", ""]));
    out
}

/// A cartographer by its table, traded paper until its second tier, its offers read after.
fn maps() -> Vec<String> {
    let mut out = village(&["setblock 6 -60 4 cartography_table", "summon villager 4.5 -60 4.5 0 0 minecraft:become_cartographer"]);
    out.extend(owned([
        "effect @e[type=villager_v2] slowness 1000 255 true", "give @s paper 64", "give @s paper 64", "give @s emerald 64", "give @s compass 4",
        "tp @s 4.5 -60 2.5 facing 4.5 -59 4.5", "", "!trade villager_v2 0 0", "", "!trade villager_v2 0 5",
    ]));
    wait(&mut out, 8);
    out.extend(owned(["!trade villager_v2 0 0", "", "!trade villager_v2 2 1", "", "!trade villager_v2 3 1", ""]));
    wait(&mut out, 3);
    out.extend(owned(["kill @e[type=!player]", "clear @s", "effect @s clear", "gamemode creative @s", ""]));
    out
}

const FEAR_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "MoveEntityDelta", "SetEntityData", "EntityEvent"];
const GROW_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "SetEntityData", "EntityEvent", "LevelSoundEvent", "UpdateAttributes"];
const FARM_SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "AddItemEntity", "TakeItemEntity", "UpdateBlock", "LevelEvent", "LevelSoundEvent", "MoveEntityDelta", "MobEquipment", "EntityEvent"];
const MAPS_SHOWN: &[&str] = &["UpdateTrade", "InventorySlot", "InventoryContent", "ItemStackResponse", "ClientboundMapItemData", "SetEntityData", "ContainerOpen", "ContainerClose"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "an1fear" => (fear(), FEAR_SHOWN),
        "an1grow" => (grow(), GROW_SHOWN),
        "an1farm" => (farm(), FARM_SHOWN),
        "an1maps" => (maps(), MAPS_SHOWN),
        _ => return None,
    })
}
