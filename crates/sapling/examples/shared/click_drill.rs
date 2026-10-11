//! The clicks2 drill of bds_command_probe: clicks the first drill left out or left unmeasured.

/// The actor in creative, looking east, clicks one block after another a block east of it, as
/// `clicks` does: the block as set, what is put in its hand, and how often it clicks. The
/// charged respawn anchor comes last, since in the overworld it blows up.
pub fn clicks2() -> Vec<String> {
    let cases: [(&str, &str, usize); 22] = [
        ("composter [\"composter_fill_level\"=8]", "air", 1),
        ("cauldron [\"fill_level\"=6,\"cauldron_liquid\"=\"water\"]", "bucket", 1),
        ("cauldron [\"fill_level\"=6,\"cauldron_liquid\"=\"lava\"]", "bucket", 1),
        ("cauldron", "lava_bucket", 1),
        ("cauldron", "powder_snow_bucket", 1),
        ("cauldron", "potion", 2),
        ("rooted_dirt", "iron_hoe", 1),
        ("grass_block", "iron_hoe", 1),
        ("dirt", "potion", 1),
        ("campfire", "iron_shovel", 1),
        ("campfire [\"extinguished\"=true]", "flint_and_steel", 1),
        ("cake", "candle", 1),
        ("chiseled_bookshelf", "book", 2),
        ("decorated_pot", "stone", 2),
        ("frame [\"facing_direction\"=1]", "diamond", 3),
        ("end_portal_frame", "ender_eye", 1),
        ("crimson_stem", "iron_axe", 1),
        ("bamboo_block", "iron_axe", 1),
        ("cave_vines_head_with_berries", "air", 1),
        ("dragon_egg", "air", 1),
        ("lodestone", "compass", 1),
        ("respawn_anchor [\"respawn_anchor_charge\"=1]", "air", 1),
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

/// Scaffolding east of the actor: a column two high, a row run out from its top eight long,
/// then the column's foot taken away.
pub fn scaffold() -> Vec<String> {
    let mut lines: Vec<String> = ["fill ~-1 ~-1 ~7 ~12 ~-1 ~11 stone", "tp @s ~ ~ ~9 -90 0", "fill ~1 ~ ~-1 ~12 ~6 ~1 air", "setblock ~2 ~ ~ scaffolding", "setblock ~2 ~1 ~ scaffolding"].map(String::from).into();
    lines.extend((3..=10).map(|x| format!("setblock ~{x} ~1 ~ scaffolding")));
    lines.extend(["", "setblock ~2 ~ ~ air", "", "", "fill ~1 ~ ~-1 ~12 ~6 ~1 air", "kill @e[type=!player]"].map(String::from));
    lines
}

/// A second row of what random ticks change, set out as `grows` sets its own: cocoa on a jungle
/// log, nether wart, kelp in water, a vine on a wall, ice and a snow layer by a torch, a block
/// of copper, a torchflower crop and a pitcher crop; then stone is set beside a cactus.
pub fn grows2() -> Vec<String> {
    let mut lines: Vec<String> = ["fill ~-1 ~-1 ~7 ~10 ~-1 ~25 stone", "tp @s ~ ~ ~16 -90 0", "fill ~1 ~ ~-9 ~10 ~8 ~9 air", "time set noon"].map(String::from).into();
    let moist = "farmland [\"moisturized_amount\"=7]";
    let row: [(i32, Vec<(i32, i32, i32, String)>); 9] = [
        (-8, vec![(3, 0, 0, "jungle_log".into()), (4, 0, 0, "cocoa [\"direction\"=1,\"age\"=0]".into())]),
        (-6, vec![(3, -1, 0, "soul_sand".into()), (3, 0, 0, "nether_wart".into())]),
        (-4, vec![(2, 0, 0, "glass".into()), (4, 0, 0, "glass".into()), (3, 0, -1, "glass".into()), (3, 0, 1, "glass".into()), (3, 0, 0, "water".into()), (3, 0, 0, "kelp".into())]),
        (-2, vec![(3, 0, 0, "stone".into()), (3, 1, 0, "stone".into()), (3, 2, 0, "stone".into()), (4, 2, 0, "vine [\"vine_direction_bits\"=2]".into())]),
        (0, vec![(3, 0, 0, "ice".into()), (4, 0, 0, "torch".into()), (5, 0, 0, "snow_layer".into())]),
        (2, vec![(3, 0, 0, "copper_block".into())]),
        (4, vec![(3, -1, 0, moist.into()), (3, 0, 0, "torchflower_crop".into())]),
        (6, vec![(3, -1, 0, moist.into()), (3, 0, 0, "pitcher_crop".into())]),
        (8, vec![(3, -1, 0, "sand".into()), (3, 0, 0, "cactus".into())]),
    ];
    for (z, blocks) in row {
        lines.extend(blocks.into_iter().map(|(x, y, dz, block)| format!("setblock ~{x} ~{y} ~{} {block}", z + dz)));
    }
    lines.push("gamerule randomtickspeed 200".into());
    lines.extend(std::iter::repeat_n(String::new(), 60));
    lines.extend(["gamerule randomtickspeed 1", "setblock ~4 ~ ~8 stone", "", "fill ~1 ~ ~-9 ~10 ~8 ~9 air", "kill @e[type=item]"].map(String::from));
    lines
}

/// Twenty-five blocks of copper and a torchflower crop east of the actor, watched for a minute
/// at the rule's 1 and half a minute more at 20.
pub fn copper() -> Vec<String> {
    let mut lines: Vec<String> = [
        "fill ~-1 ~-1 ~7 ~10 ~-1 ~13 stone", "tp @s ~ ~ ~10 -90 0", "fill ~1 ~ ~-3 ~10 ~3 ~3 air", "time set noon", "gamerule randomtickspeed 1", "fill ~2 ~ ~-2 ~6 ~ ~2 copper_block",
        "setblock ~8 ~-1 ~ farmland [\"moisturized_amount\"=7]", "setblock ~8 ~ ~ torchflower_crop", "",
    ].map(String::from).into();
    lines.extend(std::iter::repeat_n(String::new(), 60));
    lines.push("gamerule randomtickspeed 20".into());
    lines.extend(std::iter::repeat_n(String::new(), 30));
    lines.extend(["gamerule randomtickspeed 1", "fill ~1 ~ ~-3 ~10 ~3 ~3 air", "setblock ~8 ~-1 ~ stone", "kill @e[type=item]"].map(String::from));
    lines
}

/// Redstone ore under the actor, in survival: set under its feet as it stands, then it drops
/// onto the ore from a block up, then an ore beside it is broken by hand.
pub fn oresteps() -> Vec<String> {
    let lines = [
        "fill ~-1 ~-1 ~7 ~3 ~-1 ~11 stone", "tp @s ~ ~ ~9 -90 0", "fill ~-1 ~ ~-1 ~3 ~4 ~1 air", "gamemode survival @s", "setblock ~ ~-1 ~ redstone_ore", "", "", "tp @s ~ ~1 ~", "", "",
        "setblock ~ ~-1 ~ redstone_ore", "setblock ~1 ~ ~ redstone_ore", "!break 1 0 0", "", "setblock ~ ~-1 ~ stone", "setblock ~1 ~ ~ air", "gamemode creative @s",
    ];
    lines.map(String::from).into()
}
