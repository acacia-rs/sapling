//! The structure-template, `/place`, `/locate`, `/tickingarea` and hardcore drills of
//! bds_command_probe (`structures`, `place`, `locate`, `ticking`, `hardcore`): each command as it
//! works and as it fails. Positions are absolute, so where the bots stand does not matter.
//! `hardcore-setup` runs on a fresh creative world; then level.dat's `IsHardcore` byte is set to 1,
//! the gamemode to survival, and `hardcore` runs on that world (KEEP_WORLD=1). `isHardcore=true` in
//! server.properties instead turns cheats off, and every command with them.
//! `place` wants `op-permission-level=4`.

use acacia_bot::proto::packets::{UpdateBlock, UpdateSubchunkBlocks};
use acacia_bot::proto::types::BlockUpdate;
use acacia_bot::proto::{Packet, RawPacket};
use acacia_world::BlockRegistry;

pub const STRUCTURE_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "AddEntity", "RemoveEntity"];
pub const PLACE_SHOWN: &[&str] = &["BlockEntityData", "AddEntity"];
pub const TICKING_SHOWN: &[&str] = &[];
pub const HARDCORE_SHOWN: &[&str] = &["SetPlayerGameType", "UpdatePlayerGameType", "Respawn", "SetDifficulty", "DeathInfo", "SetHealth", "GameRulesChanged"];

/// A 3x3x3 build with a block entity, states that turn and an armour stand, saved and loaded back
/// every way `/structure` takes.
const BUILD: &[&str] = &[
    "gamerule domobspawning false",
    "gamerule randomtickspeed 0",
    "tp @a 5 103 -10",
    "fill 0 98 0 80 112 40 air",
    "fill 4 99 4 6 99 6 stone",
    "setblock 5 100 4 chest [\"minecraft:cardinal_direction\"=\"east\"]",
    "replaceitem block 5 100 4 slot.container 0 diamond 5",
    "setblock 6 100 4 oak_stairs [\"weirdo_direction\"=0]",
    "setblock 4 100 5 oak_planks",
    "setblock 4 101 4 glass",
    "setblock 6 100 6 red_wool",
    "setblock 5 100 6 oak_log [\"pillar_axis\"=\"x\"]",
    "setblock 4 100 6 water",
    "summon armor_stand 5.5 100 5.5",
];

const SAVES: &[&str] = &[
    "structure save s1 4 99 4 6 101 6",
    "structure save s1 4 99 4 6 101 6 true disk true",
    "structure save s2 4 99 4 6 101 6 false memory false",
    "structure save s3 4 99 4 6 101 6 disk",
    "structure save mine:s4 4 99 4 6 101 6",
    "structure save \"bad name\" 4 99 4 6 101 6",
    "structure save BadCase 4 99 4 6 101 6",
    "structure save big 0 100 0 70 100 0",
    "structure save tall 0 -64 0 0 320 0",
    "structure save under 0 -70 0 0 -60 0",
    "structure save airy 4 110 4 5 111 5",
    "structure save airy2 4 110 4 5 111 5 true memory false",
    "structure save airy3 4 110 4 5 111 5 false memory false",
];

const LOADS: &[&str] = &[
    "structure load s1 10 99 4",
    "testforblocks 4 99 4 6 101 6 10 99 4",
    "structure load s1 16 99 4 90_degrees",
    "structure load s1 22 99 4 180_degrees",
    "structure load s1 28 99 4 270_degrees",
    "structure load s1 34 99 4 0_degrees x",
    "structure load s1 40 99 4 0_degrees z",
    "structure load s1 46 99 4 90_degrees xz",
    "structure load s2 52 99 4",
    "structure load s1 58 99 4 0_degrees none false true false 50 abc",
    "structure load s1 64 99 4 0_degrees none false true false 50 abc",
    "structure load s1 70 99 4 0_degrees none true false",
    "structure load s1 4 99 12 0_degrees none false true true",
    "structure load s1 10 99 12 0_degrees none layer_by_layer 2",
    "structure load s1 16 99 12 0_degrees none block_by_block 0 true true",
    "structure load s1 22 99 12 0_degrees none false true false 0",
    "structure load s1 28 99 12 0_degrees none false true false 150",
    "structure load nosuch 10 99 20",
    "structure load s1 10 400 20",
    "structure load s1 10 -70 20",
    "structure load mystructure:s1 34 99 12",
    "structure load s1 40 99 12 0_degrees none false false",
    "structure delete s2",
    "structure delete s2",
    "structure delete nosuch",
    "structure delete mystructure:s3",
    "structure load s3 46 99 12",
];

pub fn structures() -> Vec<String> {
    [BUILD, SAVES, LOADS].concat().iter().map(|line| line.to_string()).collect()
}

const PLACE: &[&str] = &[
    "tp @a 200 120 200",
    "place feature minecraft:oak_tree_feature 210 ~ 200",
    "place feature oak_tree_feature 220 ~ 200",
    "place feature minecraft:no_such_feature 230 ~ 200",
    "place feature minecraft:oak_tree_feature 210 400 200",
    "place featurerule minecraft:overworld_surface_oak_tree 240 ~ 200",
    "place featurerule minecraft:no_such_rule 240 ~ 200",
    "place structure minecraft:trail_ruins 200 ~ 260",
    "place structure minecraft:abandoned_camp_forest 260 ~ 200",
    "place structure minecraft:trial_chambers 200 ~ 140",
    "place structure minecraft:village 140 ~ 200",
    "place structure minecraft:no_such_structure 200 ~ 200",
    "place structure minecraft:trail_ruins 200 ~ 320 true false false",
    "place jigsaw minecraft:trail_ruins/tower minecraft:bottom 3 320 ~ 200",
    "place jigsaw minecraft:no_such_pool minecraft:bottom 3 320 ~ 200",
    "place jigsaw minecraft:trail_ruins/tower minecraft:bottom 30 320 ~ 260",
    "place jigsaw minecraft:trail_ruins/tower minecraft:road_anchor 3 340 ~ 200",
    "place jigsaw minecraft:trail_ruins/tower minecraft:road_anchor 0 340 ~ 260",
    "place jigsaw minecraft:village/plains/town_centers minecraft:bottom 2 380 ~ 200",
    "place structure minecraft:trail_ruins 9000 ~ 9000",
];

pub fn place() -> Vec<String> {
    PLACE.iter().map(|line| line.to_string()).collect()
}

const LOCATE: &[&str] = &[
    "tp @a 0 120 0",
    "locate structure village",
    "locate structure minecraft:village",
    "locate structure stronghold",
    "locate structure mineshaft",
    "locate structure monument",
    "locate structure ruins",
    "locate structure shipwreck",
    "locate structure ruined_portal",
    "locate structure ancient_city",
    "locate structure trial_chambers",
    "locate structure trail_ruins",
    "locate structure pillager_outpost",
    "locate structure mansion",
    "locate structure temple",
    "locate structure buried_treasure",
    "locate structure abandoned_camp",
    "locate structure fortress",
    "locate structure endcity",
    "locate structure bastion_remnant",
    "locate structure village true",
    "locate structure no_such",
    "locate biome plains",
    "locate biome minecraft:plains",
    "locate biome minecraft:desert",
    "locate biome minecraft:mushroom_island",
    "locate biome minecraft:warm_ocean",
    "locate biome minecraft:deep_dark",
    "locate biome minecraft:hell",
    "locate biome minecraft:the_end",
    "locate biome minecraft:no_such",
    "tp @a 3000 120 -2000",
    "locate structure village",
    "locate structure monument",
    "locate biome minecraft:jungle",
    "locate biome minecraft:cherry_grove",
];

pub fn locate() -> Vec<String> {
    LOCATE.iter().map(|line| line.to_string()).collect()
}

const TICKING: &[&str] = &[
    "tp @a 0 120 0",
    "tickingarea list",
    "tickingarea add 0 0 0 40 0 40 first",
    "tickingarea add 0 0 0 40 0 40 first",
    "tickingarea add circle 100 0 100 2 round true",
    "tickingarea add 0 0 0 200 0 200 huge",
    "tickingarea add circle 0 0 0 5 wide",
    "tickingarea add circle 0 0 0 0 tiny",
    "tickingarea add 500 0 500 510 0 510",
    "tickingarea list",
    "tickingarea list all-dimensions",
    "tickingarea preload first",
    "tickingarea preload first true",
    "tickingarea preload 5 0 5",
    "tickingarea preload 5000 0 5000",
    "tickingarea remove first",
    "tickingarea remove first",
    "tickingarea remove 100 0 100",
    "tickingarea add 1000 0 0 1000 0 0 a1",
    "tickingarea add 1100 0 0 1100 0 0 a2",
    "tickingarea add 1200 0 0 1200 0 0 a3",
    "tickingarea add 1300 0 0 1300 0 0 a4",
    "tickingarea add 1400 0 0 1400 0 0 a5",
    "tickingarea add 1500 0 0 1500 0 0 a6",
    "tickingarea add 1600 0 0 1600 0 0 a7",
    "tickingarea add 1700 0 0 1700 0 0 a8",
    "tickingarea add 1800 0 0 1800 0 0 a9",
    "tickingarea add 1900 0 0 1900 0 0 a10",
    "tickingarea add 2000 0 0 2000 0 0 a11",
    "tickingarea list",
    "tickingarea remove_all",
    "tickingarea list",
    "tickingarea remove nothing",
];

pub fn ticking() -> Vec<String> {
    TICKING.iter().map(|line| line.to_string()).collect()
}

/// On a world with cheats, before `IsHardcore` is set in its level.dat: both bots left standing in
/// lava, in creative, so that they burn when they come back to it in survival.
const HARDCORE_SETUP: &[&str] = &[
    "gamerule spawnradius 0",
    "setworldspawn 0 -60 0",
    "fill -3 -63 -3 3 -58 3 lava",
    "tp @a 0.5 -60 0.5",
];

/// On the hardcore world: the deaths at the start, then which commands it still takes.
const HARDCORE: &[&str] = &[
    "", "", "", "", "",
    "difficulty", "difficulty easy", "gamemode survival @s", "kill @s", "say hi", "me hi", "tell CmdWatcher hi", "msg CmdWatcher hi",
    "tellraw @a {\"rawtext\":[{\"text\":\"hi\"}]}", "title @s title hi", "titleraw @s title {\"rawtext\":[{\"text\":\"hi\"}]}", "list", "help",
    "seed", "version", "setmaxplayers 10", "kick nobody", "op CmdWatcher", "deop CmdWatcher", "allowlist list", "time set day",
    "time query daytime", "weather clear", "toggledownfall", "gamerule", "give @s stone", "clear @s", "tp @s ~ ~ ~", "setblock ~ ~5 ~ stone",
    "fill ~ ~5 ~ ~ ~5 ~ stone", "clone ~ ~5 ~ ~ ~5 ~ ~1 ~5 ~", "testforblock ~ ~5 ~ stone", "testforblocks ~ ~5 ~ ~ ~5 ~ ~1 ~5 ~",
    "testfor @s", "summon pig", "effect @s speed", "enchant @s sharpness", "xp 1", "spawnpoint", "setworldspawn", "clearspawnpoint",
    "spreadplayers 0 0 1 2 @s", "defaultgamemode survival", "playsound random.click", "stopsound @s", "particle minecraft:heart ~ ~ ~",
    "scoreboard objectives list", "tag @s list", "replaceitem entity @s slot.hotbar 0 stone", "damage @s 1", "execute as @s run say hi",
    "structure delete x", "tickingarea list", "locate structure village", "place feature minecraft:oak_tree_feature", "transfer", "", "",
];

pub fn hardcore(setup: bool) -> Vec<String> {
    let lines = if setup { HARDCORE_SETUP } else { HARDCORE };
    lines.iter().map(|line| line.to_string()).collect()
}

/// An UpdateBlock or UpdateSubchunkBlocks with its blocks named (`x y z name[states]`).
pub fn block_names(packet: &RawPacket) -> Option<String> {
    let registry = BlockRegistry::vanilla();
    let name = |hash: u32| registry.runtime_id_from_hash(hash).and_then(|id| registry.get(id)).map_or("?".into(), |s| format!("{}[{}]", s.name.trim_start_matches("minecraft:"), s.properties));
    let cells = |blocks: &[BlockUpdate]| blocks.iter().map(|b| format!("{} {} {} {}", b.position.x, b.position.y, b.position.z, name(b.runtime_id))).collect::<Vec<_>>().join("; ");
    if packet.id == UpdateBlock::ID {
        let update = packet.decode::<UpdateBlock>().ok()?;
        let p = &update.position;
        return Some(format!("UpdateBlock {} {} {} layer {} {}", p.x, p.y, p.z, update.layer, name(update.block_runtime_id)));
    }
    if packet.id == UpdateSubchunkBlocks::ID {
        let update = packet.decode::<UpdateSubchunkBlocks>().ok()?;
        return Some(format!("UpdateSubchunkBlocks {} | extra: {}", cells(&update.blocks), cells(&update.extra)));
    }
    None
}

/// The drills of this file, by name.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "structures" => (structures(), STRUCTURE_SHOWN),
        "place" => (place(), PLACE_SHOWN),
        "locate" => (locate(), TICKING_SHOWN),
        "ticking" => (ticking(), TICKING_SHOWN),
        "hardcore-setup" => (hardcore(true), HARDCORE_SHOWN),
        "hardcore" => (hardcore(false), HARDCORE_SHOWN),
        _ => return None,
    })
}
