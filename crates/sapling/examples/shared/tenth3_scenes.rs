//! The lines of tenth3_drill.rs's scenes. Cells are absolute; `!` offsets count from the actor's
//! feet, at 10 -60 10 unless a scene moves it.

use crate::redstone_parts::{set, stage};

fn owned<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn wait(lines: &mut Vec<String>, seconds: usize) {
    lines.extend(std::iter::repeat_n(String::new(), seconds));
}

fn hold(item: &str) -> String {
    format!("replaceitem entity @s slot.weapon.mainhand 0 {item}")
}

/// 25 blocks of copper six apart (none within another's four) under `randomtickspeed` 100 for a minute.
pub fn copper() -> Vec<String> {
    let mut lines = stage([0, 30], [0, 30]);
    lines.push("gamerule randomtickspeed 100".into());
    for x in (2..=26).step_by(6) {
        for z in (2..=26).step_by(6) {
            lines.push(set(x, -60, z, "copper_block"));
        }
    }
    wait(&mut lines, 60);
    lines.extend(owned(["gamerule randomtickspeed 1", "fill 0 -60 0 30 -60 30 air", ""]));
    lines
}

/// At `randomtickspeed` 40 for a minute: 12 dry coral blocks, 12 twisting and 12 weeping vines of
/// age 0, 7 chorus flowers on end stone and 9 budding amethyst, all set at once.
pub fn odds() -> Vec<String> {
    let mut lines = stage([0, 30], [0, 24]);
    lines.extend(owned(["fill 0 -54 0 30 -49 24 air", "fill 0 -48 9 30 -48 11 stone", "gamerule randomtickspeed 40"]));
    for x in (2..=24).step_by(2) {
        lines.push(set(x, -60, 2, "tube_coral_block"));
        lines.push(set(x, -60, 6, "twisting_vines [\"twisting_vines_age\"=0]"));
        lines.push(set(x, -49, 10, "weeping_vines [\"weeping_vines_age\"=0]"));
    }
    for x in (2..=26).step_by(4) {
        lines.extend([set(x, -61, 14, "end_stone"), set(x, -60, 14, "chorus_flower")]);
    }
    for x in (2..=26).step_by(3) {
        lines.push(set(x, -60, 20, "budding_amethyst"));
    }
    wait(&mut lines, 60);
    lines.extend(owned(["gamerule randomtickspeed 1", "fill 0 -60 0 30 -48 24 air", "fill 0 -61 0 30 -61 24 stone", ""]));
    lines
}

/// A dry sponge set at the end of a channel of 14 water sources, then in the middle of a 17x17 pool.
pub fn sponge() -> Vec<String> {
    let mut lines = stage([0, 40], [0, 40]);
    lines.extend(owned(["fill 21 -60 9 37 -60 11 stone", "fill 22 -60 10 36 -60 10 water", "fill 1 -60 21 19 -60 39 stone", "fill 2 -60 22 18 -60 38 water", ""]));
    lines.extend(owned(["setblock 22 -60 10 sponge", "", "", "setblock 10 -60 30 sponge", "", "", "fill 0 -60 0 40 -60 40 air", ""]));
    lines
}

/// Survival: a lectern with a book punched twice; a compass on a lodestone and its target asked
/// for, the lodestone gone; a decorated pot and a flower pot clicked and broken; a comparator past a
/// stone reading a frame turned eight times; big dripleaf stood on and shot; an anchor clicked with dirt.
pub fn uses() -> Vec<String> {
    let mut lines = stage([0, 20], [0, 20]);
    lines.extend(owned(["gamerule falldamage false", "gamemode survival @s"]));
    lines.extend([set(12, -60, 10, "lectern"), hold("writable_book"), "!click 2 0 0".into(), String::new(), "clear @s".into()]);
    lines.extend(owned(["!punch 2 0 0", "", "!punch 2 0 0", "", ""]));
    lines.extend([set(8, -60, 10, "lodestone"), hold("compass"), "!click -2 0 0".into(), String::new(), "!track held".into(), String::new()]);
    lines.extend(owned(["gamemode creative @s", "clear @s", ""]));
    lines.extend([hold("compass"), "!click -2 0 0".into(), String::new(), "!track held".into(), String::new()]);
    lines.extend(owned(["setblock 8 -60 10 air", "", "!track held", "", "!track 99", "", "gamemode survival @s", "clear @s"]));
    lines.extend([set(8, -60, 12, "decorated_pot"), hold("diamond 3"), "!click -2 0 2".into(), String::new(), "!click -2 0 2".into(), String::new(), "clear @s".into()]);
    lines.extend(owned(["!click -2 0 2", "", "!break -2 0 2", "", ""]));
    lines.extend([set(8, -60, 8, "flower_pot"), hold("poppy"), "!click -2 0 -2".into(), String::new(), "clear @s".into(), "!click -2 0 -2".into(), String::new()]);
    lines.extend([hold("dandelion"), "!click -2 0 -2".into(), String::new(), "!break -2 0 -2".into(), String::new(), "clear @s".into()]);
    lines.extend([set(11, -60, 7, "redstone_wire"), set(12, -60, 7, "unpowered_comparator [\"minecraft:cardinal_direction\"=\"east\"]"), set(13, -60, 7, "stone")]);
    lines.extend([set(14, -60, 7, "frame [\"facing_direction\"=5]"), hold("diamond"), String::new()]);
    for _ in 0..9 {
        lines.extend(["!click 4 0 -3 east".into(), String::new()]);
    }
    lines.extend(owned(["clear @s", "fill 11 -60 7 14 -60 7 air", "kill @e[type=item]", ""]));
    lines.extend([set(12, -60, 14, "big_dripleaf [\"big_dripleaf_head\"=true]"), set(14, -60, 14, "big_dripleaf [\"big_dripleaf_head\"=false]"), set(14, -59, 14, "big_dripleaf [\"big_dripleaf_head\"=true]")]);
    lines.extend([set(16, -60, 14, "big_dripleaf [\"big_dripleaf_head\"=true]"), "tp @s 12.5 -59.0625 14.5".into()]);
    wait(&mut lines, 7);
    lines.push("tp @s 14.5 -58.0625 14.5".into());
    wait(&mut lines, 7);
    lines.extend(owned(["tp @s 10.5 -60 10.5", "summon arrow 16.5 -57 14.5", ""]));
    wait(&mut lines, 6);
    lines.extend([set(6, -60, 10, "respawn_anchor"), hold("dirt 4"), "!click -4 0 0 east".into(), String::new(), hold("glowstone 4"), "!click -4 0 0 east".into(), String::new()]);
    lines.extend([hold("dirt 4"), "!click -4 0 0 east".into(), String::new(), String::new(), "gamemode creative @s".into(), "clear @s".into(), "kill @e[type=!player]".into()]);
    lines.extend(owned(["fill 0 -60 0 20 -55 20 air", "fill 0 -61 0 20 -61 20 stone", ""]));
    lines
}

/// In the Nether on an obsidian floor at y 99: a charged anchor clicked with dirt, then empty-handed;
/// a bed's foot clicked.
// Four seconds in the Nether before the fill: sapling's first run filled before its columns were in.
pub fn nether() -> Vec<String> {
    let mut lines = stage([2, 20], [2, 20]);
    lines.extend(owned(["execute in nether run tp @s 0.5 100 0.5", "", "", "", "", "fill -6 99 -6 12 99 12 obsidian", "fill -6 100 -6 12 106 12 air", "tp @s 0.5 100 0.5", ""]));
    lines.extend(owned(["gamemode survival @s", "setblock 3 100 0 respawn_anchor [\"respawn_anchor_charge\"=1]", ""]));
    lines.extend([hold("dirt 4"), "!click 3 0 0 west".into(), String::new(), "clear @s".into(), "!click 3 0 0 west".into(), String::new(), String::new()]);
    lines.extend(owned(["gamemode creative @s", "setblock 0 100 4 bed [\"direction\"=0]", "", "!click 0 0 3", "", "", "fill -6 100 -6 12 106 12 air", "execute in overworld run tp @s 10.5 -60 10.5", "", ""]));
    lines
}

/// Chests placed in pairs by hand; one half set to air, to stone; one half and a whole pair cloned;
/// a chest placed between two single ones.
pub fn chests() -> Vec<String> {
    let mut lines = stage([2, 20], [2, 20]);
    lines.extend([hold("chest"), "!place 2 -1 0".into(), String::new(), "!place 2 -1 1".into(), String::new(), "setblock 12 -60 11 air".into(), String::new(), String::new()]);
    lines.extend(owned(["!place 2 -1 1", "", "setblock 12 -60 11 stone", "", "", "setblock 12 -60 11 air", "!place 2 -1 1", ""]));
    lines.extend(owned(["clone 12 -60 10 12 -60 10 14 -60 10", "", "clone 12 -60 10 12 -60 11 16 -60 10", "", ""]));
    lines.extend(owned(["tp @s 9.5 -60 14.5", "", "!place 3 -1 -1", "", "!place 3 -1 1", "", "!place 3 -1 0", "", "", "clear @s", "fill 2 -60 2 20 -60 20 air", ""]));
    lines
}

/// A bell left 3 s and cloned; rung, cloned a second on and 3 s on.
pub fn bells() -> Vec<String> {
    let mut lines = stage([2, 20], [2, 20]);
    lines.push(set(12, -60, 10, "bell"));
    wait(&mut lines, 3);
    lines.extend(owned(["clone 12 -60 10 12 -60 10 14 -60 13", "", "!click 2 0 0", "clone 12 -60 10 12 -60 10 16 -60 13", "", "clone 12 -60 10 12 -60 10 17 -60 13"]));
    lines.extend(owned(["clone 12 -60 10 12 -60 10 18 -60 13", "", "", "clone 12 -60 10 12 -60 10 19 -60 13", "", "fill 2 -60 2 20 -60 20 air", ""]));
    lines
}

/// Suspicious sand and gravel brushed in survival, the dust, slots and entity shown.
pub fn brush() -> Vec<String> {
    let mut lines = stage([2, 20], [2, 20]);
    lines.extend([
        "gamemode survival @s".into(), set(12, -61, 10, "suspicious_sand"), set(11, -61, 11, "suspicious_gravel"), "give @s brush".into(), String::new(),
        "!brush 2 -1 0 200".into(), String::new(), "!brush 1 -1 1 30".into(),
    ]);
    wait(&mut lines, 6);
    lines.extend(owned(["!brush 1 -1 1 200", "", "gamemode creative @s", "clear @s", "kill @e[type=item]", "fill 2 -61 2 20 -61 20 stone", ""]));
    lines
}

/// Bone meal clicked three times in survival on each plant, the actor two west of it.
pub fn meal() -> Vec<String> {
    let mut lines = stage([0, 70], [4, 16]);
    lines.extend(owned(["fill 0 -54 4 70 -40 16 air", "gamemode survival @s"]));
    let stations: [(&[&str], &[&str]); 11] = [
        (&["fill ~1 ~ ~-1 ~3 ~3 ~1 glass", "fill ~2 ~ ~ ~2 ~3 ~ water", "setblock ~2 ~-1 ~ sand", "setblock ~2 ~ ~ seagrass"], &["2 0 0"]),
        (&["fill ~1 ~ ~-1 ~3 ~5 ~1 glass", "fill ~2 ~ ~ ~2 ~5 ~ water", "setblock ~2 ~-1 ~ sand", "setblock ~2 ~ ~ kelp [\"kelp_age\"=0]"], &["2 0 0"]),
        (&["fill ~3 ~ ~ ~3 ~3 ~ stone", "setblock ~2 ~2 ~ vine [\"vine_direction_bits\"=8]"], &["2 2 0"]),
        (&["setblock ~2 ~ ~ twisting_vines [\"twisting_vines_age\"=0]"], &["2 0 0"]),
        (&["setblock ~2 ~4 ~ stone", "setblock ~2 ~3 ~ weeping_vines [\"weeping_vines_age\"=0]"], &["2 3 0"]),
        (&["setblock ~2 ~ ~ jungle_log", "setblock ~1 ~ ~ cocoa [\"direction\"=1]"], &["1 0 0"]),
        (&["setblock ~2 ~-1 ~ moss_block"], &["2 -1 0"]),
        (&["setblock ~2 ~2 ~ dirt_with_roots"], &["2 2 0 down"]),
        (&["setblock ~2 ~-1 ~ podzol", "setblock ~2 ~ ~ brown_mushroom", "setblock ~2 ~-1 ~2 podzol", "setblock ~2 ~ ~2 red_mushroom"], &["2 0 0", "2 0 2"]),
        (&["setblock ~2 ~-1 ~ sand", "setblock ~3 ~-1 ~ water", "fill ~2 ~ ~ ~2 ~2 ~ reeds"], &["2 0 0", "2 2 0"]),
        (&["setblock ~2 ~-1 ~ dirt", "fill ~2 ~ ~ ~2 ~15 ~ bamboo"], &["2 0 0"]),
    ];
    for (i, (setup, clicks)) in stations.iter().enumerate() {
        lines.extend([format!("tp @s {}.5 -60 10.5", 2 + 6 * i), String::new()]);
        lines.extend(setup.iter().map(|line| line.to_string()));
        lines.extend([hold("bone_meal 64"), String::new()]);
        for click in *clicks {
            for _ in 0..3 {
                lines.extend([format!("!click {click}"), String::new()]);
            }
        }
    }
    lines.extend(owned(["gamemode creative @s", "clear @s", "kill @e[type=item]", "fill 0 -60 4 70 -40 16 air", "fill 0 -61 4 70 -61 16 stone", ""]));
    lines
}

/// A comparator reading a frame behind a stone, the frame given a diamond and turned eight times
/// by the actor standing east of it, in survival.
pub fn frame() -> Vec<String> {
    let mut lines = stage([2, 20], [2, 20]);
    lines.extend([set(11, -60, 7, "redstone_wire"), set(12, -60, 7, "unpowered_comparator [\"minecraft:cardinal_direction\"=\"east\"]"), set(13, -60, 7, "stone")]);
    lines.extend([set(14, -60, 7, "frame [\"facing_direction\"=5]"), "tp @s 16.5 -60 7.5".into(), "gamemode survival @s".into(), hold("diamond"), String::new()]);
    for _ in 0..9 {
        lines.extend(owned(["!click -2 0 0 east", ""]));
    }
    lines.extend(owned(["gamemode creative @s", "clear @s", "fill 11 -60 7 14 -60 7 air", "kill @e[type=item]", ""]));
    lines
}

/// Bone meal clicked three times in survival on sea grass and on kelp, each in a pit of water two
/// deep, the actor two west of its rim.
pub fn water_meal() -> Vec<String> {
    let mut lines = stage([2, 20], [2, 20]);
    lines.push("gamemode survival @s".into());
    for (x, plant) in [(4, "seagrass"), (10, "kelp [\"kelp_age\"=0]")] {
        lines.extend([format!("tp @s {}.5 -60 10.5", x - 2), String::new()]);
        lines.extend([format!("fill {} -63 9 {} -61 11 stone", x - 1, x + 1), format!("setblock {x} -63 10 sand"), format!("fill {x} -62 10 {x} -61 10 water")]);
        lines.extend([format!("setblock {x} -62 10 {plant}"), hold("bone_meal 64"), String::new()]);
        for _ in 0..3 {
            lines.extend(owned(["!click 2 -2 0 up", ""]));
        }
    }
    lines.extend(owned(["gamemode creative @s", "clear @s", "fill 2 -63 2 20 -62 20 dirt", "fill 2 -61 2 20 -61 20 stone", "fill 2 -60 2 20 -55 20 air", ""]));
    lines
}

/// Repeating blocks made always active with a delay of 10, four times; a command block minecart
/// given `say cart` and set on a powered activator rail for 3 s, then off it.
pub fn command_blocks() -> Vec<String> {
    let mut lines = stage([2, 20], [2, 20]);
    lines.extend(owned(["gamerule commandblocksenabled true", "gamerule commandblockoutput true"]));
    for z in [0, 2, 4, 6] {
        lines.extend([format!("setblock 12 -60 {} repeating_command_block", 6 + z), String::new(), format!("!cmdblock 2 0 {} 1 0 0 10 0 1 say rep{z}", z - 4), String::new(), String::new()]);
        lines.extend([format!("setblock 12 -60 {} air", 6 + z), String::new()]);
    }
    lines.extend(owned(["setblock 15 -61 10 redstone_block", "setblock 15 -60 10 activator_rail", "summon command_block_minecart 15.5 -60 10.5", "", "!cartcmd say cart", "", "", ""]));
    lines.extend(owned(["setblock 15 -61 10 stone", "", "", "kill @e[type=command_block_minecart]", "fill 2 -60 2 20 -60 20 air", ""]));
    lines
}

/// Turned states: a row of mushroom blocks of each `huge_mushroom_bits`, dead coral wall fans of
/// each `coral_direction`, crafters and bells, saved and loaded at each turn and mirror; a build with
/// an armour stand loaded layer by layer; a structure block exporting a live coral fan with its water.
pub fn turns() -> Vec<String> {
    let mut lines = stage([0, 40], [0, 80]);
    for bits in 0..16 {
        lines.push(set(2 + bits, -60, 2, &format!("brown_mushroom_block [\"huge_mushroom_bits\"={bits}]")));
    }
    // Each coral fan between stones on all four sides, so that every direction holds.
    lines.extend(owned(["fill 2 -60 3 13 -60 3 stone", "fill 2 -60 5 13 -60 5 stone", "setblock 3 -60 4 stone", "setblock 5 -60 4 stone"]));
    for (x, direction) in [(2, 0), (4, 1), (6, 2), (13, 3)] {
        lines.push(set(x, -60, 4, &format!("dead_tube_coral_wall_fan [\"coral_direction\"={direction}]")));
        lines.push(set(14 + direction, -60, 4, &format!("bell [\"direction\"={direction}]")));
    }
    for (i, orientation) in ["north_up", "east_up", "south_up", "up_north", "down_east", "west_up"].iter().enumerate() {
        lines.push(set(7 + i as i32, -60, 4, &format!("crafter [\"orientation\"=\"{orientation}\"]")));
    }
    lines.extend(owned(["structure save t3turn 2 -60 2 17 -60 5", ""]));
    let loads = [("2 -60 10", "90_degrees", "none"), ("2 -60 30", "180_degrees", "none"), ("2 -60 50", "270_degrees", "none"), ("2 -60 70", "0_degrees", "x"), ("24 -60 10", "0_degrees", "z")];
    for (at, rotation, mirror) in loads {
        lines.extend([format!("structure load t3turn {at} {rotation} {mirror}"), String::new()]);
    }
    lines.extend(owned(["fill 2 -60 2 17 -60 5 air", "fill 2 -60 10 20 -60 80 air", "fill 24 -60 10 40 -60 14 air", "", "fill 2 -60 2 4 -58 4 stone", "fill 3 -59 3 3 -58 3 air", "summon armor_stand 3.5 -59 3.5"]));
    lines.extend(owned(["structure save t3anim 2 -60 2 4 -58 4 true", "kill @e[type=armor_stand]", "fill 2 -60 2 4 -58 4 air", "", "structure load t3anim 6 -60 2 0_degrees none layer_by_layer 3 true true", ""]));
    wait(&mut lines, 4);
    lines.extend(owned(["kill @e[type=armor_stand]", "fill 6 -60 2 8 -58 4 air", "structure delete t3anim", "structure delete t3turn", ""]));
    // A coral fan holding water in a pocket of stone, inside the box BDS exported (12 -60 9, 3 wide).
    lines.extend(owned(["fill 13 -61 9 15 -59 11 stone", "setblock 14 -60 10 tube_coral_fan", "setblock 12 -60 10 structure_block [\"structure_block_type\"=\"save\"]", "", "!structblock 2 0 0 1 t3liquid 1 0 -1 3 3 3 0 0 0 0 0 0", ""]));
    lines.extend(owned(["!structdata 2 0 0 t3liquid 0", "", "fill 12 -60 9 16 -58 11 air", ""]));
    lines
}

/// An 8x1x8 floor of stone saved and loaded at integrity 50 under seeds 1, 1 again, 2, abc and 12345.
pub fn integrity() -> Vec<String> {
    let mut lines = stage([0, 20], [0, 70]);
    lines.extend(owned(["fill 2 -60 2 9 -60 9 stone", "structure save t3integ 2 -60 2 9 -60 9", "fill 2 -60 2 9 -60 9 air", ""]));
    for (i, seed) in ["1", "1", "2", "abc", "12345"].iter().enumerate() {
        lines.extend([format!("structure load t3integ 2 -60 {} 0_degrees none true true false 50 {seed}", 12 + 10 * i), String::new()]);
    }
    lines.extend(owned(["fill 2 -60 12 9 -60 60 air", "structure delete t3integ", ""]));
    lines
}

/// An end gateway set in the overworld, watched for 25 s; one set in the End far from the island
/// and walked into.
pub fn gateway() -> Vec<String> {
    let mut lines = stage([2, 20], [2, 20]);
    lines.push(set(14, -57, 10, "end_gateway"));
    wait(&mut lines, 25);
    lines.extend(owned(["setblock 14 -57 10 air", "execute in the_end run tp @s 301.5 200 303.5", "", "", "fill 300 79 300 306 79 306 obsidian", "fill 300 80 300 306 84 306 air", "tp @s 301.5 80 303.5", ""]));
    lines.extend(owned(["setblock 304 81 303 end_gateway", "", "tp @s 304.5 81 303.5", "", "", "", "execute in overworld run tp @s 10.5 -60 10.5", "", ""]));
    lines
}

/// A trial spawner set by command, given a zombie egg, waited by in survival, its zombies killed
/// for the reward, then the actor omened; a vault opened with a trial key.
pub fn trial() -> Vec<String> {
    let mut lines = stage([0, 30], [0, 30]);
    // BDS's trial spawner spawned nothing under `domobspawning false` (bds-t3trial1.txt).
    lines.extend(owned(["gamerule domobspawning true", "difficulty easy", "effect @s resistance 300 255 true", "setblock 14 -60 10 trial_spawner", "", "replaceitem entity @s slot.weapon.mainhand 0 zombie_spawn_egg"]));
    lines.extend(owned(["!click 4 0 0 west", "", "clear @s", "gamemode survival @s"]));
    wait(&mut lines, 12);
    lines.extend(owned(["kill @e[type=zombie]", ""]));
    wait(&mut lines, 6);
    lines.extend(owned(["effect @s trial_omen 120 0", ""]));
    wait(&mut lines, 12);
    lines.extend(owned(["kill @e[type=zombie]", "", "gamemode creative @s", "setblock 9 -60 12 vault", "", "replaceitem entity @s slot.weapon.mainhand 0 trial_key"]));
    lines.extend(owned(["!click -1 0 2 north", ""]));
    wait(&mut lines, 5);
    // The vault's whole entity, server data too, by a structure block's export (origin the block's cell less one z).
    lines.extend(owned(["setblock 8 -60 12 structure_block [\"structure_block_type\"=\"save\"]", "", "!structdata -2 0 2 t3vault 0", ""]));
    lines.extend(owned(["clear @s", "effect @s clear", "gamerule domobspawning false", "kill @e[type=!player]", "fill 0 -60 0 30 -55 30 air", ""]));
    lines
}
