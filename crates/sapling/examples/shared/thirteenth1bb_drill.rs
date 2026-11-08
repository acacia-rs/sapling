//! The thirteenth wave's group BB drills (research/parity/THIRTEENTH-BRIEF.md). On the flat ground
//! (stand at y -60): `/event entity` hitting, missing, unknown and with no targets, and a villager's
//! job and offers changed by it (`bb1event`). On a default world of seed 20261003 (BDS:
//! server.properties `level-type=DEFAULT`, `level-seed=20261003`; sapling: `SAPLING_DRILL_WORLD=generated
//! SAPLING_DRILL_SEED=20261003`): `/locate biome` of a spread of biomes from fixed places (`bb1biome`).

use crate::villages::wait;

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn events() -> Vec<String> {
    let mut out = owned([
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 1000", "tp @a 0.5 -60 0.5", "",
        "summon sheep 2.5 -60 4.5", "summon sheep 4.5 -60 4.5", "summon cow 6.5 -60 4.5", "",
        "event entity @e[type=sheep] minecraft:entity_born", "",
        "event entity @e[type=sheep] entity_born",
        "event entity @e[type=sheep] minecraft:spawn_baby",
        "event entity @e[type=sheep] spawn_baby",
        "event entity @e[type=!player] minecraft:temperate_color",
        "event entity @e[type=cow] minecraft:temperate_color",
        "event entity @e minecraft:entity_born",
        "event entity @e[type=sheep] nosuch_event",
        "event entity @e[type=zombie] minecraft:entity_born",
        "event entity nobody minecraft:entity_born",
        "event entity @s minecraft:gain_raid_omen",
        "event entity @s gain_raid_omen",
        "event entity @e[type=sheep]",
        "", "kill @e[type=!player]", "",
        // Within the actor's reach (a villager at 2.5 -60 4.5 was 4.07 off).
        "summon villager 0.5 -60 2.5 0 0 minecraft:become_farmer",
        "effect @e[type=villager_v2] slowness 1000 255 true", "",
        "!trade villager_v2 0 0", "",
        "event entity @e[type=villager_v2] minecraft:become_cartographer", "",
        "!trade villager_v2 0 0", "",
        "event entity @e[type=villager_v2] become_librarian", "",
        "!trade villager_v2 0 0", "",
        "event entity @e[type=villager_v2] minecraft:become_unskilled", "",
        "!trade villager_v2 0 0", "",
        "event entity @e[type=villager_v2] minecraft:become_sheperd", "",
        "!trade villager_v2 0 0", "",
        "event entity @e[type=villager_v2] minecraft:become_zombie", "",
        "kill @e[type=!player]", "",
    ]);
    wait(&mut out, 2);
    out
}

/// What `bb1biome` asks for, by BDS's names.
const BIOMES: [&str; 46] = [
    "plains", "sunflower_plains", "desert", "extreme_hills", "forest", "flower_forest", "birch_forest", "roofed_forest", "taiga", "mega_taiga",
    "cold_taiga", "swampland", "mangrove_swamp", "river", "frozen_river", "beach", "stone_beach", "jungle", "jungle_edge", "bamboo_jungle",
    "savanna", "savanna_plateau", "mesa", "mesa_bryce", "mushroom_island", "ice_plains", "ice_plains_spikes", "ocean", "deep_ocean", "warm_ocean",
    "lukewarm_ocean", "cold_ocean", "frozen_ocean", "meadow", "grove", "snowy_slopes", "jagged_peaks", "frozen_peaks", "stony_peaks", "cherry_grove",
    "pale_garden", "dappled_forest", "lush_caves", "dripstone_caves", "deep_dark", "sulfur_caves",
];

/// Where `bb1biome` asks from: the origin, AX's spawn (bds-ax1temple.txt's distances put BDS's
/// player there) and one far off.
const FROM: [[i32; 2]; 3] = [[0, 0], [22, 204], [-1000, 700]];

fn biomes() -> Vec<String> {
    let mut out = owned(["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 1000", ""]);
    for [x, z] in FROM {
        out.extend([format!("tp @a {x}.5 200 {z}.5"), String::new()]);
        // Sapling's searches take seconds each (a failed one ten); asked faster, half its answers never came.
        for biome in BIOMES {
            out.push(format!("locate biome minecraft:{biome}"));
            wait(&mut out, 3);
        }
    }
    wait(&mut out, 60);
    out
}

const EVENT_SHOWN: &[&str] = &["CommandOutput", "SetEntityData", "UpdateTrade", "AddEntity", "RemoveEntity"];
const BIOME_SHOWN: &[&str] = &["CommandOutput"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "bb1event" => (events(), EVENT_SHOWN),
        "bb1biome" => (biomes(), BIOME_SHOWN),
        _ => return None,
    })
}
