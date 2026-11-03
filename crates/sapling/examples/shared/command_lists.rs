//! bds_command_probe's own drills: the command lines and the packets each shows.
use std::time::Duration;

pub const GAMERULE: &[&str] = &[
    "gamerule",
    "gamerule dodaylightcycle",
    "gamerule dodaylightcycle false",
    "gamerule dodaylightcycle false",
    "gamerule DoDaylightCycle true",
    "gamerule randomtickspeed",
    "gamerule randomtickspeed 5",
    "gamerule randomtickspeed true",
    "gamerule randomtickspeed -3",
    "gamerule dodaylightcycle 3",
    "gamerule nosuchrule true",
    "gamerule spawnradius 2000000000",
    "gamerule playerssleepingpercentage 150",
    "gamerule playerssleepingpercentage -1",
    "gamerule playerssleepingpercentage 100",
    "gamerule randomtickspeed 1",
    "gamerule randomtickspeed 4097",
    "gamerule spawnradius 128",
    "gamerule spawnradius -1",
    "gamerule functioncommandlimit 0",
    "gamerule functioncommandlimit 10001",
    "gamerule maxcommandchainlength -1",
    "gamerule DoDaylightCycle",
    "gamerule playerwaypoints",
    "gamerule playerwaypoints banana",
    "gamerule playerwaypoints off",
    "gamerule playerwaypoints 0",
    "gamerule playerwaypoints everyone",
    "gamerule locatorbar",
    "gamerule locatorbar false",
    "gamerule locatorbar true",
    "gamerule dodaylightcycle true extra",
    "gamerule",
];

/// Mobs killed: the death, the loot and when they go; with `domobloot` off, no loot.
pub const KILL: &[&str] = &[
    "gamerule domobspawning false",
    "summon chicken ~2 ~ ~",
    "kill @e[type=chicken]",
    "summon pig ~-2 ~ ~",
    "kill @e[type=pig]",
    "summon armor_stand ~ ~ ~2",
    "kill @e[type=armor_stand]",
    "gamerule domobloot false",
    "summon chicken ~2 ~ ~",
    "kill @e[type=chicken]",
    "gamerule domobloot true",
];
/// Items put into a chest's slots, and the ways that fails.
pub const REPLACEITEM: &[&str] = &[
    "setblock ~2 ~ ~ chest",
    "replaceitem block ~2 ~ ~ slot.container 0 diamond 10",
    "replaceitem block ~2 ~ ~ slot.container 1 diamond",
    "replaceitem block ~2 ~ ~ slot.container 2 stone 70",
    "replaceitem block ~2 ~ ~ slot.container 27 diamond",
    "replaceitem block ~2 ~ ~ slot.container -1 diamond",
    "replaceitem block ~2 ~ ~ slot.container 3 no_such_item",
    "replaceitem block ~2 ~ ~ slot.container 0 air",
    "replaceitem block ~ ~-1 ~ slot.container 0 diamond",
    "replaceitem block ~2 ~ ~ slot.inventory 0 diamond",
];
pub const REPLACEITEM_SHOWN: &[&str] = &["InventorySlot", "InventoryContent", "BlockEntityData"];

/// Which state `/setblock` gives a block named without states.
pub const DEFAULTS: &[&str] = &[
    "setblock ~2 ~ ~ chest",
    "testforblock ~2 ~ ~ chest [\"minecraft:cardinal_direction\"=\"north\"]",
    "testforblock ~2 ~ ~ chest [\"minecraft:cardinal_direction\"=\"south\"]",
    "setblock ~3 ~ ~ furnace",
    "testforblock ~3 ~ ~ furnace [\"minecraft:cardinal_direction\"=\"north\"]",
    "testforblock ~3 ~ ~ furnace [\"minecraft:cardinal_direction\"=\"south\"]",
    "setblock ~4 ~ ~ oak_stairs",
    "testforblock ~4 ~ ~ oak_stairs [\"weirdo_direction\"=0]",
    "setblock ~5 ~ ~ hopper",
    "testforblock ~5 ~ ~ hopper [\"facing_direction\"=0]",
    // Blocks whose creative item records another state than their first.
    "setblock ~6 ~ ~ dropper",
    "testforblock ~6 ~ ~ dropper [\"facing_direction\"=3]",
    "setblock ~7 ~ ~ piston",
    "testforblock ~7 ~ ~ piston [\"facing_direction\"=1]",
    "setblock ~8 ~ ~ amethyst_cluster",
    "testforblock ~8 ~ ~ amethyst_cluster [\"minecraft:block_face\"=\"up\"]",
    "setblock ~9 ~ ~ brown_mushroom_block",
    "testforblock ~9 ~ ~ brown_mushroom_block [\"huge_mushroom_bits\"=14]",
    "setblock ~12 ~ ~ pointed_dripstone",
    "testforblock ~12 ~ ~ pointed_dripstone [\"hanging\"=true]",
    "setblock ~11 ~ ~ small_dripleaf_block",
    "testforblock ~11 ~ ~ small_dripleaf_block [\"upper_block_bit\"=true]",
    "testforblock ~11 ~ ~ small_dripleaf_block [\"minecraft:cardinal_direction\"=\"east\"]",
    "setblock ~12 ~ ~ stonecutter_block",
    "testforblock ~12 ~ ~ stonecutter_block [\"minecraft:cardinal_direction\"=\"north\"]",
    "setblock ~13 ~ ~ suspicious_sand",
    "testforblock ~13 ~ ~ suspicious_sand [\"hanging\"=true]",
];
pub const DEFAULTS_SHOWN: &[&str] = &["UpdateBlock"];

pub const KILL_SHOWN: &[&str] = &["EntityEvent", "AddEntity", "AddItemEntity", "RemoveEntity", "LevelSoundEvent", "SetEntityData", "Animate", "MobEffect"];
/// How long each line of the kill drill is watched: a dead mob goes a second after.
pub const KILL_SETTLE: Duration = Duration::from_millis(2500);
