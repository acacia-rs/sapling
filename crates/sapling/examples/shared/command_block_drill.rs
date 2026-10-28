//! The command-block drills of bds_command_probe (`cmdblocks`, `cmdchains`, `cmdrules`): blocks put
//! by `/setblock` and by hand, set up by the packet the client's editor sends (`!cmdblock`,
//! `!cmdname`), powered by blocks of redstone, chained and read by comparators, under each of their
//! game rules. Offsets count from the actor's feet; it faces east (+x).

use acacia_bot::Bot;
use acacia_bot::proto::packets::{CommandBlockUpdate, CommandBlockUpdateContent, CommandBlockUpdateContentTrue, CommandBlockUpdateContentTrueMode};
use acacia_bot::proto::types::BlockCoordinates;

pub const COMMAND_BLOCK_SHOWN: &[&str] = &[
    "UpdateBlock", "UpdateSubchunkBlocks", "BlockEntityData", "ContainerOpen", "InventorySlot", "LevelSoundEvent", "StructureTemplateDataExportResponse", "AddEntity", "RemoveEntity",
];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    let lines = match name {
        "cmdblocks" => blocks(),
        "cmdchains" => chains(),
        "cmdrules" => rules(),
        _ => return None,
    };
    Some((lines, COMMAND_BLOCK_SHOWN))
}

pub fn lines<const N: usize>(lines: [&str; N]) -> Vec<String> {
    lines.map(String::from).into()
}

/// The command-block rules at their defaults: the test BDS's world kept `maxcommandchainlength`
/// -1 from an earlier drill, which ran no chain link (docs/research/command-blocks.md).
fn default_rules() -> Vec<String> {
    lines(["gamerule maxcommandchainlength 65535", "gamerule commandblocksenabled true", "gamerule commandblockoutput true"])
}

pub fn floor() -> Vec<String> {
    lines([
        "kill @e[type=!player]",
        "gamerule domobspawning false",
        "gamerule randomtickspeed 0",
        "gamemode creative @s",
        "tp @s 0.5 ~ 0.5 -90 0",
        "fill ~-2 ~-1 ~-6 ~12 ~-1 ~8 stone",
        "fill ~-2 ~ ~-6 ~12 ~6 ~8 air",
        "clear @s",
        "",
    ])
}

/// Placing: by `/setblock` (its entity as sent), by hand in survival and in creative, a click on
/// it in both; the editor's packet, power from above, power taken away and given again; output
/// tracked or not, a custom name, `@s` and `~` of the block, and commands above a block's level.
fn blocks() -> Vec<String> {
    let mut out = floor();
    out.extend(lines([
        "setblock ~2 ~ ~ command_block [\"facing_direction\"=5]",
        "",
        "!cmdblock 2 0 0 0 1 0 0 0 1 say hello",
        "",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "!cmdblock 2 0 0 0 1 0 0 0 0 testfor @e[type=cow]",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "!cmdblock 2 0 0 0 1 0 0 0 1 tp @e[type=armor_stand] ~ ~2 ~",
        "summon armor_stand ~6 ~ ~6",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "!cmdblock 2 0 0 0 1 0 0 0 1 say @s",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "!cmdname 2 0 0 Bob say named",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "!cmdblock 2 0 0 0 1 0 0 0 1 op CmdWatcher",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "!cmdblock 2 0 0 0 1 0 0 0 1 setblock ~ ~2 ~ glass",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "!cmdblock 2 0 0 0 1 0 0 0 1 nosuchcommand",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        // Always active: runs once on its own, then on nothing.
        "!cmdblock 2 0 0 0 0 0 0 0 1 say auto",
        "",
        "",
        "setblock ~2 ~1 ~ air",
        "setblock ~2 ~ ~ air",
        "kill @e[type=armor_stand]",
        "",
        // By hand: survival, then creative; clicked in both.
        "gamemode survival @s",
        "replaceitem entity @s slot.hotbar 0 command_block",
        "",
        "!placeat 3 -1 0 0.5 1 0.5",
        "",
        "gamemode creative @s",
        "",
        "!placeat 3 -1 0 0.5 1 0.5",
        "",
        "!click 3 0 0",
        "",
        "gamemode survival @s",
        "!click 3 0 0",
        "",
        "gamemode creative @s",
        "!break 3 0 0",
        "",
        "clear @s",
        "",
    ]));
    out
}

/// Chains: an impulse block east into two chain blocks, the second conditional; the first chain
/// block failing; delays on the impulse and the chain; a repeating block always active with a
/// delay, then executing on its first tick, then needing power; a loop of chains; a comparator
/// reading each.
fn chains() -> Vec<String> {
    let mut out = floor();
    out.extend(default_rules());
    out.extend(lines([
        "setblock ~2 ~ ~ command_block [\"facing_direction\"=5]",
        "setblock ~3 ~ ~ chain_command_block [\"facing_direction\"=5]",
        "setblock ~4 ~ ~ chain_command_block [\"facing_direction\"=5,\"conditional_bit\"=true]",
        "setblock ~5 ~ ~ chain_command_block [\"facing_direction\"=5]",
        "!cmdblock 2 0 0 0 1 0 0 0 1 setblock ~ ~-1 ~ red_wool",
        "!cmdblock 3 0 0 2 0 0 0 0 1 setblock ~ ~-1 ~ orange_wool",
        "!cmdblock 4 0 0 2 0 1 0 0 1 setblock ~ ~-1 ~ yellow_wool",
        "!cmdblock 5 0 0 2 0 0 0 0 1 setblock ~ ~-1 ~ lime_wool",
        "setblock ~2 ~ ~-1 unpowered_comparator [\"minecraft:cardinal_direction\"=\"south\"]",
        "setblock ~2 ~ ~-2 redstone_wire",
        "setblock ~4 ~ ~-1 unpowered_comparator [\"minecraft:cardinal_direction\"=\"south\"]",
        "setblock ~4 ~ ~-2 redstone_wire",
        "",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "fill ~2 ~-1 ~ ~5 ~-1 ~ stone",
        "!cmdblock 3 0 0 2 0 0 0 0 1 testfor @e[type=cow]",
        "",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "fill ~2 ~-1 ~ ~5 ~-1 ~ stone",
        "!cmdblock 2 0 0 0 1 0 5 0 1 setblock ~ ~-1 ~ red_wool",
        "!cmdblock 3 0 0 2 0 0 3 0 1 setblock ~ ~-1 ~ orange_wool",
        "!cmdblock 5 0 0 2 0 0 0 0 1 setblock ~ ~-1 ~ lime_wool",
        "",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "",
        "setblock ~2 ~1 ~ air",
        "fill ~2 ~-1 ~ ~5 ~-1 ~ stone",
        // Repeating, always active, every 10 ticks into a chain.
        "setblock ~2 ~ ~ repeating_command_block [\"facing_direction\"=5]",
        "!cmdblock 2 0 0 1 0 0 10 0 1 setblock ~ ~-1 ~ red_wool",
        "!cmdblock 3 0 0 2 0 0 0 0 1 setblock ~ ~-1 ~ orange_wool",
        "",
        "",
        "!cmdblock 2 0 0 1 0 0 10 0 1 setblock ~ ~-1 ~ stone",
        "",
        "!cmdblock 2 0 0 1 0 0 10 1 1 setblock ~ ~-1 ~ blue_wool",
        "",
        "",
        "!cmdblock 2 0 0 1 1 0 0 0 1 setblock ~ ~-1 ~ stone",
        "",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "",
        // A loop: four chains in a ring started by an impulse.
        "fill ~2 ~ ~ ~5 ~ ~ air",
        "fill ~2 ~ ~-2 ~5 ~ ~-1 air",
        "setblock ~2 ~ ~3 command_block [\"facing_direction\"=5]",
        "setblock ~3 ~ ~3 chain_command_block [\"facing_direction\"=3]",
        "setblock ~3 ~ ~4 chain_command_block [\"facing_direction\"=4]",
        "setblock ~2 ~ ~4 chain_command_block [\"facing_direction\"=2]",
        "!cmdblock 2 0 3 0 1 0 0 0 1 say ring-start",
        "!cmdblock 3 0 3 2 0 0 0 0 1 say ring-a",
        "!cmdblock 3 0 4 2 0 0 0 0 1 say ring-b",
        "!cmdblock 2 0 4 2 0 0 0 0 1 say ring-c",
        "",
        "setblock ~2 ~1 ~3 redstone_block",
        "",
        "setblock ~2 ~1 ~3 air",
        "",
    ]));
    out
}

/// The game rules: `commandblockoutput` off then on, `commandblocksenabled` off then on,
/// `maxcommandchainlength` 2 against a chain of four.
fn rules() -> Vec<String> {
    let mut out = floor();
    out.extend(default_rules());
    out.extend(lines([
        "setblock ~2 ~ ~ command_block [\"facing_direction\"=5]",
        "setblock ~3 ~ ~ chain_command_block [\"facing_direction\"=5]",
        "setblock ~4 ~ ~ chain_command_block [\"facing_direction\"=5]",
        "setblock ~5 ~ ~ chain_command_block [\"facing_direction\"=5]",
        "!cmdblock 2 0 0 0 1 0 0 0 1 say one",
        "!cmdblock 3 0 0 2 0 0 0 0 1 setblock ~ ~-1 ~ orange_wool",
        "!cmdblock 4 0 0 2 0 0 0 0 1 setblock ~ ~-1 ~ yellow_wool",
        "!cmdblock 5 0 0 2 0 0 0 0 1 setblock ~ ~-1 ~ lime_wool",
        "gamerule commandblockoutput false",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "fill ~2 ~-1 ~ ~5 ~-1 ~ stone",
        "gamerule commandblockoutput true",
        "gamerule commandblocksenabled false",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "gamerule commandblocksenabled true",
        "gamerule maxcommandchainlength 2",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "fill ~2 ~-1 ~ ~5 ~-1 ~ stone",
        "gamerule maxcommandchainlength 65535",
        "",
    ]));
    out
}

pub fn feet(actor: &mut Bot) -> Option<[i32; 3]> {
    actor.movement().and_then(|movement| movement.position()).map(|at| at.map(|c| c.floor() as i32))
}

pub fn coords([x, y, z]: [i32; 3]) -> BlockCoordinates {
    BlockCoordinates { x, y, z }
}

/// `!cmdblock dx dy dz mode needs_redstone conditional delay first_tick track command...` (mode 0
/// impulse, 1 repeating, 2 chain) and `!cmdname dx dy dz name command...` (impulse, powered,
/// tracked, named). False for any other line.
pub async fn act(actor: &mut Bot, line: &str) -> bool {
    let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
    if !["!cmdblock", "!cmdname"].contains(&word) {
        return false;
    }
    let words: Vec<&str> = rest.split(' ').collect();
    let number = |index: usize| words.get(index).and_then(|word| word.parse::<i32>().ok()).unwrap_or(0);
    let Some([fx, fy, fz]) = feet(actor) else { return true };
    let position = coords([fx + number(0), fy + number(1), fz + number(2)]);
    let named = word == "!cmdname";
    let (mode, skip) = if named { (0, 4) } else { (number(3), 9) };
    let mode = match mode {
        1 => CommandBlockUpdateContentTrueMode::Repeat,
        2 => CommandBlockUpdateContentTrueMode::Chain,
        _ => CommandBlockUpdateContentTrueMode::Impulse,
    };
    let content = CommandBlockUpdateContent::True(CommandBlockUpdateContentTrue { position, mode, needs_redstone: named || number(4) == 1, conditional: !named && number(5) == 1 });
    let name = if named { words.get(3).copied().unwrap_or_default().to_owned() } else { String::new() };
    let sent = actor.client().send(&CommandBlockUpdate {
        is_block: true,
        content,
        command: words.get(skip..).map(|rest| rest.join(" ")).unwrap_or_default(),
        last_output: String::new(),
        filtered_name: name.clone(),
        name,
        should_track_output: named || number(8) == 1,
        tick_delay: if named { 0 } else { number(6) },
        execute_on_first_tick: !named && number(7) == 1,
    });
    println!("## sent {word}: {sent:?}");
    true
}
