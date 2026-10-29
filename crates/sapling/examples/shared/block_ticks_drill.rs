//! The block drill pass of bds_command_probe (docs/research/block-uses.md, blocks.md, fire.md,
//! world-ticks.md "Not drilled"): `bkuses` (note block instruments, tools on blocks), `notekeep1` and
//! `notekeep2` (a note across a restart: run the second with KEEP_WORLD=1), `bkblocks` (frames, blasts,
//! infested blocks, dripleaves), `bknether` (beds, wet sponge), `bkfire` (lava lighting fire), `bkticks`
//! (coral, sponges, a lectern hit, burning arrows and fire charges on TNT, chorus and amethyst) and
//! `brushing` (suspicious blocks set by command, brushed as a client holds a brush).

use super::redstone_parts::{ACTOR, set, stage};

const Y: i32 = ACTOR[1];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some((
        match name {
            "bkuses" => uses(),
            "notekeep1" => note_keep(true),
            "notekeep2" => note_keep(false),
            "bkblocks" => blocks(),
            "bknether" => nether(),
            "bkfire" => fire(),
            "bkticks" => ticks(),
            "brushing" => brushing(),
            _ => return None,
        },
        super::redstone_parts::SHOWN,
    ))
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

/// A note block east of the actor over each instrument's block, and under a head, clicked; then
/// tools on blocks there: axes, shovels, hoes, a bucket on full cauldrons.
fn uses() -> Vec<String> {
    let mut lines = stage([2, 20], [2, 20]);
    lines.push("clear @s".to_owned());
    for under in ["gold_block", "clay", "packed_ice", "white_wool", "bone_block", "iron_block", "soul_sand", "pumpkin", "emerald_block", "hay_block", "glowstone", "oak_planks", "sand", "glass", "stone"] {
        lines.extend([set(12, Y - 1, 10, under), set(12, Y, 10, "noteblock"), "!click 2 0 0".to_owned(), String::new()]);
    }
    lines.push(set(12, Y - 1, 10, "stone"));
    for head in ["zombie_head", "skeleton_skull", "creeper_head", "piglin_head"] {
        lines.extend([set(12, Y, 10, "noteblock"), set(12, Y + 1, 10, &format!("{head} [\"facing_direction\"=1]")), "!click 2 0 0 west".to_owned(), String::new()]);
    }
    lines.push(set(12, Y + 1, 10, "air"));
    let tools = [
        ("iron_axe", ["exposed_copper", "waxed_copper", "oak_wood", "warped_stem", "crimson_hyphae", "pale_oak_log", "weathered_cut_copper", "pumpkin"].as_slice()),
        ("iron_shovel", ["grass_block", "dirt", "podzol", "mycelium", "coarse_dirt", "dirt_with_roots", "farmland", "snow_layer"].as_slice()),
        ("iron_hoe", ["coarse_dirt", "dirt_with_roots", "grass_path", "podzol", "mycelium", "farmland"].as_slice()),
    ];
    for (tool, blocks) in tools {
        lines.push(hold(tool));
        for block in blocks {
            lines.extend([set(12, Y, 10, "air"), set(12, Y, 10, block), "!click 2 0 0 up".to_owned(), String::new()]);
        }
    }
    lines.push(hold("bucket"));
    for cauldron in ["[\"cauldron_liquid\"=\"water\",\"fill_level\"=6]", "[\"cauldron_liquid\"=\"lava\",\"fill_level\"=6]", "[\"cauldron_liquid\"=\"powder_snow\",\"fill_level\"=6]", "[\"cauldron_liquid\"=\"water\",\"fill_level\"=3]"] {
        lines.extend([set(12, Y, 10, &format!("cauldron {cauldron}")), "!click 2 0 0 up".to_owned(), String::new()]);
    }
    lines.extend(["clear @s".to_owned(), set(12, Y, 10, "air")]);
    lines
}

/// A note block east of the actor clicked thrice (`first`), or, after a restart, once more.
fn note_keep(first: bool) -> Vec<String> {
    let mut lines = vec!["gamemode creative @s".to_owned(), "clear @s".to_owned(), format!("tp @s {}.5 {Y} {}.5", ACTOR[0], ACTOR[2]), String::new()];
    if first {
        lines.extend([format!("fill 2 {} 2 20 {} 20 stone", Y - 1, Y - 1), format!("fill 2 {Y} 2 20 {} 20 air", Y + 3), set(12, Y, 10, "noteblock")]);
        lines.extend(["!click 2 0 0".to_owned(), String::new(), "!click 2 0 0".to_owned(), String::new(), "!click 2 0 0".to_owned(), String::new(), String::new()]);
    } else {
        lines.extend(["!click 2 0 0".to_owned(), String::new()]);
    }
    lines
}

/// An item frame on stone, the stone taken; a frame given a map; a charged anchor clicked;
/// TNT under water; infested stone broken by hand (tile drops on and off) and by a blast;
/// big dripleaves stood on, powered and shot.
fn blocks() -> Vec<String> {
    let mut lines = stage([2, 40], [2, 40]);
    lines.extend([set(11, Y, 12, "stone"), set(11, Y, 11, "frame [\"facing_direction\"=2]"), hold("diamond"), "!click 1 0 1".to_owned(), String::new(), set(11, Y, 12, "air"), String::new(), String::new()]);
    lines.extend([set(9, Y, 12, "stone"), set(9, Y, 11, "frame [\"facing_direction\"=2]"), hold("empty_map"), "!use".to_owned(), String::new(), "!click -1 0 1".to_owned(), String::new()]);
    // TNT in a glass tank of water at x 30.
    lines.extend([format!("fill 28 {Y} 28 32 {} 32 glass", Y + 4), format!("fill 29 {} 29 31 {} 31 water", Y + 1, Y + 3), format!("summon tnt 30.5 {} 30.5", Y + 2)]);
    lines.extend((0..5).map(|_| String::new()));
    // Infested stone: by hand in survival, with tile drops off, and by a blast.
    lines.extend([set(12, Y, 10, "infested_stone"), set(12, Y, 11, "infested_stone"), "clear @s".to_owned(), "gamemode survival @s".to_owned(), "!break 2 0 0".to_owned(), String::new(), String::new()]);
    lines.extend(["gamerule dotiledrops false".to_owned(), "!break 2 0 1".to_owned(), String::new(), String::new(), "gamerule dotiledrops true".to_owned(), "gamemode creative @s".to_owned()]);
    lines.extend([set(34, Y, 20, "infested_stone"), format!("summon tnt 35.5 {Y} 20.5"), String::new(), String::new(), String::new(), String::new(), "kill @e[type=silverfish]".to_owned(), "kill @e[type=item]".to_owned()]);
    // Big dripleaves at x 20 on dirt (on stone BDS broke them at the first update): one stood on,
    // one stood on and powered, one shot.
    for x in [20, 24, 28] {
        lines.extend([set(x, Y - 1, 22, "dirt"), set(x, Y, 22, "big_dripleaf [\"big_dripleaf_head\"=true]")]);
    }
    lines.extend([format!("tp @s 20.5 {} 22.5", Y + 1), String::new(), String::new(), String::new(), String::new(), String::new(), String::new(), String::new()]);
    lines.extend([format!("tp @s 24.5 {} 22.5", Y + 1), String::new(), set(25, Y, 22, "redstone_block"), String::new(), String::new(), set(25, Y, 22, "air")]);
    lines.extend([format!("tp @s {}.5 {Y} {}.5", ACTOR[0], ACTOR[2]), format!("summon arrow 28.5 {} 22.5", Y + 4), String::new(), String::new(), String::new(), String::new(), String::new(), String::new()]);
    // Last: a charged respawn anchor clicked (a blast, maybe fire).
    lines.extend([set(12, Y, 8, "respawn_anchor [\"respawn_anchor_charge\"=4]"), "clear @s".to_owned(), "!click 2 0 -2".to_owned(), String::new(), String::new(), String::new()]);
    lines
}

/// In the Nether: a bed clicked (a blast), again with respawnBlocksExplode off; a wet sponge set.
fn nether() -> Vec<String> {
    let mut lines = vec!["gamemode creative @s".to_owned(), "clear @s".to_owned(), "execute in nether run tp @s 0.5 101 0.5".to_owned(), String::new(), String::new()];
    lines.extend(["fill -8 100 -8 8 100 8 stone", "fill -8 101 -8 8 106 8 air", "tp @s 0.5 101 0.5", ""].map(String::from));
    lines.extend(["setblock 2 101 0 bed [\"direction\"=1]", "", "!click 2 0 0", "", "", "fill -8 100 -8 8 100 8 stone", "fill -8 101 -8 8 106 8 air"].map(String::from));
    lines.extend(["gamerule respawnblocksexplode false", "setblock 2 101 0 bed [\"direction\"=1]", "", "!click 2 0 0", "", "gamerule respawnblocksexplode true"].map(String::from));
    lines.extend(["setblock -3 101 0 wet_sponge", "", "", "", "", "execute in overworld run tp @s 10.5 -60 10.5", ""].map(String::from));
    lines
}

/// Lava with air over it in a stone pit, burnable blocks around it at each distance; fire ticks fast.
fn fire() -> Vec<String> {
    let mut lines = stage([18, 40], [18, 40]);
    lines.extend(["gamerule dofiretick true", "gamerule randomtickspeed 30"].map(String::from));
    lines.extend([format!("fill 26 {Y} 26 34 {} 34 stone", Y + 1), set(30, Y + 1, 30, "lava")]);
    for (x, y, z) in [(31, 1, 30), (29, 2, 30), (30, 2, 31), (32, 1, 32), (28, 2, 28)] {
        lines.push(set(x, Y + y, z, "oak_planks"));
    }
    lines.extend((0..40).map(|_| String::new()));
    lines.extend(["gamerule randomtickspeed 1", "gamerule dofiretick false", set(30, Y + 1, 30, "stone").as_str()].map(String::from));
    lines
}

/// Coral of each shape out of water; a sponge set by a pool with plants; a lectern given a book
/// then hit; an arrow dropped through fire onto TNT; a fire charge shot at TNT; chorus and budding
/// amethyst ticked fast.
fn ticks() -> Vec<String> {
    let mut lines = stage([2, 50], [2, 50]);
    lines.extend([set(22, Y, 22, "stone"), set(23, Y, 22, "tube_coral_block"), set(24, Y, 22, "tube_coral"), set(25, Y, 22, "tube_coral_fan"), set(22, Y, 23, "tube_coral_wall_fan [\"coral_direction\"=3]")]);
    lines.extend((0..8).map(|_| String::new()));
    // A pool 13 across at x 30..42, z 30..42, kelp and seagrass in it, a sponge set at its middle.
    lines.extend([format!("fill 29 {} 29 43 {} 43 stone", Y - 1, Y + 1), format!("fill 30 {Y} 30 42 {} 42 water", Y + 1), set(33, Y, 36, "seagrass"), set(36, Y, 33, "kelp")]);
    lines.extend([String::new(), set(36, Y, 36, "sponge"), String::new(), String::new(), String::new()]);
    // A lectern east of the actor given a book, then hit.
    lines.extend([set(12, Y, 10, "lectern [\"minecraft:cardinal_direction\"=\"west\"]"), hold("writable_book"), "!click 2 0 0".to_owned(), String::new(), "clear @s".to_owned(), "gamemode survival @s".to_owned(), "!break 2 0 0".to_owned()]);
    lines.extend([String::new(), String::new(), "gamemode creative @s".to_owned(), "kill @e[type=item]".to_owned()]);
    // TNT under fire held up by planks; an arrow dropped through the fire; a fire charge shot at TNT.
    lines.extend(["gamerule dofiretick false".to_owned(), "gamerule tntexplodes false".to_owned(), set(22, Y, 30, "tnt"), set(23, Y + 1, 30, "oak_planks"), set(22, Y + 1, 30, "fire")]);
    lines.extend([format!("summon arrow 22.5 {} 30.5", Y + 5), String::new(), String::new(), String::new()]);
    lines.extend([set(18, Y, 34, "dispenser [\"facing_direction\"=5]"), "replaceitem block 18 -60 34 slot.container 0 fire_charge 1".to_owned(), set(21, Y, 34, "tnt"), set(17, Y, 34, "redstone_block"), String::new(), String::new(), String::new()]);
    lines.extend(["gamerule tntexplodes true".to_owned(), "kill @e[type=tnt]".to_owned()]);
    // Chorus on end stone, budding amethyst in the open, nether vines: ticked fast for a while.
    lines.extend([set(44, Y, 22, "end_stone"), set(44, Y + 1, 22, "chorus_flower"), set(46, Y, 22, "budding_amethyst"), set(48, Y + 3, 22, "netherrack"), set(48, Y + 2, 22, "weeping_vines"), set(48, Y, 24, "twisting_vines")]);
    lines.push("gamerule randomtickspeed 200".to_owned());
    lines.extend((0..10).map(|_| String::new()));
    lines.push("gamerule randomtickspeed 1".to_owned());
    lines
}

/// Suspicious sand and gravel set by command, brushed from above as a client holds a brush: one
/// to the end, one a while and let go, then again; a brush on plain sand.
fn brushing() -> Vec<String> {
    let mut lines = stage([2, 20], [2, 20]);
    lines.extend(["gamemode survival @s".to_owned(), set(12, Y - 1, 10, "suspicious_sand"), set(11, Y - 1, 11, "suspicious_gravel"), set(9, Y - 1, 10, "sand"), "give @s brush".to_owned(), String::new()]);
    lines.extend(["!brush 2 -1 0 200".to_owned(), String::new(), "!brush 1 -1 1 30".to_owned()]);
    lines.extend((0..6).map(|_| String::new()));
    lines.extend(["!brush 1 -1 1 200".to_owned(), "!brush -1 -1 0 30".to_owned(), "gamemode creative @s".to_owned(), "clear @s".to_owned(), "kill @e[type=item]".to_owned()]);
    lines
}

/// Does `!brush x y z ticks`: holds use with the held brush on the top of the block that far from
/// the feet for that many ticks. False for any other line.
pub async fn act(actor: &mut acacia_bot::Bot, line: &str) -> bool {
    let Some(words) = line.strip_prefix("!brush ") else { return false };
    let numbers: Vec<i32> = words.split(' ').filter_map(|word| word.parse().ok()).collect();
    let (Some(feet), &[x, y, z, ticks]) = (actor.movement().and_then(|movement| movement.position()), &numbers[..]) else { return true };
    let pos = [feet[0].floor() as i32 + x, feet[1].floor() as i32 + y, feet[2].floor() as i32 + z];
    let top = [pos[0] as f32 + 0.5, pos[1] as f32 + 1.0, pos[2] as f32 + 0.5];
    println!("## look: {:?}", actor.look_at(top).await);
    let face = acacia_bot::interact::Face::Up;
    super::brush_hold::press(actor, pos, face);
    for _ in 0..ticks {
        drop(actor.wait_ticks(1).await);
        super::brush_hold::held(actor, pos, face);
    }
    super::brush_hold::release(actor, pos);
    println!("## let go");
    true
}
