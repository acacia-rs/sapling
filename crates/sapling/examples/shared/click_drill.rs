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
