//! The clock, sounds, loot, names and sun drills of bds_command_probe: commands, what blocks
//! sound like and leave, the names they go by, and daylight.

/// `/time` and `/xp` in every form, with a query between.
pub fn clock() -> Vec<String> {
    let mut lines: Vec<String> = ["gamerule dodaylightcycle false", "time set 0", "time query daytime", "time query gametime", "time query day"].map(String::from).into();
    for to in ["day", "noon", "sunset", "night", "midnight", "sunrise", "1000", "30000", "-5"] {
        lines.extend([format!("time set {to}"), "time query daytime".into(), "time query day".into()]);
    }
    lines.extend(["time add 500", "time query daytime", "time add -100", "time query daytime", "time set banana", "time", "gamerule dodaylightcycle true"].map(String::from));
    lines.extend(["xp 7 @s", "xp 30 @s", "xp 3L @s", "xp -1L @s", "xp -5 @s", "xp 5", "xp -100L @s", "xp 2l @s", "xp banana @s", "xp 1 @e[type=cow]", ""].map(String::from));
    lines
}

pub const SOUNDS_SHOWN: &[&str] = &["LevelSoundEvent", "LevelEvent", "BlockEvent", "UpdateBlock", "UpdateSubchunkBlocks", "ContainerOpen"];

/// The actor clicks a door, a trapdoor and a fence gate open and shut, and a chest, a barrel
/// and an ender chest open; then puts down doors by hand: one alone, one with a block on each
/// side in turn, and one beside a door.
pub fn sounds() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "fill ~-2 ~-1 ~-2 ~6 ~-1 ~40 stone", "fill ~-2 ~ ~-2 ~6 ~4 ~40 air", "gamemode creative @s", ""].map(String::from).into();
    for block in ["wooden_door", "trapdoor", "fence_gate", "chest", "barrel", "ender_chest"] {
        lines.extend(["tp @s ~ ~ ~3".into(), format!("setblock ~1 ~ ~ {block}"), String::new(), "!click 1 0 0".into(), String::new(), String::new(), "!click 1 0 0".into(), String::new(), String::new()]);
    }
    lines.extend(["clear @s", "give @s wooden_door 8", ""].map(String::from));
    // Alone; a block on one side, then the other; a door on one side, then the other.
    for setup in [&[][..], &["setblock ~1 ~ ~1 stone"], &["setblock ~1 ~ ~-1 stone"], &["!place 1 -1 1", ""], &["!place 1 -1 -1", ""]] {
        lines.push("tp @s ~ ~ ~4".into());
        lines.extend(setup.iter().map(|line| (*line).to_owned()));
        lines.extend(["!place 1 -1 0", ""].map(String::from));
    }
    lines.extend(["clear @s", ""].map(String::from));
    lines
}

pub const LOOT_SHOWN: &[&str] = &["AddItemEntity", "AddEntity", "LevelEvent"];

/// The actor in survival breaks blocks with a diamond pickaxe, those whose loot varies several
/// times over, to see what each leaves.
pub fn loot() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "fill ~-2 ~-1 ~-2 ~4 ~-1 ~4 bedrock", "fill ~-2 ~ ~-2 ~4 ~4 ~4 air", "gamemode survival @s", "clear @s", "replaceitem entity @s slot.weapon.mainhand 0 diamond_pickaxe", ""].map(String::from).into();
    let blocks = [
        ("stone", 1), ("cobblestone", 1), ("grass_block", 1), ("coal_ore", 4), ("iron_ore", 1), ("diamond_ore", 4), ("redstone_ore", 4), ("lapis_ore", 4), ("emerald_ore", 1), ("quartz_ore", 2),
        ("glowstone", 4), ("sea_lantern", 4), ("clay", 2), ("gravel", 6), ("glass", 1), ("bookshelf", 2), ("melon_block", 4), ("oak_leaves", 6), ("snow", 1), ("stonebrick", 1),
        ("crafting_table", 1), ("deepslate_diamond_ore", 1), ("nether_gold_ore", 3), ("amethyst_cluster", 2), ("copper_ore", 3), ("ender_chest", 1), ("ice", 1),
    ];
    for (block, times) in blocks {
        for _ in 0..times {
            lines.extend([format!("setblock ~1 ~ ~ {block}"), "!break 1 0 0".into(), String::new()]);
        }
        lines.push("kill @e[type=!player]".into());
    }
    lines.extend(["gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}

pub const LOOT2_SHOWN: &[&str] = &["AddItemEntity", "AddEntity", "UpdateBlock"];

/// What `loot` left open: copper ore twelve times over, blocks set by old names and asked
/// for by their new ones, and ice broken last, with the blocks that change shown.
pub fn loot2() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "fill ~-2 ~-1 ~-2 ~4 ~-1 ~4 bedrock", "fill ~-2 ~ ~-2 ~4 ~4 ~4 air", "gamemode survival @s", "clear @s", "replaceitem entity @s slot.weapon.mainhand 0 diamond_pickaxe", ""].map(String::from).into();
    for _ in 0..12 {
        lines.extend(["setblock ~1 ~ ~ copper_ore", "!break 1 0 0", ""].map(String::from));
    }
    lines.push("kill @e[type=!player]".into());
    for (old, new) in [("stonebrick", "stone_bricks"), ("grass", "grass_block"), ("planks", "oak_planks"), ("log", "oak_log"), ("wool", "white_wool"), ("stone_slab", "smooth_stone_slab"), ("concrete", "white_concrete"), ("leaves", "oak_leaves")] {
        lines.extend([format!("setblock ~3 ~ ~ {old}"), format!("testforblock ~3 ~ ~ {new}"), format!("testforblock ~3 ~ ~ {old}"), "setblock ~3 ~ ~ air".into()]);
    }
    lines.extend(["setblock ~1 ~ ~ ice", "!break 1 0 0", "", "", "gamemode creative @s", "clear @s", ""].map(String::from));
    lines
}

/// Old block names set and then asked for by the name each is thought to have now.
pub fn names() -> Vec<String> {
    let pairs = [
        ("sapling", "oak_sapling"), ("fence", "oak_fence"), ("carpet", "white_carpet"), ("stained_glass", "white_stained_glass"), ("stained_glass_pane", "white_stained_glass_pane"),
        ("stained_hardened_clay", "white_terracotta"), ("concrete_powder", "white_concrete_powder"), ("concretepowder", "white_concrete_powder"), ("log2", "acacia_log"), ("leaves2", "acacia_leaves"),
        ("wooden_slab", "oak_slab"), ("stone_slab2", "red_sandstone_slab"), ("stone_slab3", "end_stone_brick_slab"), ("stone_slab4", "mossy_stone_brick_slab"), ("double_stone_slab", "smooth_stone_double_slab"),
        ("double_wooden_slab", "oak_double_slab"), ("red_flower", "poppy"), ("yellow_flower", "dandelion"), ("tallgrass", "short_grass"), ("double_plant", "sunflower"), ("coral", "tube_coral"),
        ("coral_block", "tube_coral_block"), ("coral_fan", "tube_coral_fan"), ("coral_fan_dead", "dead_tube_coral_fan"), ("skull", "skeleton_skull"), ("monster_egg", "infested_stone"), ("wood", "oak_wood"),
        ("shulker_box", "white_shulker_box"), ("sealantern", "sea_lantern"), ("light_block", "light_block_0"), ("sand", "sand"), ("dirt", "dirt"), ("stone", "stone"), ("prismarine", "prismarine"),
        ("quartz_block", "quartz_block"), ("sandstone", "sandstone"), ("red_sandstone", "red_sandstone"), ("purpur_block", "purpur_block"), ("sponge", "sponge"), ("cobblestone_wall", "cobblestone_wall"),
        ("anvil", "anvil"), ("tnt", "tnt"), ("torch", "torch"), ("hardened_clay", "hardened_clay"), ("chemistry_table", "compound_creator"), ("colored_torch_rg", "colored_torch_red"), ("colored_torch_bp", "colored_torch_blue"),
        ("yellow_flower", "yellow_flower"), ("nosuchblock", "stone"),
    ];
    let mut lines: Vec<String> = ["fill ~2 ~ ~-1 ~4 ~2 ~1 air", "fill ~2 ~-1 ~-1 ~4 ~-1 ~1 dirt", "gamemode creative @s", ""].map(String::from).into();
    for (old, new) in pairs {
        lines.extend([format!("setblock ~3 ~ ~ {old}"), format!("testforblock ~3 ~ ~ {new}"), "setblock ~3 ~ ~ air".into()]);
    }
    lines
}

/// The actor in creative under an open sky at noon: a zombie left twelve seconds, one under
/// a stone roof and one under glass, then other kinds, then zombies at other times of day.
pub fn sun() -> Vec<String> {
    let mut lines: Vec<String> =
        ["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "weather clear", "time set noon", "fill ~-3 ~-1 ~-12 ~12 ~-1 ~12 stone", "fill ~-3 ~ ~-12 ~12 ~12 ~12 air", "gamemode creative @s", ""].map(String::from).into();
    let left = |lines: &mut Vec<String>, setup: &[&str], kind: &str, seconds: usize| {
        lines.extend(setup.iter().map(|line| (*line).to_owned()));
        lines.push(format!("summon {kind} ~4 ~ ~"));
        lines.extend(std::iter::repeat_n(String::new(), seconds));
        lines.extend(["kill @e[type=!player]", "fill ~1 ~ ~-3 ~7 ~4 ~3 air", ""].map(String::from));
    };
    left(&mut lines, &[], "zombie", 12);
    left(&mut lines, &["fill ~1 ~3 ~-3 ~7 ~3 ~3 stone"], "zombie", 6);
    left(&mut lines, &["fill ~1 ~3 ~-3 ~7 ~3 ~3 glass"], "zombie", 6);
    left(&mut lines, &["fill ~1 ~3 ~-3 ~7 ~3 ~3 oak_leaves"], "zombie", 6);
    for kind in ["skeleton", "creeper", "husk", "stray", "drowned", "zombie_villager", "spider", "phantom"] {
        left(&mut lines, &[], kind, 6);
    }
    for time in ["12000", "12600", "13000", "23000", "23500", "0"] {
        left(&mut lines, &[&format!("time set {time}")], "zombie", 6);
    }
    lines.extend(["time set noon", "weather rain", ""].map(String::from));
    left(&mut lines, &[], "zombie", 6);
    lines.extend(["weather clear", "gamerule dodaylightcycle true", ""].map(String::from));
    lines
}

pub const WEATHER_SHOWN: &[&str] = &["LevelEvent", "GameRulesChanged", "LevelSoundEvent"];

/// `/weather` in each form, and what the players are told of the sky.
pub fn weather() -> Vec<String> {
    ["weather query", "weather rain", "", "", "weather query", "weather rain", "weather thunder", "", "", "weather query", "weather clear", "", "", "weather query", "weather rain 100", "", "", "", "", "", "", "weather query", "weather clear 50", "weather banana", "weather", "toggledownfall", "", "weather query", "toggledownfall", "", "weather clear", ""]
        .map(String::from)
        .into()
}

/// The actor in survival and a spider, then a cave spider, 5 blocks off for six seconds at noon
/// under the sky, at noon under a roof, and at midnight.
pub fn spiders() -> Vec<String> {
    let mut lines: Vec<String> =
        ["kill @e[type=!player]", "gamerule domobspawning false", "gamerule dodaylightcycle false", "difficulty easy", "fill ~-3 ~-1 ~-12 ~12 ~-1 ~12 stone", "fill ~-3 ~ ~-12 ~12 ~12 ~12 air", "gamemode survival @s", ""].map(String::from).into();
    for kind in ["spider", "cave_spider"] {
        for setup in [&["time set noon"][..], &["time set noon", "fill ~-3 ~4 ~-12 ~12 ~4 ~12 stone"], &["time set midnight", "fill ~-3 ~4 ~-12 ~12 ~4 ~12 air"]] {
            lines.extend(setup.iter().map(|line| (*line).to_owned()));
            lines.extend(["effect @s instant_health 1 4".into(), format!("summon {kind} ~ ~ ~-5")]);
            lines.extend(std::iter::repeat_n(String::new(), 6));
            lines.extend(["kill @e[type=!player]", "effect @s clear", ""].map(String::from));
        }
    }
    lines.extend(["gamemode creative @s", "gamerule dodaylightcycle true", "time set noon", ""].map(String::from));
    lines
}

/// As `kinds`, more kinds that hit: each 8 blocks from the actor at midnight for seven seconds.
pub fn kinds2() -> Vec<String> {
    let mut lines: Vec<String> =
        ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "time set midnight", "fill ~-3 ~-1 ~-12 ~12 ~-1 ~60 stone", "fill ~-3 ~ ~-12 ~12 ~6 ~60 air", "gamemode survival @s", "tp @s ~6 ~ ~20", ""].map(String::from).into();
    for kind in ["drowned", "endermite", "piglin_brute", "hoglin", "zoglin", "ravager", "slime", "magma_cube", "polar_bear", "witch", "blaze", "enderman"] {
        lines.extend(["effect @s instant_health 1 4".into(), format!("summon {kind} ~ ~ ~-8")]);
        lines.extend(std::iter::repeat_n(String::new(), 7));
        lines.extend(["kill @e[type=!player]", "effect @s clear", "effect @s instant_health 1 4", "", ""].map(String::from));
    }
    lines.extend(["gamemode creative @s", "kill @e[type=!player]", ""].map(String::from));
    lines
}

pub const PUSHED_SHOWN: &[&str] = &["MovePlayer", "SetEntityMotion", "CorrectPlayerMovePrediction", "MoveEntityDelta", "MoveEntityAbsolute", "UpdateBlock", "BlockEvent"];

/// The actor in survival pushed by pistons: one beside it that pushes east into it, one under
/// it that lifts it, one that pushes it against a wall, and a sticky one drawn back from it.
pub fn pushed() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "fill ~-4 ~-1 ~-4 ~8 ~-1 ~30 stone", "fill ~-4 ~ ~-4 ~8 ~6 ~30 air", "gamemode survival @s", "tp @s ~ ~ ~4", ""].map(String::from).into();
    let east = "piston [\"facing_direction\"=4]";
    let rows: [&[&str]; 4] = [
        &["setblock ~-1 ~ ~ {east}", "", "setblock ~-2 ~ ~ redstone_block", "", "", "setblock ~-2 ~ ~ air", "", ""],
        &["setblock ~ ~-1 ~ piston [\"facing_direction\"=1]", "", "setblock ~ ~-2 ~ redstone_block", "", "", "setblock ~ ~-2 ~ air", "", ""],
        &["setblock ~1 ~ ~ obsidian", "setblock ~1 ~1 ~ obsidian", "setblock ~-1 ~ ~ {east}", "", "setblock ~-2 ~ ~ redstone_block", "", "", "setblock ~-2 ~ ~ air", "", ""],
        &["setblock ~-1 ~ ~ sticky_{east}", "", "setblock ~-2 ~ ~ redstone_block", "", "", "setblock ~-2 ~ ~ air", "", ""],
    ];
    for row in rows {
        lines.push("tp @s ~ ~ ~5".into());
        lines.extend(row.iter().map(|line| line.replace("{east}", east)));
    }
    lines.extend(["gamemode creative @s", ""].map(String::from));
    lines
}

/// The actor hits the watcher, both in survival where they came in: with the fist four times a
/// second apart and twice at once, with a wooden and a diamond sword, then with `pvp` off, on
/// peaceful, and with the watcher in creative.
pub fn pvp() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamerule pvp true", "difficulty easy", "gamemode survival @a", "clear @a", "effect @a instant_health 1 4", ""].map(String::from).into();
    let hit = ["!hit player", "", ""].map(String::from);
    (0..4).for_each(|_| lines.extend(hit.clone()));
    lines.extend(["effect @a instant_health 1 4", "", "!hit2 player", "", ""].map(String::from));
    for sword in ["wooden_sword", "diamond_sword"] {
        lines.extend(["effect @a instant_health 1 4".into(), format!("replaceitem entity @s slot.weapon.mainhand 0 {sword}"), String::new()]);
        lines.extend(hit.clone());
    }
    lines.extend(["clear @a", "effect @a instant_health 1 4", "gamerule pvp false", ""].map(String::from));
    lines.extend(hit.clone());
    lines.extend(["gamerule pvp true", "difficulty peaceful", ""].map(String::from));
    lines.extend(hit.clone());
    lines.extend(["difficulty easy", "gamemode creative @a", "gamemode survival @s", ""].map(String::from));
    lines.extend(hit.clone());
    lines.extend(["gamemode creative @a", ""].map(String::from));
    lines
}

/// The actor in survival, proof against fire and healed each second, with a blaze 8 blocks off
/// for twenty-five seconds, then one 20 blocks off for ten.
pub fn blaze() -> Vec<String> {
    let mut lines: Vec<String> =
        ["kill @e[type=!player]", "gamerule domobspawning false", "difficulty easy", "time set midnight", "fill ~-3 ~-1 ~-30 ~12 ~-1 ~60 stone", "fill ~-3 ~ ~-30 ~12 ~8 ~60 air", "gamemode survival @s", "tp @s ~6 ~ ~20", ""].map(String::from).into();
    for (far, seconds) in [(8, 25), (20, 10)] {
        lines.extend(["effect @s fire_resistance 60 0".into(), format!("summon blaze ~ ~ ~-{far}")]);
        lines.extend(std::iter::repeat_n("effect @s instant_health 1 4".to_owned(), seconds));
        lines.extend(["kill @e[type=!player]", "effect @s clear", "", ""].map(String::from));
    }
    lines.extend(["gamemode creative @s", ""].map(String::from));
    lines
}

/// The actor kills the watcher with a diamond sword, both in survival, to hear what is said.
pub fn duel() -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "gamerule pvp true", "difficulty easy", "gamemode survival @a", "clear @a", "effect @a instant_health 1 4", "replaceitem entity @s slot.weapon.mainhand 0 diamond_sword", ""].map(String::from).into();
    (0..3).for_each(|_| lines.extend(["!hit player", "", ""].map(String::from)));
    lines.extend(["", "", "gamemode creative @a", "clear @a", ""].map(String::from));
    lines
}

pub const PLACING_SHOWN: &[&str] = &["UpdateBlock", "MovePlayer"];

/// The actor in creative, looking south, puts each of many blocks that turn or attach on the
/// floor a block east of it, to see the state each takes.
pub fn placing() -> Vec<String> {
    let items = [
        "oak_stairs", "oak_slab", "oak_log", "torch", "oak_sign", "oak_trapdoor", "fence_gate", "stone_button", "lever", "rail", "repeater", "comparator", "anvil", "white_glazed_terracotta", "carved_pumpkin",
        "end_rod", "lantern", "bell", "grindstone", "lectern", "loom", "stonecutter_block", "campfire", "barrel", "hopper", "dropper", "dispenser", "observer", "piston", "sticky_piston", "furnace", "chest", "ender_chest",
        "undyed_shulker_box", "skeleton_skull", "bamboo", "candle", "ladder", "chain", "lightning_rod", "pointed_dripstone", "amethyst_cluster", "hay_block", "bone_block", "quartz_pillar", "beehive", "jukebox", "cake",
        "brewing_stand", "cauldron", "composter", "smoker", "blast_furnace", "respawn_anchor", "lodestone", "target", "daylight_detector", "tripwire_hook", "redstone_torch", "sea_pickle", "turtle_egg", "scaffolding",
        "oak_fence", "cobblestone_wall", "glass_pane", "iron_bars", "snow_layer", "white_carpet", "white_bed", "flower_pot", "enchanting_table", "end_portal_frame", "frame", "white_banner", "conduit",
    ];
    placed_in_turn(&items)
}

fn placed_in_turn(items: &[&str]) -> Vec<String> {
    let mut lines: Vec<String> = ["kill @e[type=!player]", "fill ~-2 ~-1 ~-2 ~4 ~-1 ~170 stone", "fill ~-2 ~ ~-2 ~4 ~3 ~170 air", "gamemode creative @s", "clear @s", ""].map(String::from).into();
    for item in items {
        // The cell is emptied first: BDS's drill world keeps what an earlier run put there.
        lines.extend(["tp @s ~ ~ ~2".into(), "fill ~1 ~ ~ ~1 ~1 ~ air".into(), format!("replaceitem entity @s slot.weapon.mainhand 0 {item}"), "!place 1 -1 0".into()]);
    }
    lines.extend(["clear @s", ""].map(String::from));
    lines
}

/// As `placing`: the blocks its run spoilt or named wrongly, and more that turn.
pub fn placing2() -> Vec<String> {
    placed_in_turn(&[
        "stone", "oak_trapdoor", "bed", "banner", "iron_chain", "crafter", "decorated_pot", "chiseled_bookshelf", "calibrated_sculk_sensor", "wooden_door", "iron_door", "soul_lantern", "soul_campfire", "soul_torch",
        "big_dripleaf", "lit_pumpkin", "glow_frame", "moss_carpet", "pink_petals", "sculk_shrieker", "trapped_chest", "mangrove_propagule", "small_amethyst_bud", "polished_basalt", "purpur_pillar", "deepslate",
        "ochre_froglight", "bamboo_block", "stripped_oak_log", "iron_trapdoor", "oak_button", "wooden_pressure_plate", "golden_rail", "detector_rail", "activator_rail", "heavy_core", "copper_bulb", "vault", "trial_spawner",
        "creaking_heart", "pale_hanging_moss", "resin_clump", "sniffer_egg", "torchflower", "pitcher_plant", "spore_blossom", "hanging_roots", "glow_lichen", "sculk_vein", "vine", "cocoa_beans", "oak_hanging_sign",
    ])
}

pub const ITEM_NAMES_SHOWN: &[&str] = &["InventorySlot", "InventoryContent"];

/// Old item names given, each beside the name it is thought to have now: one stack of two
/// where they are one item.
pub fn item_names() -> Vec<String> {
    let pairs = [
        ("chain", "iron_chain"), ("sign", "oak_sign"), ("boat", "oak_boat"), ("skull", "skeleton_skull"), ("record_13", "music_disc_13"), ("fish", "cod"), ("cooked_fish", "cooked_cod"), ("clownfish", "tropical_fish"),
        ("muttonraw", "mutton"), ("muttoncooked", "cooked_mutton"), ("appleenchanted", "enchanted_golden_apple"), ("reeds", "sugar_cane"), ("melon", "melon_slice"), ("speckled_melon", "glistering_melon_slice"),
        ("netherstar", "nether_star"), ("fireball", "fire_charge"), ("fireworks", "firework_rocket"), ("fireworkscharge", "firework_star"), ("horsearmorleather", "leather_horse_armor"),
        ("horsearmoriron", "iron_horse_armor"), ("carrotonastick", "carrot_on_a_stick"), ("emptymap", "empty_map"), ("totem", "totem_of_undying"), ("turtle_shell_piece", "turtle_scute"),
        ("chorus_fruit_popped", "popped_chorus_fruit"), ("netherbrick", "nether_brick"), ("gold_ingot", "gold_ingot"), ("stone_slab", "smooth_stone_slab"), ("planks", "oak_planks"), ("log", "oak_log"),
        ("wool", "white_wool"), ("stonebrick", "stone_bricks"), ("bed", "white_bed"), ("banner", "white_banner"), ("dye", "ink_sac"), ("coal", "coal"), ("door", "wooden_door"), ("nosuchitem", "stone"),
    ];
    let mut lines: Vec<String> = ["gamemode creative @s", "clear @s", ""].map(String::from).into();
    for (old, new) in pairs {
        lines.extend([format!("give @s {old} 1"), format!("give @s {new} 1"), "clear @s".into()]);
    }
    lines
}
