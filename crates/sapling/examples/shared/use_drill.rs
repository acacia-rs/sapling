//! The clicks drill of bds_command_probe: what a click does to blocks that take one, with the
//! hand empty or holding the thing they take.

#[path = "click_drill.rs"]
mod more;

pub const CLICKS_SHOWN: &[&str] = &["UpdateBlock", "LevelSoundEvent", "LevelEvent", "BlockEvent", "AddItemEntity", "InventorySlot", "MovePlayer"];

/// The actor in creative, looking east, clicks one block after another a block east of it: the block as set,
/// what is put in its hand, and how often it clicks. Each case lays its own floor: one fill of
/// the whole row was refused on both servers, where it ran past what was loaded.
pub fn clicks() -> Vec<String> {
    let cases: [(&str, &str, usize); 30] = [
        ("noteblock", "air", 3),
        ("bell", "air", 1),
        ("daylight_detector", "air", 2),
        ("cake", "air", 2),
        ("composter", "wheat_seeds", 6),
        ("cauldron", "water_bucket", 1),
        ("cauldron [\"fill_level\"=6,\"cauldron_liquid\"=\"water\"]", "glass_bottle", 2),
        ("flower_pot", "poppy", 2),
        ("jukebox", "music_disc_13", 2),
        ("respawn_anchor", "glowstone", 2),
        ("candle", "flint_and_steel", 2),
        ("campfire", "beef", 2),
        ("redstone_ore", "air", 1),
        ("sweet_berry_bush [\"growth\"=3]", "air", 1),
        ("beehive [\"honey_level\"=5]", "glass_bottle", 1),
        ("beehive [\"honey_level\"=5]", "shears", 1),
        ("pumpkin", "shears", 1),
        ("grass_block", "iron_shovel", 1),
        ("dirt", "iron_hoe", 1),
        ("coarse_dirt", "iron_hoe", 1),
        ("oak_log", "iron_axe", 1),
        ("waxed_copper", "iron_axe", 1),
        ("exposed_copper", "iron_axe", 1),
        ("copper_block", "honeycomb", 1),
        ("grass_block", "bone_meal", 1),
        ("oak_sapling", "bone_meal", 3),
        ("farmland", "wheat_seeds", 1),
        ("tnt", "flint_and_steel", 1),
        ("obsidian", "flint_and_steel", 1),
        ("iron_trapdoor", "air", 1),
    ];
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamemode creative @s", "clear @s", ""].map(String::from).into();
    for (block, hand, times) in cases {
        lines.extend(["fill ~-1 ~-1 ~3 ~3 ~-1 ~5 stone".into(), "tp @s ~ ~ ~4 -90 0".into(), "fill ~1 ~ ~-1 ~2 ~2 ~1 air".into(), format!("setblock ~1 ~ ~ {block}"), format!("replaceitem entity @s slot.weapon.mainhand 0 {hand}")]);
        lines.extend(std::iter::repeat_n("!click 1 0 0".to_owned(), times));
        lines.push(String::new());
    }
    lines.extend(["kill @e[type=!player]", "clear @s", ""].map(String::from));
    lines
}

pub const TNT_SHOWN: &[&str] = &[
    "SetEntityData", "LevelSoundEvent", "LevelEvent", "SetEntityMotion", "RemoveEntity", "AddEntity", "AddItemEntity", "MoveEntityDelta", "UpdateBlock", "UpdateSubchunkBlocks", "MovePlayer",
];

/// The actor in creative on stone over dirt: TNT 3 blocks east lit with flint and steel and left
/// to burst, then one lit by a block of redstone, then one summoned.
pub fn tnt() -> Vec<String> {
    let mut lines: Vec<String> =
        ["kill @e[type=!player]", "fill ~-6 ~-4 ~-6 ~10 ~-2 ~6 dirt", "fill ~-6 ~-1 ~-6 ~10 ~-1 ~6 stone", "fill ~-6 ~ ~-6 ~10 ~6 ~6 air", "gamemode creative @s", "tp @s ~ ~ ~ -90 0", ""].map(String::from).into();
    let lit = [
        vec!["setblock ~3 ~ ~ tnt", "replaceitem entity @s slot.weapon.mainhand 0 flint_and_steel", "!click 3 0 0"],
        vec!["fill ~-6 ~-1 ~-6 ~10 ~-1 ~6 stone", "fill ~-6 ~ ~-6 ~10 ~6 ~6 air", "setblock ~3 ~ ~ tnt", "setblock ~3 ~ ~1 redstone_block"],
        vec!["fill ~-6 ~-1 ~-6 ~10 ~-1 ~6 stone", "fill ~-6 ~ ~-6 ~10 ~6 ~6 air", "summon tnt ~3 ~ ~"],
    ];
    for steps in lit {
        lines.extend(steps.into_iter().map(String::from));
        lines.extend(std::iter::repeat_n(String::new(), 7));
    }
    lines.extend(["kill @e[type=!player]", "clear @s", ""].map(String::from));
    lines
}

pub const BONEMEAL_SHOWN: &[&str] = &["UpdateBlock", "UpdateSubchunkBlocks", "LevelSoundEvent", "LevelEvent", "AddItemEntity", "MovePlayer"];

/// The actor in creative, looking east on stone, bone-meals one plant after another a block east
/// of it, each on a field of its own floor; a case without a plant is the floor itself. The actor
/// keeps to stone: on farmland, which is lower, `~` named the cell under its feet.
pub fn bonemeal() -> Vec<String> {
    let moist = "farmland [\"moisturized_amount\"=7]";
    let cases: [(&str, &str, usize); 12] = [
        (moist, "wheat", 5),
        (moist, "carrots", 4),
        (moist, "beetroot", 5),
        (moist, "melon_stem", 4),
        ("grass_block", "oak_sapling", 8),
        ("grass_block", "sweet_berry_bush", 3),
        ("grass_block", "short_grass", 1),
        ("grass_block", "poppy", 1),
        ("grass_block", "", 2),
        ("sand", "reeds", 2),
        ("dirt", "bamboo_sapling", 2),
        ("stone", "", 1),
    ];
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamemode creative @s", "clear @s", "replaceitem entity @s slot.weapon.mainhand 0 bone_meal", ""].map(String::from).into();
    for (floor, plant, times) in cases {
        lines.extend(["fill ~-1 ~-1 ~7 ~1 ~-1 ~9 stone".into(), "tp @s ~ ~ ~8 -90 0".into(), "fill ~1 ~ ~-3 ~7 ~8 ~3 air".into(), format!("fill ~1 ~-1 ~-3 ~7 ~-1 ~3 {floor}")]);
        if !plant.is_empty() {
            lines.push(format!("setblock ~1 ~ ~ {plant}"));
        }
        let click = if plant.is_empty() { "!click 1 -1 0" } else { "!click 1 0 0" };
        lines.extend(std::iter::repeat_n(click.to_owned(), times));
        lines.push(String::new());
    }
    lines.extend(["kill @e[type=!player]", "clear @s", ""].map(String::from));
    lines
}

/// The actor in creative, looking east on stone, strikes flint and steel (a fire charge, last) on
/// the top of one block after another a block east of it, and waits to see what the fire does.
pub fn fire() -> Vec<String> {
    let cases: [(&str, &str, usize); 7] = [
        ("stone", "flint_and_steel", 6),
        ("netherrack", "flint_and_steel", 6),
        ("soul_sand", "flint_and_steel", 6),
        ("obsidian", "flint_and_steel", 6),
        ("oak_planks", "flint_and_steel", 20),
        ("white_wool", "flint_and_steel", 12),
        ("stone", "fire_charge", 6),
    ];
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamemode creative @s", "clear @s", "gamerule dofiretick true", ""].map(String::from).into();
    for (floor, held, waits) in cases {
        lines.extend(["fill ~-1 ~-1 ~7 ~1 ~-1 ~9 stone".into(), "tp @s ~ ~ ~8 -90 0".into(), "fill ~1 ~-1 ~-3 ~7 ~8 ~3 air".into(), format!("fill ~1 ~-1 ~-1 ~3 ~-1 ~1 {floor}")]);
        lines.extend([format!("replaceitem entity @s slot.weapon.mainhand 0 {held}"), "!click 2 -1 0".into()]);
        lines.extend(std::iter::repeat_n(String::new(), waits));
    }
    lines.extend(["kill @e[type=!player]", "clear @s", ""].map(String::from));
    lines
}

/// The actor in survival, looking east on stone, puts fire two blocks east out: by hand, with a
/// bucket of water, by taking the block under it away, and by rain (over stone, then netherrack).
pub fn quench() -> Vec<String> {
    let cases: [(&str, &[&str], usize); 5] = [
        ("netherrack", &["!break 2 0 0"], 2),
        ("netherrack", &["replaceitem entity @s slot.weapon.mainhand 0 water_bucket", "!click 2 -1 0"], 3),
        ("netherrack", &["setblock ~2 ~-1 ~ air"], 3),
        ("stone", &["weather rain"], 12),
        ("netherrack", &[], 12),
    ];
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamemode survival @s", "clear @s", "gamerule dofiretick true", "weather clear", ""].map(String::from).into();
    for (floor, steps, waits) in cases {
        lines.extend(["fill ~-1 ~-1 ~7 ~1 ~-1 ~9 stone".into(), "tp @s ~ ~ ~8 -90 0".into(), "fill ~1 ~-1 ~-3 ~7 ~8 ~3 air".into(), format!("fill ~1 ~-1 ~-1 ~3 ~-1 ~1 {floor}"), "setblock ~2 ~ ~ fire".into()]);
        lines.extend(steps.iter().map(|step| (*step).to_owned()));
        lines.extend(std::iter::repeat_n(String::new(), waits));
    }
    lines.extend(["weather clear", "kill @e[type=!player]", "gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}

/// The actor in creative puts one disc after another into a jukebox a block east of it and takes
/// it out again, a comparator and dust behind the jukebox showing what it reads.
pub fn discs() -> Vec<String> {
    let mut lines: Vec<String> = [
        "kill @e[type=!player]", "gamemode creative @s", "clear @s", "fill ~-1 ~-1 ~7 ~4 ~-1 ~9 stone", "tp @s ~ ~ ~8 -90 0", "fill ~1 ~ ~-1 ~4 ~2 ~1 air", "setblock ~1 ~ ~ jukebox",
        "setblock ~2 ~ ~ unpowered_comparator [\"minecraft:cardinal_direction\"=\"west\",\"output_subtract_bit\"=false]", "setblock ~3 ~ ~ redstone_wire", "",
    ].map(String::from).into();
    for disc in ["13", "cat", "blocks", "wait", "pigstep", "otherside", "5", "relic", "creator", "creator_music_box", "precipice", "tears", "lava_chicken"] {
        lines.extend([format!("replaceitem entity @s slot.weapon.mainhand 0 music_disc_{disc}"), "!click 1 0 0".into(), String::new(), "!click 1 0 0".into(), "kill @e[type=item]".into()]);
    }
    lines.extend(["setblock ~1 ~ ~ air", "kill @e[type=!player]", "clear @s", ""].map(String::from));
    lines
}

/// Twenty-five lit redstone ores east of the actor, watched for a minute and a half to see when
/// each goes out.
pub fn ores() -> Vec<String> {
    let mut lines: Vec<String> =
        ["fill ~-1 ~-1 ~7 ~8 ~-1 ~13 stone", "tp @s ~ ~ ~10 -90 0", "fill ~1 ~ ~-3 ~8 ~3 ~3 air", "fill ~2 ~ ~-2 ~6 ~ ~2 lit_redstone_ore"].map(String::from).into();
    lines.extend(std::iter::repeat_n(String::new(), 90));
    lines.push("fill ~2 ~ ~-2 ~6 ~ ~2 air".into());
    lines
}

/// Twenty-four wheat on wet farmland around a water block at noon, east of the actor, with the
/// random ticks sped up two hundred times and watched for a minute.
pub fn crops() -> Vec<String> {
    let mut lines: Vec<String> = [
        "fill ~-1 ~-1 ~7 ~8 ~-1 ~13 stone", "tp @s ~ ~ ~10 -90 0", "fill ~1 ~ ~-3 ~8 ~3 ~3 air", "time set noon", "fill ~2 ~-1 ~-2 ~6 ~-1 ~2 farmland [\"moisturized_amount\"=7]",
        "setblock ~4 ~-1 ~ water", "fill ~2 ~ ~-2 ~6 ~ ~2 wheat", "gamerule randomtickspeed 200",
    ].map(String::from).into();
    lines.extend(std::iter::repeat_n(String::new(), 60));
    lines.extend(["gamerule randomtickspeed 1", "fill ~2 ~ ~-2 ~6 ~ ~2 air", "fill ~2 ~-1 ~-2 ~6 ~-1 ~2 stone"].map(String::from));
    lines
}

/// A row of what else random ticks change, east of the actor at noon, with the random ticks sped
/// up two hundred times and watched for a minute: cane by water, a cactus, a ripe melon stem by
/// dirt, bamboo, an oak sapling, dirt by grass, and farmland dry and wet with no water near.
pub fn grows() -> Vec<String> {
    let mut lines: Vec<String> = ["fill ~-1 ~-1 ~7 ~10 ~-1 ~23 stone", "tp @s ~ ~ ~15 -90 0", "fill ~1 ~ ~-8 ~10 ~16 ~8 air", "time set noon"].map(String::from).into();
    let row: [(i32, &[(i32, i32, &str)]); 8] = [
        (-7, &[(3, -1, "sand"), (4, -1, "water"), (3, 0, "reeds")]),
        (-5, &[(3, -1, "sand"), (3, 0, "cactus")]),
        (-3, &[(3, -1, "farmland [\"moisturized_amount\"=7]"), (4, -1, "dirt"), (3, 0, "melon_stem [\"growth\"=7]")]),
        (-1, &[(3, -1, "dirt"), (3, 0, "bamboo")]),
        (1, &[(3, -1, "dirt"), (3, 0, "oak_sapling")]),
        (3, &[(3, -1, "grass_block"), (4, -1, "dirt"), (5, -1, "dirt")]),
        (5, &[(3, -1, "farmland [\"moisturized_amount\"=0]")]),
        (7, &[(3, -1, "farmland [\"moisturized_amount\"=7]")]),
    ];
    for (z, blocks) in row {
        lines.extend(blocks.iter().map(|(x, y, block)| format!("setblock ~{x} ~{y} ~{z} {block}")));
    }
    lines.push("gamerule randomtickspeed 200".into());
    lines.extend(std::iter::repeat_n(String::new(), 60));
    lines.extend(["gamerule randomtickspeed 1", "fill ~1 ~ ~-8 ~10 ~16 ~8 air", "fill ~1 ~-1 ~-8 ~10 ~-1 ~8 stone", "kill @e[type=item]"].map(String::from));
    lines
}

/// Two cubes of oak leaves east of the actor, each around a log, the second set down as a
/// player's are: with the random ticks sped up two hundred times, nothing is touched for five
/// seconds, then both logs are taken out and the leaves watched for a minute. (Leaves by no log
/// that nothing touched did not decay on BDS.)
pub fn leaves() -> Vec<String> {
    let mut lines: Vec<String> = [
        "fill ~-1 ~-1 ~7 ~10 ~-1 ~23 stone",
        "tp @s ~ ~ ~15 -90 0",
        "fill ~1 ~ ~-8 ~10 ~16 ~8 air",
        "fill ~3 ~1 ~-5 ~5 ~3 ~-3 oak_leaves [\"persistent_bit\"=false]",
        "setblock ~4 ~2 ~-4 oak_log",
        "fill ~3 ~1 ~3 ~5 ~3 ~5 oak_leaves [\"persistent_bit\"=true]",
        "setblock ~4 ~2 ~4 oak_log",
        "gamerule randomtickspeed 200",
    ]
    .map(String::from)
    .into();
    lines.extend(std::iter::repeat_n(String::new(), 5));
    lines.extend(["setblock ~4 ~2 ~-4 air", "setblock ~4 ~2 ~4 air"].map(String::from));
    lines.extend(std::iter::repeat_n(String::new(), 60));
    lines.extend(["gamerule randomtickspeed 1", "fill ~1 ~ ~-8 ~10 ~16 ~8 air", "kill @e[type=item]"].map(String::from));
    lines
}

/// The actor in survival drops onto farmland from four heights, three times each, the farmland
/// laid anew before every drop, then twice onto farmland with wheat on it.
pub fn trample() -> Vec<String> {
    let mut lines: Vec<String> = ["fill ~-1 ~-1 ~7 ~3 ~-1 ~11 stone", "tp @s ~ ~ ~9 -90 0", "fill ~-1 ~ ~-1 ~3 ~8 ~1 air", "gamemode survival @s", "effect @s resistance 300 4 true"].map(String::from).into();
    for height in ["0.4", "0.8", "1.5", "3"] {
        for _ in 0..3 {
            lines.extend(["setblock ~ ~-1 ~ farmland [\"moisturized_amount\"=7]".to_owned(), format!("tp @s ~ ~{height} ~"), String::new()]);
        }
    }
    for _ in 0..2 {
        lines.extend(["setblock ~ ~-1 ~ farmland [\"moisturized_amount\"=7]", "setblock ~ ~ ~ wheat", "tp @s ~ ~1.5 ~", ""].map(String::from));
    }
    lines.extend(["setblock ~ ~-1 ~ stone", "effect @s clear", "gamemode creative @s", "kill @e[type=item]"].map(String::from));
    lines
}

/// The drill called `name` of this file, and the packets shown of it.
pub fn named(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "clicks" => (clicks(), CLICKS_SHOWN),
        "clicks2" => (more::clicks2(), CLICKS_SHOWN),
        "scaffold" => (more::scaffold(), TNT_SHOWN),
        "grows2" => (more::grows2(), BONEMEAL_SHOWN),
        "copper" => (more::copper(), BONEMEAL_SHOWN),
        "oresteps" => (more::oresteps(), BONEMEAL_SHOWN),
        "bonemeal2" => (more::bonemeal2(), BONEMEAL_SHOWN),
        "leafreach" => (more::leafreach(), BONEMEAL_SHOWN),
        "stacks" => (more::stacks(), CLICKS_SHOWN),
        "tnt" => (tnt(), TNT_SHOWN),
        "bonemeal" => (bonemeal(), BONEMEAL_SHOWN),
        "fire" => (fire(), BONEMEAL_SHOWN),
        "quench" => (quench(), BONEMEAL_SHOWN),
        "discs" => (discs(), BONEMEAL_SHOWN),
        "ores" => (ores(), BONEMEAL_SHOWN),
        "crops" => (crops(), BONEMEAL_SHOWN),
        "grows" => (grows(), BONEMEAL_SHOWN),
        "leaves" => (leaves(), BONEMEAL_SHOWN),
        "trample" => (trample(), BONEMEAL_SHOWN),
        _ => return None,
    })
}
