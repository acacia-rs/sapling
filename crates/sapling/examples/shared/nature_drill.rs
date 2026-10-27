//! Drills of blocks that act by themselves: bubble columns, pointed dripstone, mushrooms, kelp
//! and cave vines growing, and ticks left waiting when the server stops.

/// Packets shown of these drills.
pub const NATURE_SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateSubchunkBlocks", "UpdateBlockSynced", "LevelSoundEvent", "LevelEvent", "AddEntity", "AddItemEntity", "RemoveEntity", "EntityEvent", "SetEntityMotion", "MoveEntityDelta",
    "MoveEntityAbsolute",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "bubbles" => bubbles(),
            "dripstone" => dripstone(),
            "plants" => plants(),
            "plantmeal" => plantmeal(),
            "pendingticks" => pending_ticks(),
            _ => return None,
        },
        NATURE_SHOWN,
    ))
}

fn floor() -> Vec<String> {
    ["kill @e[type=!player]", "gamemode creative @s", "clear @s", "fill ~-1 ~-1 ~7 ~8 ~-1 ~13 stone", "tp @s ~ ~ ~10 -90 0", "fill ~1 ~ ~-4 ~8 ~10 ~4 air", "time set noon"].map(String::from).into()
}

fn waits(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

/// Two glass tubes of still water seven high east of the actor, soul sand put under the first and
/// magma under the second; a pig and a dropped cobblestone in each; then the first tube's water cut
/// in the middle and the magma swapped for stone.
pub fn bubbles() -> Vec<String> {
    let mut lines = floor();
    lines.extend(["fill ~3 ~ ~-2 ~5 ~7 ~3 glass", "fill ~4 ~1 ~-1 ~4 ~7 ~-1 water", "fill ~4 ~1 ~2 ~4 ~7 ~2 water", "", "setblock ~4 ~ ~-1 soul_sand", "setblock ~4 ~ ~2 magma"].map(String::from));
    waits(&mut lines, 4);
    lines.extend(["summon pig ~4 ~2 ~-1", "summon pig ~4 ~6 ~2", "setblock ~4 ~9 ~-1 cobblestone destroy", "setblock ~4 ~9 ~2 cobblestone destroy", "setblock ~4 ~9 ~-1 air", "setblock ~4 ~9 ~2 air"].map(String::from));
    waits(&mut lines, 8);
    lines.push("setblock ~4 ~4 ~-1 air".into());
    waits(&mut lines, 4);
    lines.push("setblock ~4 ~ ~2 stone".into());
    waits(&mut lines, 4);
    lines.extend(["fill ~1 ~ ~-4 ~8 ~10 ~4 air", "kill @e[type=!player]", ""].map(String::from));
    lines
}

/// A stalactite three long hanging from stone over a pig held still, its stone taken away; then
/// stalactites under water and under lava over a cauldron each, at `randomtickspeed` 200.
pub fn dripstone() -> Vec<String> {
    let drip = |thickness: &str| format!("pointed_dripstone [\"hanging\"=true,\"dripstone_thickness\"=\"{thickness}\"]");
    let mut lines = floor();
    lines.extend(["setblock ~4 ~6 ~-3 stone".to_owned(), format!("setblock ~4 ~5 ~-3 {}", drip("base")), format!("setblock ~4 ~4 ~-3 {}", drip("frustum")), format!("setblock ~4 ~3 ~-3 {}", drip("tip"))]);
    lines.extend(["summon pig ~4 ~ ~-3", "effect @e[type=pig] slowness 100 255 true", "", "setblock ~4 ~6 ~-3 air"].map(String::from));
    waits(&mut lines, 4);
    for (z, liquid) in [(0, "water"), (3, "lava")] {
        lines.extend([
            format!("fill ~3 ~6 ~{} ~5 ~7 ~{} glass", z - 1, z + 1),
            format!("setblock ~4 ~6 ~{z} stone"),
            format!("setblock ~4 ~7 ~{z} {liquid}"),
            format!("setblock ~4 ~5 ~{z} {}", drip("frustum")),
            format!("setblock ~4 ~4 ~{z} {}", drip("tip")),
            format!("setblock ~4 ~ ~{z} cauldron"),
        ]);
    }
    lines.push("gamerule randomtickspeed 200".into());
    waits(&mut lines, 40);
    lines.extend(["gamerule randomtickspeed 1", "fill ~1 ~ ~-4 ~8 ~10 ~4 air", "kill @e[type=!player]", ""].map(String::from));
    lines
}

/// At `randomtickspeed` 200: a brown mushroom in a dark stone room, kelp at the foot of a glass
/// tube of water ten high, and cave vines under a stone ceiling, watched for a minute.
pub fn plants() -> Vec<String> {
    let mut lines = floor();
    lines.extend(
        [
            "fill ~1 ~ ~-4 ~7 ~3 ~-1 stone", "fill ~2 ~ ~-3 ~6 ~2 ~-2 air", "setblock ~4 ~ ~-3 brown_mushroom", "fill ~3 ~ ~0 ~5 ~9 ~2 glass", "fill ~4 ~ ~1 ~4 ~9 ~1 water", "setblock ~4 ~ ~1 kelp",
            "setblock ~4 ~8 ~4 stone", "setblock ~4 ~7 ~4 cave_vines", "gamerule randomtickspeed 200",
        ]
        .map(String::from),
    );
    waits(&mut lines, 60);
    lines.extend(["gamerule randomtickspeed 1", "fill ~1 ~ ~-4 ~8 ~10 ~4 air", "kill @e[type=item]", ""].map(String::from));
    lines
}

/// The actor in creative, looking east on stone, bone-meals a brown mushroom on dirt, a red one on
/// mycelium, kelp in a tube of water and cave vines hanging from stone, a block east of it.
pub fn plantmeal() -> Vec<String> {
    let cases: [(&[&str], &str, usize); 4] = [
        (&["fill ~1 ~-1 ~-3 ~7 ~-1 ~3 dirt", "setblock ~1 ~ ~ brown_mushroom"], "!click 1 0 0", 4),
        (&["fill ~1 ~-1 ~-3 ~7 ~-1 ~3 mycelium", "setblock ~1 ~ ~ red_mushroom"], "!click 1 0 0", 4),
        // A pit four deep in glass: kelp at its foot, water to the floor's level.
        (&["fill ~1 ~-5 ~-1 ~2 ~-1 ~1 glass", "fill ~1 ~-3 ~ ~1 ~-1 ~ water", "setblock ~1 ~-4 ~ kelp"], "!click 1 -4 0", 3),
        (&["setblock ~1 ~1 ~ stone", "setblock ~1 ~ ~ cave_vines"], "!click 1 0 0", 3),
    ];
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamemode creative @s", "clear @s", "replaceitem entity @s slot.weapon.mainhand 0 bone_meal", "time set noon", ""].map(String::from).into();
    for (setup, click, times) in cases {
        lines.extend(["fill ~-1 ~-1 ~7 ~1 ~-1 ~9 stone", "tp @s ~ ~ ~8 -90 0", "fill ~1 ~ ~-3 ~7 ~10 ~3 air"].map(String::from));
        lines.extend(setup.iter().map(|line| line.to_string()));
        lines.extend(std::iter::repeat_n(click.to_owned(), times));
        lines.push(String::new());
    }
    lines.extend(["fill ~1 ~ ~-3 ~7 ~10 ~3 air", "kill @e[type=!player]", "clear @s", ""].map(String::from));
    lines
}

/// Lava let fall onto a long floor and an observer clock, left running as the server stops, so
/// the saved world holds `PendingTicks` (read with sapling-leveldb's `records` example).
pub fn pending_ticks() -> Vec<String> {
    let mut lines = floor();
    lines.extend(
        [
            "fill ~1 ~-1 ~-4 ~8 ~-1 ~4 stone", "setblock ~3 ~ ~-3 observer [\"minecraft:facing_direction\"=\"east\"]", "setblock ~4 ~ ~-3 observer [\"minecraft:facing_direction\"=\"west\"]",
            // Liquid set by a command stands still until a neighbour changes.
            "setblock ~6 ~4 ~2 lava", "setblock ~2 ~4 ~2 water", "setblock ~6 ~3 ~2 stone", "setblock ~2 ~3 ~2 stone", "fill ~2 ~3 ~2 ~6 ~3 ~2 air",
        ]
        .map(String::from),
    );
    lines
}
