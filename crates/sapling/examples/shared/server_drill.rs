//! The server-command and structure-block drills of bds_command_probe: `/list`, `/reload`,
//! `/permission`, `/save` and `/stop` as an operator and every monster on `/difficulty peaceful`
//! (`server`); `/spreadplayers` failing, old coloured names and an overfull `/give` (`spread`);
//! a structure block's editor and placement (`structclick`); saves by power and from the editor
//! kept across a restart (`structpower`, then `structpower2` on the kept world); an animated load
//! with an entity and a waterlogged block exported (`structanim2`).

use crate::cmdblocks::{COMMAND_BLOCK_SHOWN, floor};

pub const SERVER_SHOWN: &[&str] = &["SetDifficulty", "AddEntity", "RemoveEntity", "InventorySlot", "InventoryContent", "AddItemEntity"];

/// Summoned before `/difficulty peaceful`: the packs' `monster` family and a few hostile others.
const KINDS: [&str; 46] = [
    "blaze", "bogged", "breeze", "cave_spider", "creaking", "creeper", "drowned", "elder_guardian", "enderman", "endermite", "evocation_illager", "ghast", "guardian", "husk",
    "magma_cube", "parched", "phantom", "piglin", "piglin_brute", "pillager", "ravager", "shulker", "silverfish", "skeleton", "slime", "spider", "stray", "vex", "vindicator",
    "warden", "witch", "wither_skeleton", "zoglin", "zombie", "zombie_nautilus", "zombie_pigman", "zombie_villager", "zombie_villager_v2", "hoglin", "skeleton_horse",
    "zombie_horse", "polar_bear", "camel_husk", "sulfur_cube", "wither", "ender_dragon",
];

fn lines<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

fn server() -> Vec<String> {
    let mut out = floor();
    out.extend(lines(["difficulty easy", "", "list", "reload", "reload all", "permission list", "permission reload", "save hold", "save query", "save resume", ""]));
    out.extend(KINDS.iter().enumerate().map(|(i, kind)| format!("summon {kind} ~{} ~ ~{}", 3 + (i % 6) * 2, (i / 6 * 2) as i32 - 6)));
    out.extend(lines(["", "", "difficulty peaceful", "", "", "", "summon zombie ~3 ~ ~", "", "", "kill @e[type=!player]", "difficulty easy", "", "stop", "", ""]));
    out
}

fn spread() -> Vec<String> {
    let mut out = floor();
    out.extend(lines(["tp @a[name=CmdWatcher] ~-2 ~ ~4", ""]));
    out.extend((0..20).map(|i| format!("summon cow ~{} ~ ~{}", 2 + i % 5, i / 5)));
    out.extend(lines(["", "spreadplayers ~ ~ 5 6 @e[type=cow]", "spreadplayers ~ ~ 1 6 @e[type=cow]", "spreadplayers ~ ~ 4 5 @e[type=cow]", "spreadplayers ~ ~ 0 0.5 @e[type=cow]"]));
    out.extend(lines(["kill @e[type=cow]", "spreadplayers ~ ~ 2 3 @a", "tp @s 0.5 ~ 0.5", "clear @s", ""]));
    for name in ["wool", "carpet", "concrete", "concrete_powder", "stained_glass", "stained_glass_pane", "stained_hardened_clay", "shulker_box", "glazed_terracotta", "coral_block"] {
        out.push(format!("give @s {name} 1 14"));
    }
    out.extend(lines(["clear @s", "", "give @s stone 5000", "", "", "clear @s stone 0 0", "kill @e[type=item]", "give @s ender_pearl 3000", "", "clear @s ender_pearl 0 0", "clear @s", ""]));
    out
}

/// A save block clicked by a creative and a survival operator; structure blocks placed in each
/// mode; the same after the actor is no longer an operator.
fn struct_click() -> Vec<String> {
    let mut out = floor();
    out.extend(lines(["setblock ~2 ~ ~ structure_block [\"structure_block_type\"=\"save\"]", "", "!click 2 0 0", "", "", "!close", ""]));
    out.extend(lines(["gamemode survival @s", "", "!click 2 0 0", "", "", "!close", ""]));
    for (mode, z) in [("creative", 2), ("survival", -2)] {
        out.extend([format!("gamemode {mode} @s"), "replaceitem entity @s slot.hotbar 0 structure_block".into(), String::new(), format!("!place 2 -1 {z}"), String::new(), String::new()]);
    }
    out.extend(lines(["gamemode creative @s", "fill ~2 ~ ~-2 ~2 ~ ~2 air", "setblock ~2 ~ ~ structure_block [\"structure_block_type\"=\"save\"]", "deop @s", "", "!click 2 0 0", "", ""]));
    out.extend(lines(["replaceitem entity @s slot.hotbar 0 structure_block", "", "!place 2 -1 2", "", ""]));
    out
}

/// Three 3x3x3 builds saved: one by power with `redstoneSaveMode` 0, one by power with 1, one
/// from the editor; each queried. `structpower2` loads and queries them after a restart.
fn struct_power() -> Vec<String> {
    let mut out = floor();
    for z in [0, 4, -4] {
        out.push(format!("fill ~4 ~ ~{} ~6 ~ ~{} stone", z - 1, z + 1));
        out.push(format!("setblock ~5 ~1 ~{z} glass"));
        out.push(format!("setblock ~3 ~ ~{z} structure_block [\"structure_block_type\"=\"save\"]"));
    }
    out.extend(lines(["", "!structblock 3 0 0 1 pw0 1 0 -1 3 3 3 0 0 0 0 0 0", "!structblock 3 0 4 1 pw1 1 0 -1 3 3 3 1 0 0 0 0 0", "!structblock 3 0 -4 1 pw2 1 0 -1 3 3 3 0 0 0 0 0 1", ""]));
    out.extend(lines(["setblock ~2 ~ ~ redstone_block", "setblock ~2 ~ ~4 redstone_block", "", "", "!structdata 3 0 0 pw0 2", "", "!structdata 3 0 0 pw1 2", "", "!structdata 3 0 0 pw2 2", ""]));
    out
}

fn struct_power_after() -> Vec<String> {
    let mut out = lines(["tp @s 0.5 ~ 0.5 -90 0", "", "!structdata 3 0 0 pw0 2", "", "!structdata 3 0 0 pw1 2", "", "!structdata 3 0 0 pw2 2", ""]);
    out.extend(lines(["structure load pw0 ~8 ~ ~-1", "structure load pw1 ~8 ~ ~3", "structure load pw2 ~8 ~ ~-5", "", "structure delete pw0", "structure delete pw1", "structure delete pw2", ""]));
    out
}

/// A one-layer build with an armour stand above it saved with entities and loaded layer by
/// layer over 2 s; then a water-logged stair saved by a structure block and exported.
fn struct_anim_entities() -> Vec<String> {
    let mut out = floor();
    out.extend(lines(["fill ~4 ~ ~-1 ~6 ~ ~1 stone", "summon armor_stand ~5 ~1 ~", "", "structure save ent ~4 ~ ~-1 ~6 ~2 ~1 true", "kill @e[type=armor_stand]", "fill ~4 ~ ~-1 ~6 ~2 ~1 air", ""]));
    out.extend(lines(["structure load ent ~4 ~ ~-1 0_degrees none layer_by_layer 2 true true", "", "", "", "", "kill @e[type=armor_stand]", "fill ~4 ~ ~-1 ~6 ~2 ~1 air", "structure delete ent", ""]));
    out.extend(lines(["fill ~4 ~ ~-1 ~6 ~ ~1 stone", "setblock ~5 ~1 ~ water", "setblock ~3 ~ ~ structure_block [\"structure_block_type\"=\"save\"]", "replaceitem entity @s slot.hotbar 0 oak_stairs", ""]));
    out.extend(lines(["tp @s 3.5 ~ 0.5 -90 0", "", "!place 2 0 0", "", "tp @s 0.5 ~ 0.5 -90 0", "", "!structdata 3 0 0 wet 0", "", "fill ~4 ~ ~-1 ~6 ~2 ~1 air", ""]));
    out
}

/// The lines and shown packets of the server drill `name`, if it is one.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    Some(match name {
        "server" => (server(), SERVER_SHOWN),
        "spread" => (spread(), SERVER_SHOWN),
        "structclick" => (struct_click(), COMMAND_BLOCK_SHOWN),
        "structpower" => (struct_power(), COMMAND_BLOCK_SHOWN),
        "structpower2" => (struct_power_after(), COMMAND_BLOCK_SHOWN),
        "structanim2" => (struct_anim_entities(), COMMAND_BLOCK_SHOWN),
        _ => return None,
    })
}
