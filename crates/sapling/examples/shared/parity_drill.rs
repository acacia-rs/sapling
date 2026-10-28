//! The command-parity drills of bds_command_probe: `saysel` (selectors inside `/say`, `/me` and
//! `/tell` messages, from the actor, `/execute` and a command block), `cmdlist` and `member` (how an
//! operator's and a member's commands are answered), `rulesync` (which game rules a change is told
//! to clients) and `idle` (quiet, for tools/bds-drill.sh's CONSOLE lines).

use acacia_bot::proto::packets::AvailableCommands;
use acacia_bot::proto::{Packet, RawPacket};

use super::cmdblocks::{floor, lines};

pub const SAY_SHOWN: &[&str] = &["AvailableCommands"];

/// The drill called `name` of this file.
pub fn drill(name: &str) -> Option<(Vec<String>, &'static [&'static str])> {
    let lines = match name {
        "saysel" => say_selectors(),
        "cmdlist" => command_list(),
        "rulesync" => rule_sync(),
        // For a server whose players join as members: selectors in what a member may and may not run.
        "member" => lines([
            "tell @a hi",
            "tell CmdWatcher hi @s",
            "me hi @a",
            "list",
            "tp @s ~ ~ ~",
            "tp CmdActor ~ ~ ~",
            "tp CmdActor @s",
            "give CmdActor dirt",
            "give @s dirt",
            "gamemode creative",
            "difficulty banana",
            "kill @e[type=cow",
            "nosuch @a",
            "say hi",
            "say hi @a",
            "tell @s[name=CmdActor] x",
            "w @p hi",
        ]),
        // Forty quiet seconds, for console lines sent meanwhile (tools/bds-drill.sh CONSOLE).
        "idle" => vec![String::new(); 40],
        _ => return None,
    };
    Some((lines, SAY_SHOWN))
}

/// AvailableCommands as one line per command: name, permission, flags, aliases, overloads.
pub fn listed(packet: &RawPacket) -> Option<Vec<String>> {
    let list = (packet.id == AvailableCommands::ID).then(|| packet.decode::<AvailableCommands>().ok()).flatten()?;
    let words = |at: i32| usize::try_from(at).ok().map(|at| list.enums[at].values.iter().map(|v| list.enum_values[*v as usize].clone()).collect::<Vec<_>>().join("|"));
    let mut rows: Vec<String> = list.command_data.iter().map(|c| format!("/{} perm={} flags={:#x} aliases={:?} overloads={} {:?}", c.name, c.permission_level, c.flags, words(c.alias), c.overloads.len(), c.description)).collect();
    rows.sort();
    rows.insert(0, format!("AvailableCommands {} commands", rows.len()));
    Some(rows)
}

fn say_selectors() -> Vec<String> {
    let mut out = floor();
    out.extend(lines([
        "gamerule commandblocksenabled true",
        "gamerule commandblockoutput true",
        "summon cow ~4 ~ ~3",
        "say hi @s",
        "say hi @a",
        "say @s@s x",
        "say a@s b",
        "say hi @e[type=cow]",
        "say hi @e[type=cow,c=0] end",
        "say hi @p[name=nobody] x",
        "say hi @x y",
        "say hi @s[ y",
        "say hi @s[bad=1] y",
        "say mail@example.com",
        "say hi @ y",
        "say hi @here",
        "say hi \"@s\" y",
        "say hi @s, @a.",
        "say hi @initiator",
        "say hi @r",
        "say  two  spaces  @s  end ",
        "me waves at @a",
        "me @s",
        "tell CmdWatcher hi @s",
        "tell @a hey @p and @e[type=cow]",
        "execute as @e[type=cow] run say I am @s",
        "execute as @e[type=cow] run me moos at @s",
        "execute as @e[type=cow] run tell CmdWatcher from @s",
        "setblock ~2 ~ ~ command_block [\"facing_direction\"=5]",
        "",
        "!cmdblock 2 0 0 0 1 0 0 0 1 say hi @s",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "!cmdblock 2 0 0 0 1 0 0 0 1 say hi @a and @p",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "!cmdblock 2 0 0 0 1 0 0 0 1 me hi @s",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "!cmdblock 2 0 0 0 1 0 0 0 1 tell @a hi @s",
        "setblock ~2 ~1 ~ redstone_block",
        "",
        "setblock ~2 ~1 ~ air",
        "kill @e[type=cow]",
    ]));
    out
}

/// What an operator is answered (and, where `/deop` works, a member): the watcher deopped and
/// opped again, then the actor's lines before and after its own `/deop`. BDS refused an operator
/// both (bds-cmdlist.txt); its lists come from `bds_ui_probe`, a member's from the `member` drill.
fn command_list() -> Vec<String> {
    let asked = ["seed", "version", "help", "help say", "help 2", "?", "list", "me hi", "tell CmdWatcher hi", "say hi", "gamerule", "nosuch", "op CmdActor", "gamemode creative @s", "tp @s ~ ~ ~", "kick nobody"];
    let mut out = lines(["deop CmdWatcher", "", "op CmdWatcher", ""]);
    out.extend(asked.map(String::from));
    out.extend(lines(["deop CmdActor", ""]));
    out.extend(asked.map(String::from));
    out
}

/// Every rule BDS lists, each set to its default, changed, set to the same again and put back.
fn rule_sync() -> Vec<String> {
    const RULES: &[(&str, &str, &str)] = &[
        ("commandblockoutput", "true", "false"),
        ("dodaylightcycle", "true", "false"),
        ("doentitydrops", "true", "false"),
        ("dofiretick", "true", "false"),
        ("recipesunlock", "true", "false"),
        ("dolimitedcrafting", "false", "true"),
        ("domobloot", "true", "false"),
        ("domobspawning", "true", "false"),
        ("dotiledrops", "true", "false"),
        ("doweathercycle", "true", "false"),
        ("drowningdamage", "true", "false"),
        ("falldamage", "true", "false"),
        ("firedamage", "true", "false"),
        ("keepinventory", "false", "true"),
        ("mobgriefing", "true", "false"),
        ("pvp", "true", "false"),
        ("showcoordinates", "false", "true"),
        ("playerwaypoints", "everyone", "off"),
        ("showdaysplayed", "false", "true"),
        ("naturalregeneration", "true", "false"),
        ("tntexplodes", "true", "false"),
        ("maxcommandchainlength", "65535", "5"),
        ("doinsomnia", "true", "false"),
        ("commandblocksenabled", "true", "false"),
        ("randomtickspeed", "1", "0"),
        ("doimmediaterespawn", "false", "true"),
        ("showdeathmessages", "true", "false"),
        ("functioncommandlimit", "10000", "5"),
        ("spawnradius", "10", "5"),
        ("showtags", "true", "false"),
        ("freezedamage", "true", "false"),
        ("respawnblocksexplode", "true", "false"),
        ("showborderEffect", "true", "false"),
        ("showrecipemessages", "true", "false"),
        ("playerssleepingpercentage", "100", "50"),
        ("projectilescanbreakblocks", "true", "false"),
        ("tntexplosiondropdecay", "false", "true"),
        ("sendcommandfeedback", "true", "false"),
    ];
    let mut out = vec!["gamerule".to_owned()];
    for (rule, default, other) in RULES {
        out.extend([default, other, other, default].map(|value| format!("gamerule {rule} {value}")));
    }
    out.push("gamerule".to_owned());
    out
}
