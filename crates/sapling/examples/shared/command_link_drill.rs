//! The command-block drills of bds_command_probe that follow up `cmdchains` (`cmdlinks`,
//! `cmdlinks2`): chain blocks read back by clicks, which neighbour a chain goes on to, and which
//! commands a block may run. Offsets count from the actor's feet; it faces east (+x).

use crate::cmdblocks::{COMMAND_BLOCK_SHOWN, floor, lines};

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    let lines = match name {
        "cmdlinks" => links(),
        "cmdlinks2" => links_around(),
        _ => return None,
    };
    Some((lines, COMMAND_BLOCK_SHOWN))
}

/// What `cmdchains` left open: a chain block after an impulse block, read back by a click; the
/// chain block needing power and powered; an impulse block made always active unpowered; a
/// repeating block put by `/setblock`; what a run left in an entity (`op`, `list`), read by a
/// click; output to a player who is no operator.
fn links() -> Vec<String> {
    let mut out = floor();
    out.extend(lines([
        "setblock ~2 ~ ~ command_block [\"facing_direction\"=5]",
        "setblock ~3 ~ ~ chain_command_block [\"facing_direction\"=5]",
        "!cmdblock 2 0 0 0 1 0 0 0 1 say head",
        "!cmdblock 3 0 0 2 0 0 0 0 1 say link",
        "",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "!click 3 0 0",
        "!click 2 0 0",
        "",
        "setblock ~2 ~1 ~ air",
        "!cmdblock 3 0 0 2 1 0 0 0 1 say powered-link",
        "setblock ~3 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "!click 3 0 0",
        "",
        "setblock ~2 ~1 ~ air",
        "setblock ~3 ~1 ~ air",
        "fill ~2 ~ ~ ~3 ~ ~ air",
        "",
        "setblock ~2 ~ ~2 command_block [\"facing_direction\"=5]",
        "!cmdblock 2 0 2 0 0 0 0 0 1 say auto-unpowered",
        "",
        "",
        "!cmdblock 2 0 2 0 1 0 0 0 1 say back-to-power",
        "",
        "!cmdblock 2 0 2 0 0 0 0 0 1 say auto-again",
        "",
        "setblock ~2 ~ ~2 air",
        "setblock ~2 ~ ~4 repeating_command_block [\"facing_direction\"=5]",
        "",
        "setblock ~2 ~ ~4 air",
        "",
        "setblock ~2 ~ ~ command_block [\"facing_direction\"=5]",
        "!cmdblock 2 0 0 0 1 0 0 0 1 op CmdWatcher",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "!click 2 0 0",
        "",
        "setblock ~2 ~1 ~ air",
        "!cmdblock 2 0 0 0 1 0 0 0 1 list",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "!click 2 0 0",
        "",
        "setblock ~2 ~1 ~ air",
        "deop CmdWatcher",
        "!cmdblock 2 0 0 0 1 0 0 0 1 tp @e[type=armor_stand] ~ ~2 ~",
        "summon armor_stand ~6 ~ ~6",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "op CmdWatcher",
        "setblock ~2 ~1 ~ air",
        "setblock ~2 ~ ~ air",
        "kill @e[type=armor_stand]",
        "",
    ]));
    out
}

/// Which neighbour a chain goes on to: an impulse block facing east with a chain block on each
/// side but the west, where its power is; then one facing up with a chain block above. Then
/// which commands a block may run: each run's LastOutput, read back by a click.
fn links_around() -> Vec<String> {
    let mut out = floor();
    out.extend(lines([
        "gamerule maxcommandchainlength 65535",
        "setblock ~2 ~1 ~ command_block [\"facing_direction\"=5]",
        "setblock ~3 ~1 ~ chain_command_block [\"facing_direction\"=5]",
        "setblock ~2 ~ ~ chain_command_block [\"facing_direction\"=0]",
        "setblock ~2 ~2 ~ chain_command_block [\"facing_direction\"=1]",
        "setblock ~2 ~1 ~1 chain_command_block [\"facing_direction\"=3]",
        "setblock ~2 ~1 ~-1 chain_command_block [\"facing_direction\"=2]",
        "!cmdblock 2 1 0 0 1 0 0 0 1 say head",
        "!cmdblock 3 1 0 2 0 0 0 0 1 say east",
        "!cmdblock 2 0 0 2 0 0 0 0 1 say below",
        "!cmdblock 2 2 0 2 0 0 0 0 1 say above",
        "!cmdblock 2 1 1 2 0 0 0 0 1 say south",
        "!cmdblock 2 1 -1 2 0 0 0 0 1 say north",
        "",
        "setblock ~1 ~1 ~ redstone_block",
        "",
        "setblock ~1 ~1 ~ air",
        "fill ~2 ~ ~-1 ~3 ~2 ~1 air",
        "",
        "setblock ~2 ~ ~ command_block [\"facing_direction\"=1]",
        "setblock ~2 ~1 ~ chain_command_block [\"facing_direction\"=1]",
        "!cmdblock 2 0 0 0 1 0 0 0 1 say up-head",
        "!cmdblock 2 1 0 2 0 0 0 0 1 say up-link",
        "setblock ~1 ~ ~ redstone_block",
        "",
        "setblock ~1 ~ ~ air",
        "fill ~2 ~ ~ ~2 ~1 ~ air",
        "",
        "setblock ~2 ~ ~ command_block [\"facing_direction\"=5]",
    ]));
    for command in ["kick Nobody", "deop Nobody", "allowlist list", "setmaxplayers 10", "transfer Nobody 1.1.1.1", "place feature minecraft:oak_tree_feature ~ ~5 ~", "banlist", "seed", "version", "daylock true", "gamerule", "time query daytime"] {
        out.extend([format!("!cmdblock 2 0 0 0 1 0 0 0 1 {command}"), "setblock ~2 ~1 ~ redstone_block".into(), String::new(), "!click 2 0 0".into(), "setblock ~2 ~1 ~ air".into()]);
    }
    out.extend(lines(["setblock ~2 ~ ~ air", ""]));
    out
}
