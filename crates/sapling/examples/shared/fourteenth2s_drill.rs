//! The fourteenth wave's group S drills (research/parity/FOURTEENTH-BRIEF.md). `f14snest`: with mobs spawning, a
//! nest and a hive set and bee nests placed by tree features (`/place` needs op-permission-level=4 on the BDS
//! copy) at noon, bees counted over a minute; `f14snest2` a nest set at midnight. The folder's block entities are
//! read after each with sapling-leveldb's `block_entity_dump`. `f14sstored` (KEEP_WORLD=1, after sapling-leveldb's
//! `set_blocks` edited the stopped world, [`CHECKS`]): what the blocks a folder holds do once loaded, before
//! anything beside them is written; then trial spawners with inline configs, each config's `spawn_potentials`
//! entry written another way (sapling-leveldb's `put_trial_spawners`), with a survival player by them.

use super::creatures::{floor, waits, with};

const SHOWN: &[&str] = &["AddEntity", "RemoveEntity", "UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "LevelEvent", "LevelSoundEvent", "CommandOutput"];

/// What each cell `set_blocks` wrote for `f14sstored` holds, as `testforblock` asks. The cells (the flat world's
/// ground is y -61): scaffolding on the ground (20 -60 2), floating (20 -57 4) and with stability 5 on the ground
/// (22 -60 4); a lamp (21 -60 8) by a block of redstone (20 -60 8), a lit lamp (24 -60 8) and dust at 15
/// (20 -60 12) with nothing powering them, an unlit torch on the ground (24 -60 12).
const CHECKS: &[&str] = &[
    "testforblock 20 -60 2 scaffolding",
    "testforblock 20 -57 4 scaffolding",
    "testforblock 22 -60 4 scaffolding [\"stability\"=0]",
    "testforblock 22 -60 4 scaffolding [\"stability\"=5]",
    "testforblock 21 -60 8 lit_redstone_lamp",
    "testforblock 24 -60 8 lit_redstone_lamp",
    "testforblock 20 -60 12 redstone_wire [\"redstone_signal\"=15]",
    "testforblock 20 -60 12 redstone_wire [\"redstone_signal\"=0]",
    "testforblock 24 -60 12 redstone_torch",
];

pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "f14snest" => (nests(), SHOWN),
        "f14snest2" => (night_nest(), SHOWN),
        "f14sstored" => (stored(), SHOWN),
        _ => return None,
    })
}

/// A nest set at midnight, with mobs spawning: its bees are made at once or wait for the day (read from the folder).
fn night_nest() -> Vec<String> {
    let mut lines = with(floor(), &["gamerule domobspawning true", "time set midnight", "setblock -6 -26 -6 bee_nest", "testfor @e[type=bee]"]);
    waits(&mut lines, 5);
    with(lines, &["testfor @e[type=bee]"])
}

fn nests() -> Vec<String> {
    let mut lines = with(floor(), &[
        "gamerule domobspawning true",
        "time set noon",
        "fill -8 -27 -8 16 -27 8 grass_block",
        "setblock -6 -26 -6 bee_nest",
        "setblock -6 -26 6 beehive",
        "place feature minecraft:cherry_tree_with_beehive_feature 2 -26 -5",
        "place feature minecraft:birch_tree_with_beehive_feature 12 -26 -5",
        "place feature minecraft:fancy_oak_tree_with_beehive_feature 2 -26 5",
        "place feature minecraft:birch_tree_with_beehive_feature 12 -26 5",
        "testfor @e[type=bee]",
    ]);
    for seconds in [2, 10, 20, 30] {
        waits(&mut lines, seconds);
        lines.push("testfor @e[type=bee]".into());
    }
    lines
}

fn stored() -> Vec<String> {
    let mut lines = with(Vec::new(), &["gamemode creative @s", "tp @s 22.5 -60 -1.5", ""]);
    lines.extend(CHECKS.iter().map(|line| line.to_string()));
    waits(&mut lines, 10);
    lines.extend(CHECKS.iter().map(|line| line.to_string()));
    // sapling-leveldb's `put_trial_spawners` at 20 -60 -6: a pig, a cow and a sheep spawner, each written another way.
    lines = with(lines, &["difficulty easy", "gamemode survival @s", "effect @s resistance 100 4 true", "tp @s 24.5 -60 -9.5", ""]);
    waits(&mut lines, 15);
    with(lines, &["testfor @e[type=pig,x=24,y=-60,z=-6,r=10]", "testfor @e[type=cow,x=24,y=-60,z=-6,r=10]", "testfor @e[type=sheep,x=24,y=-60,z=-6,r=10]", "gamemode creative @s"])
}
