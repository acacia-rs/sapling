//! The clicks drill of bds_command_probe: what a click does to blocks that take one, with the
//! hand empty or holding the thing they take.

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
