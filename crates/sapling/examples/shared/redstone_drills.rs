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
pub const REDSTONE_SHOWN: &[&str] = &["UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "LevelSoundEvent", "LevelEvent", "BlockEvent"];

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
    lines.extend((0..layers).map(|layer| format!("fill ~2 ~{} ~4 ~22 ~{0} ~16 stone", (layer * LAYER) as i32 - 1)));
    for (index, block) in ASKED.iter().copied().enumerate() {
        let (x, floor) = (2 + 2 * (index % COLUMNS), LAYER * (index / COLUMNS));
        let mut put = |y: usize, z: usize, block: &str| lines.push(format!("setblock ~{x} ~{} ~{z} {block}", floor + y));
        for (y, z, what) in [(0, 5, "redstone_wire"), (0, 6, block), (0, 7, "redstone_lamp")] {
            put(y, z, what);
        }
        for (y, z, what) in [(0, 10, block), (1, 10, "redstone_wire"), (0, 11, "redstone_wire")] {
            put(y, z, what);
        }
        for (y, z, what) in [(0, 14, "redstone_wire"), (1, 14, block), (0, 15, "stone"), (1, 15, "redstone_wire")] {
            put(y, z, what);
        }
        for (y, z) in [(0, 4), (1, 9), (0, 13)] {
            put(y, z, "redstone_block");
        }
    }
    lines.extend(std::iter::repeat_n("testforblock ~ ~-1 ~ stone".to_owned(), 4));
    lines
}

const LEVER_UNDER: &str = "lever [\"lever_direction\"=\"down_east_west\",\"open_bit\"=true]";
const DUST: &str = "redstone_wire";
const LAMP: &str = "redstone_lamp";
const POWER: &str = "redstone_block";
/// Rigs asking where dust powers what is beside it (research C8), as blocks at east, up, south of
/// the dust asked about; a rig's source comes last.
const SHAPES: &[&[(i32, i32, i32, &str)]] = &[
    // 0: one dust by a power block.
    &[(0, 0, 0, DUST), (1, 0, 0, LAMP), (0, 0, -1, LAMP), (0, 0, 1, LAMP), (-1, 0, 0, POWER)],
    // 1: the end of a line.
    &[(-1, 0, 0, DUST), (0, 0, 0, DUST), (1, 0, 0, LAMP), (0, 0, -1, LAMP), (0, 0, 1, LAMP), (-2, 0, 0, POWER)],
    // 2: a corner, and the end past it.
    &[(-1, 0, 0, DUST), (0, 0, 0, DUST), (0, 0, 1, DUST), (1, 0, 0, LAMP), (0, 0, -1, LAMP), (0, 0, 2, LAMP), (1, 0, 1, LAMP), (-2, 0, 0, POWER)],
    // 3: a T.
    &[(-1, 0, 0, DUST), (0, 0, 0, DUST), (1, 0, 0, DUST), (0, 0, 1, DUST), (0, 0, -1, LAMP), (2, 0, 0, LAMP), (0, 0, 2, LAMP), (-2, 0, 0, POWER)],
    // 4: a line fed at its middle from the side.
    &[(-1, 0, 0, DUST), (0, 0, 0, DUST), (1, 0, 0, DUST), (-2, 0, 0, LAMP), (2, 0, 0, LAMP), (0, 0, 1, LAMP), (0, 0, -1, POWER)],
    // 5: a dot on a block a lever under it powers.
    &[(0, 1, 0, "stone"), (0, 2, 0, DUST), (1, 2, 0, LAMP), (-1, 2, 0, LAMP), (0, 2, 1, LAMP), (0, 2, -1, LAMP), (0, 0, 0, LEVER_UNDER)],
    // 6: as 5 with dust on a block to its south.
    &[
        (0, 1, 0, "stone"),
        (0, 1, 1, "stone"),
        (0, 2, 0, DUST),
        (0, 2, 1, DUST),
        (1, 2, 0, LAMP),
        (-1, 2, 0, LAMP),
        (0, 2, -1, LAMP),
        (0, 2, 2, LAMP),
        (1, 2, 1, LAMP),
        (0, 0, 0, LEVER_UNDER),
    ],
    // 7: dust a step up to the east, and a lamp straight on.
    &[(0, 0, 0, DUST), (1, 0, 0, "stone"), (1, 1, 0, DUST), (0, 0, 1, LAMP), (0, 0, -1, POWER)],
    // 8: as 7 with the step cut by stone.
    &[(0, 0, 0, DUST), (1, 0, 0, "stone"), (1, 1, 0, DUST), (0, 1, 0, "stone"), (0, 0, 1, LAMP), (0, 0, -1, POWER)],
    // 9: as 8 with glass.
    &[(0, 0, 0, DUST), (1, 0, 0, "stone"), (1, 1, 0, DUST), (0, 1, 0, "glass"), (0, 0, 1, LAMP), (0, 0, -1, POWER)],
];
const SHAPE_LAYER: i32 = 5;

/// The rigs of [`SHAPES`], two to a layer, the dust asked about at x 7 and 17, z 10.
pub fn shapes() -> Vec<String> {
    let layers = SHAPES.len().div_ceil(2) as i32;
    let mut lines = vec![format!("fill ~2 ~-1 ~4 ~22 ~{} ~16 air", layers * SHAPE_LAYER)];
    lines.extend((0..layers).map(|layer| format!("fill ~2 ~{} ~4 ~22 ~{0} ~16 stone", layer * SHAPE_LAYER - 1)));
    for (index, rig) in SHAPES.iter().enumerate() {
        let (x, floor) = (7 + 10 * (index as i32 % 2), SHAPE_LAYER * (index as i32 / 2));
        lines.extend(rig.iter().map(|(east, up, south, block)| format!("setblock ~{} ~{} ~{} {block}", x + east, floor + up, 10 + south)));
    }
    lines.extend(std::iter::repeat_n("testforblock ~ ~-1 ~ stone".to_owned(), 4));
    lines
}

/// A torch that puts itself out (research C10): on the side of a block, a block over it powering
/// dust on the first, a lamp past the dust. Left to burn out, then blocks put beside it and taken away.
pub const BURNOUT: &[&str] = &[
    "fill ~2 ~-1 ~4 ~12 ~-1 ~12 stone",
    "fill ~2 ~ ~4 ~12 ~4 ~12 air",
    "setblock ~6 ~ ~8 stone",
    "setblock ~7 ~1 ~8 stone",
    "setblock ~6 ~1 ~8 redstone_wire",
    "setblock ~5 ~1 ~8 redstone_lamp",
    "setblock ~7 ~ ~8 redstone_torch [\"torch_facing_direction\"=\"west\"]",
    "testforblock ~7 ~ ~8 redstone_torch",
    "testforblock ~7 ~ ~8 unlit_redstone_torch",
    "setblock ~8 ~ ~8 stone",
    "setblock ~8 ~ ~8 air",
    "testforblock ~7 ~ ~8 unlit_redstone_torch",
    "testforblock ~7 ~ ~8 unlit_redstone_torch",
    "setblock ~7 ~ ~9 stone",
    "setblock ~7 ~ ~9 air",
    "testforblock ~7 ~ ~8 unlit_redstone_torch",
];

/// Which way a repeater passes power: four lines eastward (power block, dust, repeater, dust,
/// lamp), a repeater of each `minecraft:cardinal_direction`.
pub fn repeater_facing() -> Vec<String> {
    let mut lines = vec!["fill ~2 ~-1 ~4 ~22 ~-1 ~16 stone".to_owned(), "fill ~2 ~ ~4 ~22 ~4 ~16 air".to_owned()];
    for (row, facing) in ["north", "east", "south", "west"].iter().enumerate() {
        let z = 5 + 2 * row;
        lines.push(format!("setblock ~5 ~ ~{z} redstone_wire"));
        lines.push(format!("setblock ~6 ~ ~{z} unpowered_repeater [\"minecraft:cardinal_direction\"=\"{facing}\",\"repeater_delay\"=0]"));
        lines.push(format!("setblock ~7 ~ ~{z} redstone_wire"));
        lines.push(format!("setblock ~8 ~ ~{z} redstone_lamp"));
        lines.push(format!("setblock ~4 ~ ~{z} redstone_block"));
        lines.push(format!("setblock ~4 ~ ~{z} air"));
    }
    lines
}

fn repeater(input: &str, delay: u8) -> String {
    format!("unpowered_repeater [\"minecraft:cardinal_direction\"=\"{input}\",\"repeater_delay\"={delay}]")
}

/// Repeaters, a rig to a floor (y 0, 5, 10, 15, 20), each taking power on its west and passing it east
/// unless said (research/captures/bds-redstone-repeaters.txt):
/// - 0: one of each delay between dust, a power block put and taken;
/// - 5: one of each delay fed by a torch that puts itself out, 2 ticks on and 2 off;
/// - 10: one into a block with dust on and beside it; one between a power block and a lamp with no dust;
///   one behind a block that dust powers;
/// - 15: dust along the side of one; one locked by another pointing into its side;
/// - 20: one after 15 dust.
pub fn repeaters() -> Vec<String> {
    let mut lines = vec!["fill ~2 ~-1 ~4 ~22 ~24 ~16 air".to_owned()];
    lines.extend((0..5).map(|floor| format!("fill ~2 ~{} ~4 ~22 ~{0} ~16 stone", floor * 5 - 1)));
    let mut put = |x: i32, y: i32, z: i32, block: &str| lines.push(format!("setblock ~{x} ~{y} ~{z} {block}"));
    let toggle = |put: &mut dyn FnMut(i32, i32, i32, &str), x: i32, y: i32, z: i32| {
        put(x, y, z, "redstone_block");
        put(x, y, z, "air");
    };
    for delay in 0..4u8 {
        let z = 5 + 2 * i32::from(delay);
        for (x, block) in [(5, DUST.to_owned()), (6, repeater("west", delay)), (7, DUST.to_owned()), (8, LAMP.to_owned())] {
            put(x, 0, z, &block);
        }
        toggle(&mut put, 4, 0, z);
    }
    for delay in 0..4u8 {
        let x = 4 + 5 * i32::from(delay);
        for (dx, dy, block) in [(-2, 0, "stone".to_owned()), (-1, 0, "stone".to_owned()), (0, 0, "stone".to_owned()), (1, 1, "stone".to_owned())] {
            put(x + dx, 5 + dy, 8, &block);
        }
        for (dx, block) in [(-2, DUST.to_owned()), (-1, repeater("east", delay)), (0, DUST.to_owned())] {
            put(x + dx, 6, 8, &block);
        }
        put(x + 1, 5, 8, "redstone_torch [\"torch_facing_direction\"=\"west\"]");
    }
    for (x, y, z, block) in [(5, 10, 6, DUST), (7, 10, 6, "stone"), (8, 10, 6, LAMP), (7, 11, 6, DUST), (7, 10, 5, DUST)] {
        put(x, y, z, block);
    }
    put(6, 10, 6, &repeater("west", 0));
    toggle(&mut put, 4, 10, 6);
    put(5, 10, 9, &repeater("west", 0));
    put(6, 10, 9, LAMP);
    toggle(&mut put, 4, 10, 9);
    for (x, block) in [(5, DUST.to_owned()), (6, "stone".to_owned()), (7, repeater("west", 0)), (8, LAMP.to_owned())] {
        put(x, 10, 12, &block);
    }
    toggle(&mut put, 4, 10, 12);

    put(6, 15, 6, &repeater("west", 0));
    put(7, 15, 6, LAMP);
    put(6, 15, 5, DUST);
    toggle(&mut put, 5, 15, 5);
    for (x, z, block) in [(5, 11, DUST.to_owned()), (6, 11, repeater("west", 0)), (7, 11, LAMP.to_owned()), (6, 10, repeater("north", 0))] {
        put(x, 15, z, &block);
    }
    // Fed, locked, unfed, unlocked; then locked, fed, unlocked, unfed.
    for (x, z, block) in [(4, 11, POWER), (6, 9, POWER), (4, 11, "air"), (6, 9, "air"), (6, 9, POWER), (4, 11, POWER), (6, 9, "air"), (4, 11, "air")] {
        put(x, 15, z, block);
    }

    for x in 3..=17 {
        put(x, 20, 6, DUST);
    }
    for (x, block) in [(18, repeater("west", 0)), (19, DUST.to_owned()), (20, LAMP.to_owned())] {
        put(x, 20, 6, &block);
    }
    toggle(&mut put, 2, 20, 6);
    lines
}

fn comparator(input: &str, subtracts: bool) -> String {
    format!("unpowered_comparator [\"minecraft:cardinal_direction\"=\"{input}\",\"output_subtract_bit\"={subtracts}]")
}

/// Comparators, a rig to a floor, each with its back to the west:
/// - 0: between dust, a power block put and taken; after 8 dust; a power block straight behind;
/// - 5: 11 behind it, then 13 at its north side put and taken, then 9 at its south side;
/// - 10: the same, subtracting;
/// - 15: a chest behind it filled a slot at a time; a chest behind a block behind it.
pub fn comparators() -> Vec<String> {
    let mut lines = vec!["fill ~2 ~-1 ~4 ~22 ~24 ~16 air".to_owned()];
    lines.extend((0..4).map(|floor| format!("fill ~2 ~{} ~4 ~22 ~{0} ~16 stone", floor * 5 - 1)));
    let mut put = |x: i32, y: i32, z: i32, block: &str| lines.push(format!("setblock ~{x} ~{y} ~{z} {block}"));
    let compares = comparator("west", false);
    for (x, z, block) in [(5, 5, DUST), (6, 5, &compares), (7, 5, DUST), (8, 5, DUST), (9, 5, LAMP), (4, 5, POWER), (4, 5, "air")] {
        put(x, 0, z, block);
    }
    for x in 3..=10 {
        put(x, 0, 8, DUST);
    }
    for (x, z, block) in [(11, 8, compares.as_str()), (12, 8, DUST), (13, 8, DUST), (2, 8, POWER), (2, 8, "air"), (6, 11, &compares), (7, 11, DUST), (5, 11, POWER), (5, 11, "air")] {
        put(x, 0, z, block);
    }
    for (y, subtracts) in [(5, false), (10, true)] {
        for x in 3..=7 {
            put(x, y, 8, DUST);
        }
        put(8, y, 8, &comparator("west", subtracts));
        put(9, y, 8, DUST);
        for z in (5..=7).chain(9..=15) {
            put(8, y, z, DUST);
        }
        for (x, z, block) in [(2, 8, POWER), (8, 4, POWER), (8, 4, "air"), (8, 16, POWER), (8, 16, "air"), (2, 8, "air")] {
            put(x, y, z, block);
        }
    }
    for (x, z, block) in [(5, 5, "chest"), (6, 5, compares.as_str()), (7, 5, DUST), (5, 9, "chest"), (6, 9, "stone"), (7, 9, &compares), (8, 9, DUST)] {
        put(x, 15, z, block);
    }
    lines.extend((0..27).map(|slot| format!("replaceitem block ~5 ~15 ~5 slot.container {slot} stone 64")));
    lines.push("replaceitem block ~5 ~15 ~9 slot.container 0 stone 64".to_owned());
    lines.push("setblock ~5 ~15 ~5 air".to_owned());
    lines
}
