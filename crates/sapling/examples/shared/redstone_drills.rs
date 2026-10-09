//! The redstone drills of bds_command_probe (docs/research/redstone.md).

use std::time::Duration;

/// Redstone on a stone stage: a lamp by a power block, dust from a power block to a lamp 15 away
/// and 16 away, a lever powering through the block it is on, a torch put out by power into its
/// block, a door and a hopper powered, and pressed buttons left to spring back.
pub const REDSTONE: &[&str] = &[
    "fill ~2 ~-1 ~4 ~22 ~-1 ~16 stone",
    "fill ~2 ~ ~4 ~22 ~4 ~16 air",
    "setblock ~3 ~ ~5 redstone_lamp",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "fill ~3 ~ ~7 ~17 ~ ~7 redstone_wire",
    "setblock ~18 ~ ~7 redstone_lamp",
    "fill ~3 ~ ~9 ~18 ~ ~9 redstone_wire",
    "setblock ~19 ~ ~9 redstone_lamp",
    "setblock ~2 ~ ~7 redstone_block",
    "setblock ~2 ~ ~9 redstone_block",
    "setblock ~2 ~ ~7 air",
    "setblock ~3 ~ ~12 stone",
    "setblock ~4 ~ ~12 redstone_lamp",
    "setblock ~3 ~1 ~12 lever [\"lever_direction\"=\"up_north_south\",\"open_bit\"=true]",
    "setblock ~3 ~1 ~12 lever [\"lever_direction\"=\"up_north_south\",\"open_bit\"=false]",
    "setblock ~7 ~ ~12 stone",
    "setblock ~7 ~1 ~12 redstone_torch",
    "setblock ~7 ~2 ~12 redstone_lamp",
    "setblock ~6 ~ ~12 redstone_block",
    "setblock ~6 ~ ~12 air",
    "setblock ~10 ~ ~12 iron_door",
    "setblock ~11 ~ ~12 redstone_block",
    "setblock ~11 ~ ~12 air",
    "setblock ~13 ~ ~12 hopper",
    "setblock ~14 ~ ~12 redstone_block",
    "setblock ~14 ~ ~12 air",
    "setblock ~16 ~ ~12 stone",
    "setblock ~17 ~ ~12 redstone_lamp",
    "setblock ~16 ~1 ~12 stone_button [\"facing_direction\"=1,\"button_pressed_bit\"=true]",
    "setblock ~16 ~1 ~12 wooden_button [\"facing_direction\"=1,\"button_pressed_bit\"=true]",
    "testforblock ~16 ~1 ~12 wooden_button [\"button_pressed_bit\"=true]",
];
/// A block of redstone put by a lamp, an iron door and a hopper and taken away again, 20 times,
/// [`PHASE_SETTLE`] apart so the toggles fall on both ticks of the 2-tick evaluation (research C1).
pub const REDSTONE_PHASE: &[&str] = &[
    "fill ~2 ~-1 ~4 ~6 ~-1 ~6 stone",
    "fill ~2 ~ ~4 ~6 ~2 ~6 air",
    "setblock ~3 ~ ~5 redstone_lamp",
    "setblock ~5 ~ ~5 iron_door",
    "setblock ~4 ~ ~6 hopper",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
    "setblock ~4 ~ ~5 redstone_block",
    "setblock ~4 ~ ~5 air",
];
/// 10.5 ticks: a lamp is out 6 after its power goes, and each toggle lands half a tick later in
/// the 2-tick cycle than the one before.
pub const PHASE_SETTLE: Duration = Duration::from_millis(525);
pub const REDSTONE_SHOWN: &[&str] = &[
    "UpdateBlock",
    "UpdateBlockSynced",
    "UpdateSubchunkBlocks",
    "LevelSoundEvent",
    "LevelEvent",
    "BlockEvent",
];

/// Blocks asked whether they pass power (research C5), a column each.
const ASKED: &[&str] = &[
    "stone",
    "glass",
    "white_stained_glass",
    "tinted_glass",
    "glowstone",
    "sea_lantern",
    "blue_ice",
    "packed_ice",
    "oak_leaves",
    "azalea_leaves",
    "smooth_stone_slab",
    "smooth_stone_slab [\"minecraft:vertical_half\"=\"top\"]",
    "smooth_stone_double_slab",
    "oak_stairs",
    "oak_stairs [\"upside_down_bit\"=true]",
    "tnt",
    "piston",
    "sticky_piston",
    "observer",
    "dispenser",
    "dropper",
    "noteblock",
    "jukebox",
    "target",
    "slime",
    "honey_block",
    "mob_spawner",
    "barrier",
    "soul_sand",
    "farmland",
    "grass_path",
    "mud",
    "chest",
    "ender_chest",
    "furnace",
    "crafting_table",
    "enchanting_table",
    "bookshelf",
    "chiseled_bookshelf",
    "dirt",
    "grass_block",
    "sand",
    "gravel",
    "obsidian",
    "bedrock",
    "cauldron",
    "composter",
    "beacon",
    "snow",
    "snow_layer",
    "white_carpet",
    "scaffolding",
    "melon_block",
    "pumpkin",
    "lit_pumpkin",
    "hay_block",
    "sponge",
    "undyed_shulker_box",
    "anvil",
    "magma",
    "copper_grate",
    "mangrove_roots",
    "moss_block",
    "redstone_ore",
    "iron_block",
    "oak_planks",
    "oak_log",
    "white_wool",
    "white_concrete_powder",
    "cactus",
    "cobblestone_wall",
    "oak_fence",
    "iron_bars",
    "glass_pane",
    "brewing_stand",
    "lectern",
    "stonecutter_block",
    "grindstone",
    "bell",
    "daylight_detector",
];
const COLUMNS: usize = 10;
/// Layers of columns are stacked this far apart: a wider stage leaves the loaded chunks and `/fill` refuses it.
const LAYER: usize = 4;
pub const CONDUCTORS_SETTLE: Duration = Duration::from_millis(250);

/// For each of [`ASKED`], in a column of its own, three rigs along z:
/// - 4..=7: a power block, dust pointing into the block, a lamp behind it: lit when it conducts;
/// - 9..=11: a power block beside dust on the block, dust at its foot: powered when dust steps down it;
/// - 13..=15: a power block, dust under the block, dust a step up beside it: powered unless the block cuts the step.
pub fn conductors() -> Vec<String> {
    let layers = ASKED.len().div_ceil(COLUMNS);
    let mut lines = vec![format!("fill ~2 ~-1 ~4 ~22 ~{} ~16 air", layers * LAYER)];
    lines.extend((0..layers).map(|layer| {
        format!(
            "fill ~2 ~{} ~4 ~22 ~{0} ~16 stone",
            (layer * LAYER) as i32 - 1
        )
    }));
    for (index, block) in ASKED.iter().copied().enumerate() {
        let (x, floor) = (2 + 2 * (index % COLUMNS), LAYER * (index / COLUMNS));
        let mut put = |y: usize, z: usize, block: &str| {
            lines.push(format!("setblock ~{x} ~{} ~{z} {block}", floor + y))
        };
        for (y, z, what) in [
            (0, 5, "redstone_wire"),
            (0, 6, block),
            (0, 7, "redstone_lamp"),
        ] {
            put(y, z, what);
        }
        for (y, z, what) in [
            (0, 10, block),
            (1, 10, "redstone_wire"),
            (0, 11, "redstone_wire"),
        ] {
            put(y, z, what);
        }
        for (y, z, what) in [
            (0, 14, "redstone_wire"),
            (1, 14, block),
            (0, 15, "stone"),
            (1, 15, "redstone_wire"),
        ] {
            put(y, z, what);
        }
        for (y, z) in [(0, 4), (1, 9), (0, 13)] {
            put(y, z, "redstone_block");
        }
    }
    lines.extend(std::iter::repeat_n(
        "testforblock ~ ~-1 ~ stone".to_owned(),
        4,
    ));
    lines
}
