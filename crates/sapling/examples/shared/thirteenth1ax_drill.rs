//! The thirteenth wave's group AX drills (research/parity/THIRTEENTH-BRIEF.md). On the flat ground
//! (stand at y -60): forty ripe beetroots broken by `setblock ... destroy` (`ax1beet`). On a default
//! world of seed 20261003 (BDS: server.properties `level-type=DEFAULT`, `level-seed=20261003`;
//! sapling: `SAPLING_DRILL_WORLD=generated SAPLING_DRILL_SEED=20261003`): AT's cartographer with
//! every map pixel shown, and `/locate` of the temples and their biomes (`ax1maps`); cartographers over
//! the temple, their tier-1 explorer maps bought (`ax1temple`); the same in a swamp (`ax1swamp`),
//! there summoned plain and made cartographers by `/event` (`ax1swamp2`, eight of them `ax1swamp3`); eight in a jungle (`ax1jungle`).

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

const BEETROOTS: usize = 40;

fn beetroots() -> Vec<String> {
    let mut out = owned([
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 1000", "tp @a 0.5 -60 0.5",
        "fill -8 -61 -8 16 -61 8 grass_block", "fill -8 -60 -8 16 -55 8 air", "setblock 10 -61 0 farmland", "",
    ]);
    for at in 0..BEETROOTS {
        out.extend(["setblock 10 -60 0 beetroot [\"growth\"=7]".into(), "setblock 10 -60 0 air destroy".into(), String::new()]);
        if at % 5 == 4 {
            out.push("kill @e[type=item]".into());
        }
    }
    out
}

fn maps() -> Vec<String> {
    let mut out = owned([
        "locate structure jungle_temple", "locate structure swamp_hut", "locate structure temple", "locate structure desert_pyramid", "locate biome jungle", "locate biome swamp", "",
    ]);
    out.extend(super::thirteenth1at::drill("at1maps").map(|(lines, _)| lines).unwrap_or_default());
    out
}

/// Where BDS's `/locate structure temple` answered (bds-ax1maps.txt).
const TEMPLE: [i32; 2] = [680, -424];
/// Where BDS's `/locate biome minecraft:swampland` answered (bds-ax1temple.txt).
const SWAMP: [i32; 2] = [3509, -2067];
/// Where it answered `minecraft:jungle`.
const JUNGLE: [i32; 2] = [-2635, -1427];

/// `/locate biome` of the temples' biomes, then `cartographers` cartographers in turn on a barrier pad at y 150
/// over `[x, z]`, each traded paper to its second tier and its tier-1 explorer maps (offers 2 and 3)
/// bought (bds-ax1temple, ax1swamp and ax1swamp2.txt were taken before the paper, so only their
/// offers show). Each is summoned by
/// `become_cartographer`, or when `spawned` summoned plain and made a cartographer by `/event`.
fn temples([x, z]: [i32; 2], spawned: bool, cartographers: usize) -> Vec<String> {
    let mut out = owned([
        "kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "time set 1000", "difficulty easy", "locate biome minecraft:jungle",
        "locate biome minecraft:swampland", "locate biome minecraft:desert",
    ]);
    out.extend([format!("tp @a {x}.5 151 {z}.5"), String::new()]);
    crate::villages::wait(&mut out, 10);
    out.extend([format!("fill {} 150 {} {} 150 {} barrier", x - 4, z - 4, x + 4, z + 4), format!("tp @a {x}.5 151 {z}.5"), "locate structure temple".into(), String::new()]);
    crate::villages::wait(&mut out, 20);
    out.extend(owned(["gamemode survival @s", "clear @s", "effect @s resistance 1000 4 true", "give @s emerald 64", "give @s emerald 64", "give @s compass 8"]));
    out.push(format!("tp @s {x}.5 151 {z}.5 facing {x}.5 152 {}.5", z + 2));
    for _ in 0..cartographers {
        // Summoned by `become_cartographer`, BDS's cartographers kept the plains type (bds-ax1swamp.txt).
        let summon = match spawned {
            true => [format!("summon villager {x}.5 151 {}.5", z + 2), "event entity @e[type=villager_v2] minecraft:become_cartographer".into()],
            false => [format!("summon villager {x}.5 151 {}.5 0 0 minecraft:become_cartographer", z + 2), String::new()],
        };
        out.extend(summon.into_iter().filter(|line| !line.is_empty()));
        out.extend(["effect @e[type=villager_v2] slowness 1000 255 true".into(), String::new()]);
        out.extend(owned(["give @s paper 64", "give @s paper 64", "!trade villager_v2 0 0", "", "!trade villager_v2 0 5"]));
        crate::villages::wait(&mut out, 8);
        out.extend(owned(["!trade villager_v2 0 0", "", "!trade villager_v2 2 1", "", "!trade villager_v2 3 1", ""]));
        crate::villages::wait(&mut out, 5);
        out.push("kill @e[type=villager_v2]".into());
    }
    out.extend(owned(["kill @e[type=!player]", "clear @s", "effect @s clear", "gamemode creative @s", ""]));
    out
}

const BEET_SHOWN: &[&str] = &["AddItemEntity", "CommandOutput"];
const MAPS_SHOWN: &[&str] = &["UpdateTrade", "InventorySlot", "InventoryContent", "ItemStackResponse", "ClientboundMapItemData", "MapPixels", "CommandOutput", "ContainerOpen", "ContainerClose"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "ax1beet" => (beetroots(), BEET_SHOWN),
        "ax1maps" => (maps(), MAPS_SHOWN),
        "ax1temple" => (temples(TEMPLE, false, 3), MAPS_SHOWN),
        "ax1swamp" => (temples(SWAMP, false, 3), MAPS_SHOWN),
        "ax1swamp2" => (temples(SWAMP, true, 3), MAPS_SHOWN),
        "ax1swamp3" => (temples(SWAMP, true, 8), MAPS_SHOWN),
        "ax1jungle" => (temples(JUNGLE, true, 8), MAPS_SHOWN),
        _ => return None,
    })
}
