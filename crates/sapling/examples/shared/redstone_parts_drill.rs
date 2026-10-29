//! The redstone drill pass of bds_command_probe (docs/research/redstone.md "Not drilled"): `rsparts`
//! (conductors, power blocks, doors, buttons, comparators, repeater locks), `rsreads` (what a comparator
//! reads, clicks), `rsplace` (dust and inverted detectors put by hand), `rsobserve` (observers),
//! `rspistons` (what pistons move, short pulses, slime) and `rsdevices` (bulbs, targets, note blocks,
//! dispensers, crafters, rods, jukeboxes). Built at absolute cells on the flat world; the actor stands
//! at [`ACTOR`] for the clicks.

pub const SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateBlockSynced", "UpdateSubchunkBlocks", "LevelSoundEvent", "LevelEvent", "BlockEvent", "BlockEntityData", "AddEntity", "RemoveEntity", "AddItemEntity",
    "InventorySlot",
];

/// The floor's top; the stage is the layer over it.
const Y: i32 = -60;
/// Where the actor stands for clicks (its feet).
pub const ACTOR: [i32; 3] = [10, Y, 10];
const LAMP: &str = "redstone_lamp";
const DUST: &str = "redstone_wire";
const POWER: &str = "redstone_block";

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "rsparts" => parts(),
            "rsreads" => reads(),
            "rsplace" => placed(),
            "rsobserve" => observe(),
            "rspistons" => pistons(),
            "rsdevices" => devices(),
            _ => return None,
        },
        SHOWN,
    ))
}

pub fn set(x: i32, y: i32, z: i32, block: &str) -> String {
    format!("setblock {x} {y} {z} {block}")
}

/// A stone floor under air over x and z, the actor at [`ACTOR`] in creative with nothing held.
pub fn stage(x: [i32; 2], z: [i32; 2]) -> Vec<String> {
    let mut lines = vec!["gamerule domobspawning false".to_owned(), "kill @e[type=!player]".to_owned()];
    lines.push(format!("fill {} {} {} {} {} {} stone", x[0], Y - 1, z[0], x[1], Y - 1, z[1]));
    lines.push(format!("fill {} {Y} {} {} {} {} air", x[0], z[0], x[1], Y + 5, z[1]));
    lines.extend(["gamemode creative @s".to_owned(), "clear @s".to_owned()]);
    lines.push(format!("tp @s {}.5 {Y} {}.5", ACTOR[0], ACTOR[2]));
    lines.push(String::new());
    lines
}

fn comparator(input_from: &str, subtract: bool) -> String {
    format!("unpowered_comparator [\"minecraft:cardinal_direction\"=\"{input_from}\",\"output_subtract_bit\"={subtract}]")
}

fn repeater(input_from: &str, delay: u8) -> String {
    format!("unpowered_repeater [\"minecraft:cardinal_direction\"=\"{input_from}\",\"repeater_delay\"={delay}]")
}

/// Conductors (ice, pistons), a power block beside stone beside a lamp, a lever-held block with dust,
/// doors powered at one half, buttons under a falling arrow, comparators with equal and stronger
/// sides, and a repeater held by a comparator.
fn parts() -> Vec<String> {
    let mut lines = stage([18, 60], [18, 50]);
    // C5: dust pointing into each block, a lamp past it, the dust fed and starved.
    for (i, block) in ["ice", "packed_ice", "blue_ice", "piston [\"facing_direction\"=1]", "sticky_piston [\"facing_direction\"=1]"].iter().enumerate() {
        let x = 22 + 5 * i as i32;
        lines.extend([set(x, Y, 22, DUST), set(x + 1, Y, 22, block), set(x + 2, Y, 22, LAMP), set(x - 1, Y, 22, POWER), set(x - 1, Y, 22, "air")]);
    }
    // C7: a power block beside stone beside a lamp.
    lines.extend([set(24, Y, 25, LAMP), set(23, Y, 25, "stone"), set(22, Y, 25, POWER), set(22, Y, 25, "air")]);
    // C9: stone with a lever on it, dust leading off east to a lamp, dust pointing into it from the west.
    lines.extend([set(24, Y, 28, "stone"), set(23, Y, 28, DUST), set(22, Y, 28, LAMP)]);
    lines.extend((25..=27).map(|x| set(x, Y, 28, DUST)));
    lines.push(set(28, Y, 28, LAMP));
    lines.push(set(24, Y + 1, 28, "lever [\"lever_direction\"=\"up_north_south\",\"open_bit\"=true]"));
    lines.push(String::new());
    lines.push(set(24, Y + 1, 28, "lever [\"lever_direction\"=\"up_north_south\",\"open_bit\"=false]"));
    // C12: an iron door and a wooden door powered at the upper half, a wooden one at the lower.
    for (x, door, half) in [(24, "iron_door", 1), (28, "wooden_door", 1), (32, "wooden_door", 0)] {
        lines.extend([set(x, Y, 31, door), String::new(), set(x + 1, Y + half, 31, POWER), String::new(), set(x + 1, Y + half, 31, "air")]);
    }
    // C13: a wooden and a stone button on stone, a lamp beside each, an arrow dropped on each.
    for (x, button) in [(24, "wooden_button"), (28, "stone_button")] {
        lines.extend([set(x, Y, 34, "stone"), set(x + 1, Y, 34, LAMP), set(x, Y + 1, 34, &format!("{button} [\"facing_direction\"=1]"))]);
        lines.push(format!("summon arrow {x}.5 {} 34.5", Y + 4));
        lines.extend([String::new(), String::new(), String::new()]);
    }
    lines.extend(["kill @e[type=arrow]".to_owned(), String::new(), String::new()]);
    // Comparators: 11 behind (dust 21..25 from a power block at 20), a side fed to 11 (dust z 42..38),
    // then 15 at the side from a power block, a repeater at the side, the subtracting mode.
    lines.extend((21..=25).map(|x| set(x, Y, 37, DUST)));
    lines.extend([set(26, Y, 37, &comparator("west", false)), set(27, Y, 37, DUST), set(28, Y, 37, LAMP)]);
    lines.extend((38..=42).map(|z| set(26, Y, z, DUST)));
    lines.extend([set(20, Y, 37, POWER), String::new(), set(26, Y, 43, POWER), String::new()]);
    lines.extend([set(26, Y, 37, &comparator("west", true)), String::new(), set(26, Y, 37, &comparator("west", false))]);
    lines.extend([set(26, Y, 43, "air"), set(26, Y, 38, POWER), String::new(), set(26, Y, 38, &repeater("south", 0)), set(26, Y, 39, POWER), String::new()]);
    lines.extend([set(26, Y, 39, "air"), set(20, Y, 37, "air"), String::new()]);
    // A repeater held by a lit comparator at its side, its input put on, the comparator let go.
    lines.extend([set(40, Y, 46, &repeater("west", 0)), set(41, Y, 46, LAMP), set(40, Y, 45, &comparator("north", false)), set(40, Y, 44, POWER), String::new()]);
    lines.extend([set(39, Y, 46, POWER), String::new(), set(40, Y, 44, "air"), String::new(), String::new()]);
    lines
}

/// Comparators behind a lectern given a book, a wall frame turned, cauldrons of lava and powder
/// snow, a chiseled bookshelf given books; a comparator's and a repeater's click.
fn reads() -> Vec<String> {
    let mut lines = stage([2, 20], [2, 20]);
    let hold = |item: &str| format!("replaceitem entity @s slot.weapon.mainhand 0 {item}");
    // Lectern east of the actor; comparator and dust past it.
    lines.extend([set(12, Y, 10, "lectern [\"minecraft:cardinal_direction\"=\"west\"]"), set(13, Y, 10, &comparator("west", false)), set(14, Y, 10, DUST)]);
    lines.extend([hold("writable_book"), "!click 2 0 0".to_owned(), String::new()]);
    // A frame on the north face of stone south of the actor, the comparator past the stone.
    lines.extend([set(10, Y, 12, "stone"), set(10, Y, 11, "frame [\"facing_direction\"=2]"), set(10, Y, 13, &comparator("north", false)), set(10, Y, 14, DUST)]);
    lines.extend([hold("diamond"), "!click 0 0 1".to_owned(), String::new(), "!click 0 0 1".to_owned(), String::new(), "!click 0 0 1".to_owned(), String::new()]);
    // Cauldrons of lava and powder snow.
    for (z, liquid) in [(16, "lava"), (18, "powder_snow")] {
        lines.extend([set(16, Y, z, &comparator("west", false)), set(17, Y, z, DUST)]);
        lines.extend((0..=6).step_by(3).map(|level| set(15, Y, z, &format!("cauldron [\"cauldron_liquid\"=\"{liquid}\",\"fill_level\"={level}]"))));
    }
    // A chiseled bookshelf west of the actor facing it, given two books.
    lines.extend([set(8, Y, 10, "chiseled_bookshelf [\"direction\"=3]"), set(7, Y, 10, &comparator("east", false)), set(6, Y, 10, DUST)]);
    lines.extend([hold("book"), "!click -2 0 0 east".to_owned(), String::new(), "!click -2 0 0 east".to_owned(), String::new()]);
    // Clicks on a comparator and a repeater north of the actor.
    lines.extend([set(10, Y, 8, &comparator("south", false)), set(12, Y, 8, &repeater("south", 0)), "clear @s".to_owned(), String::new()]);
    lines.extend(["!click 0 0 -2".to_owned(), String::new(), "!click 2 0 -2".to_owned(), String::new()]);
    lines
}

/// Dust put by hand beside a power block, then a second beside it; a daylight detector put by
/// hand, turned inside out by a click, read through the day.
fn placed() -> Vec<String> {
    let mut lines = stage([2, 20], [2, 20]);
    lines.extend(["gamerule dodaylightcycle false".to_owned(), "time set 6000".to_owned(), set(12, Y, 10, POWER)]);
    lines.extend(["replaceitem entity @s slot.weapon.mainhand 0 redstone".to_owned(), "!place 1 -1 0".to_owned(), String::new(), "!place 1 -1 -1".to_owned(), String::new()]);
    lines.extend([set(8, Y, 10, DUST), "replaceitem entity @s slot.weapon.mainhand 0 daylight_detector".to_owned(), "!place -1 -1 0".to_owned(), String::new(), "clear @s".to_owned()]);
    lines.extend(["!click -1 0 0".to_owned(), String::new(), String::new()]);
    for time in [0, 6000, 12000, 13000, 18000, 23000] {
        lines.extend([format!("time set {time}"), String::new(), String::new()]);
    }
    lines.push("time set 6000".to_owned());
    lines
}

fn observer(watching: &str) -> String {
    format!("observer [\"minecraft:facing_direction\"=\"{watching}\"]")
}

/// An observer watching the cell a piston pushes a block into (two changes a tick apart); an
/// observer pushed by a piston; an observer watching a wooden pressure plate a chicken lands on.
fn observe() -> Vec<String> {
    let mut lines = stage([18, 50], [18, 40]);
    // Piston at x 20 pushing east a stone at 21 into 22, which the observer at 22,y+1 watches below.
    lines.extend([set(20, Y, 22, "piston [\"facing_direction\"=4]"), set(21, Y, 22, "stone"), set(22, Y + 1, 22, &observer("down")), set(22, Y + 2, 22, LAMP)]);
    lines.extend([set(19, Y, 22, POWER), String::new(), set(19, Y, 22, "air"), String::new()]);
    // An observer watching west, pushed east by a piston; a lamp east of where it lands.
    lines.extend([set(20, Y, 26, "piston [\"facing_direction\"=4]"), set(21, Y, 26, &observer("west")), set(23, Y, 26, LAMP)]);
    lines.extend([set(19, Y, 26, POWER), String::new(), set(19, Y, 26, "air"), String::new()]);
    // An observer watching a plate; a chicken dropped on it.
    lines.extend([set(30, Y, 30, "wooden_pressure_plate"), set(30, Y, 31, &observer("north")), set(30, Y, 32, LAMP)]);
    lines.extend([format!("summon chicken 30.5 {} 30.5", Y + 2), String::new(), String::new(), String::new(), "kill @e[type=chicken]".to_owned(), String::new(), String::new()]);
    lines
}

/// Pistons pushing east (at x 20) into one block each, a power block put on top and taken; a
/// piston put beside a power block; pistons given a 2-tick observer pulse; slime pushed up with
/// stone on every side, a torch on its side; a sticky piston whose slime holds 12 stones.
fn pistons() -> Vec<String> {
    let mut lines = stage([16, 40], [16, 64]);
    let blocks = [
        "furnace", "chest", "barrel", "ender_chest", "enchanting_table", "anvil", "beacon", "lectern", "crafting_table", "jukebox", "noteblock", "bell", "hopper", "dispenser", "ladder [\"facing_direction\"=3]",
        "carpet", "snow_layer", "scaffolding", "bamboo", "oak_leaves", "undyed_shulker_box", "conduit", "respawn_anchor", "lodestone", "crying_obsidian", "ancient_debris", "reinforced_deepslate", "mob_spawner",
        "beehive", "end_portal_frame", "pumpkin", "melon_block", "web", "flower_pot", "cactus", "dragon_egg", "end_rod", "white_glazed_terracotta", "glass", "grindstone", "heavy_core", "vault", "trial_spawner",
    ];
    // A piston's `facing_direction` 4 pushes east (5 pushed west on BDS). Two rows keep z within the
    // chunks BDS keeps loaded around the actor.
    let east = "piston [\"facing_direction\"=4]";
    for (i, block) in blocks.iter().enumerate() {
        let (x, z) = (16 + 4 * (i as i32 / 22), 18 + 2 * (i as i32 % 22));
        lines.extend([set(x, Y, z, east), set(x + 1, Y, z, block), set(x, Y + 1, z, POWER), String::new(), set(x, Y + 1, z, "air")]);
    }
    // A piston put beside a power block already there.
    lines.extend([set(30, Y + 1, 18, POWER), set(30, Y, 18, east), String::new(), set(30, Y + 1, 18, "air")]);
    // Plain and sticky pistons fed a 2-tick pulse by an observer behind them, a stone in front.
    for (z, piston) in [(22, "piston"), (26, "sticky_piston")] {
        lines.extend([set(30, Y, z, &format!("{piston} [\"facing_direction\"=4]")), set(31, Y, z, "stone"), set(29, Y, z, &observer("west")), set(28, Y, z, "stone"), String::new(), String::new()]);
    }
    // Slime on a piston pushing up, a stone at each side, a torch on its east side over the stone.
    lines.extend([set(34, Y, 30, "sticky_piston [\"facing_direction\"=1]"), set(34, Y + 1, 30, "slime")]);
    lines.extend([set(34, Y + 1, 29, "stone"), set(33, Y + 1, 30, "stone"), set(35, Y + 1, 30, "stone"), set(34, Y + 1, 31, "stone"), set(34, Y + 2, 31, "redstone_torch")]);
    lines.extend([set(33, Y + 2, 30, "torch [\"torch_facing_direction\"=\"west\"]"), set(34, Y + 2, 30, "brown_mushroom")]);
    lines.extend([set(35, Y, 30, POWER), String::new(), set(35, Y, 30, "air"), String::new()]);
    // A sticky piston at x 30 facing east whose slime (x 32 once out) holds 12 stones on its south.
    lines.extend([set(30, Y, 34, "sticky_piston [\"facing_direction\"=4]"), set(31, Y, 34, "slime"), set(30, Y + 1, 34, POWER), String::new()]);
    lines.push(format!("fill 32 {Y} 35 32 {Y} 46 stone"));
    lines.extend([set(30, Y + 1, 34, "air"), String::new(), String::new()]);
    lines
}

/// Copper bulbs powered twice; a target and a lamp under a falling arrow; note blocks over other
/// blocks powered; a dispenser with an arrow and a crafter with a log powered; a lightning rod
/// struck; a jukebox given a disc.
fn devices() -> Vec<String> {
    let mut lines = stage([2, 40], [2, 40]);
    lines.extend([set(22, Y, 22, "copper_bulb"), set(23, Y, 22, POWER), String::new(), set(23, Y, 22, "air"), String::new(), set(23, Y, 22, POWER), String::new(), set(23, Y, 22, "air"), String::new()]);
    lines.extend([set(22, Y, 25, "target"), set(23, Y, 25, DUST), format!("summon arrow 22.5 {} 25.5", Y + 4), String::new(), String::new(), String::new(), String::new()]);
    for (x, under) in [(22, "gold_block"), (24, "clay"), (26, "packed_ice"), (28, "white_wool"), (30, "bone_block"), (32, "iron_block"), (34, "soul_sand"), (36, "pumpkin")] {
        lines.extend([set(x, Y, 28, under), set(x, Y + 1, 28, "noteblock"), set(x + 1, Y + 1, 28, POWER), set(x + 1, Y + 1, 28, "air")]);
    }
    lines.extend([set(22, Y, 31, "dispenser [\"facing_direction\"=1]"), "replaceitem block 22 -60 31 slot.container 0 arrow 2".to_owned(), set(23, Y, 31, POWER), String::new(), set(23, Y, 31, "air")]);
    lines.extend([set(26, Y, 31, "crafter [\"orientation\"=\"up_north\"]"), "replaceitem block 26 -60 31 slot.container 0 oak_log 1".to_owned(), set(27, Y, 31, POWER), String::new(), set(27, Y, 31, "air"), String::new()]);
    lines.extend([set(30, Y, 31, "lightning_rod"), set(31, Y, 31, DUST), format!("summon lightning_bolt 30.5 {} 31.5", Y + 1), String::new(), String::new()]);
    lines.extend([set(12, Y, 10, "jukebox"), set(13, Y, 10, DUST), "replaceitem entity @s slot.weapon.mainhand 0 music_disc_cat".to_owned(), "!click 2 0 0".to_owned(), String::new(), String::new()]);
    lines.extend(["clear @s".to_owned(), "!click 2 0 0".to_owned(), String::new(), "kill @e[type=item]".to_owned()]);
    lines
}
