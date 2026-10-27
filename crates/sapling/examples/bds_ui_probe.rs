//! Shows what a vanilla server sends an operator for commands and HUD: its `AvailableCommands`
//! (tables summarised, a few commands in full) and every packet that answers `/title`,
//! `/scoreboard`, `/gamemode`, `/tp`, a boss and commands that fail. The ground truth for sapling's
//! step 4 packets (docs/DESIGN.md "Step 4 as built").
//! `cargo run -p sapling --example bds_ui_probe -- <server> > probe.txt` (offline mode, operators by default)
//! `... -- <server> syntax <command>...` prints only those commands' overloads, enums spelled out.
#[path = "shared/dump.rs"]
mod dump;

use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::Duration;

use acacia_bot::client::{Client, PacketFilter};
use acacia_bot::proto::packets::{AvailableCommands, BossEvent};
use acacia_bot::proto::{Packet, RawPacket};
use acacia_bot::{Bot, BotConfig, BotEvent};

/// Commands printed with every overload.
const SHOWN: &[&str] = &["gamemode", "tp", "title", "me", "say", "tell", "list", "kick", "help", "give", "op", "scoreboard", "xp"];
/// Run in order, each followed by that many seconds of listening. The wither is summoned high up
/// and killed while it still charges; its bar goes when its death animation ends. Out of its
/// range BDS sends no HideBar: the ShowBar it repeats every second just stops.
const COMMANDS: &[(&str, u64)] = &[
    ("list", 1),
    ("say hello", 1),
    ("nonexistent foo", 1),
    ("gamemode banana", 1),
    ("gamemode", 1),
    ("gamemode creative", 1),
    ("gamemode survival", 1),
    ("tp @s ~ ~3 ~", 1),
    ("title @s times 10 40 10", 1),
    ("title @s subtitle Probe subtitle", 1),
    ("title @s title Probe title", 1),
    ("title @s actionbar Probe actionbar", 1),
    ("title @s clear", 1),
    ("title @s reset", 1),
    ("scoreboard objectives add probe dummy \"Probe board\"", 1),
    ("scoreboard objectives setdisplay sidebar probe", 1),
    ("scoreboard players set \"line one\" probe 1", 1),
    ("scoreboard players set \"line two\" probe 2", 1),
    ("scoreboard players set \"line one\" probe 5", 1),
    ("scoreboard players reset \"line two\" probe", 1),
    ("scoreboard objectives setdisplay sidebar", 1),
    ("scoreboard objectives remove probe", 1),
    ("difficulty easy", 1),
    ("summon wither ~ ~40 ~", 3),
    ("tp @s ~300 ~ ~", 6),
    ("tp @s ~-300 ~ ~", 6),
    ("kill @e[type=wither]", 20),
    ("difficulty peaceful", 1),
];

/// The commands `syntax` asked for; empty for the whole drill.
static SYNTAX: OnceLock<Vec<String>> = OnceLock::new();

/// Each overload of the commands in [`SYNTAX`], one line, an enum parameter as its name and words.
fn syntax(p: &AvailableCommands, names: &[String]) {
    use acacia_bot::proto::packets::AvailableCommandsCommandDataItemOverloadsItemParametersItemEnumType as EnumType;
    let words = |e: &acacia_bot::proto::packets::AvailableCommandsEnumsItem| e.values.iter().map(|v| p.enum_values[*v as usize].clone()).collect::<Vec<_>>().join("|");
    for command in p.command_data.iter().filter(|c| names.contains(&c.name)) {
        let alias = usize::try_from(command.alias).ok().map(|i| words(&p.enums[i]));
        println!("/{} {:?} aliases {:?} chained {:?}", command.name, command.description, alias, command.chained_subcommand_offsets);
        for overload in &command.overloads {
            let shown: Vec<String> = overload
                .parameters
                .iter()
                .map(|parameter| {
                    let index = parameter.value_type as usize;
                    let kind = match parameter.enum_type {
                        EnumType::Enum => format!("{}{{{}}}", p.enums[index].name, words(&p.enums[index])),
                        EnumType::SoftEnum => p.dynamic_enums.get(index).map_or_else(|| format!("soft{index}"), |e| format!("~{}{{{}}}", e.name, e.values.len())),
                        EnumType::ChainedSubcommand => format!("chained{:?}", p.chained_subcommands.get(index).map(|c| &c.name)),
                        other => format!("{}/{}", parameter.value_type, other.to_raw()),
                    };
                    let optional = if parameter.optional { "?" } else { "" };
                    format!("{}{optional}:{kind}", parameter.parameter_name)
                })
                .collect();
            println!("    {}{}", if overload.chaining { "chaining " } else { "" }, shown.join("  "));
        }
    }
    for chained in &p.chained_subcommands {
        let values: Vec<String> = chained.values.iter().map(|v| format!("{}={}", p.chained_subcommand_values.get(v.index as usize).map_or("?", String::as_str), v.value)).collect();
        println!("chained {} {}", chained.name, values.join(" "));
    }
}

fn commands(p: &AvailableCommands) {
    if let Some(names) = SYNTAX.get().filter(|names| !names.is_empty()) {
        return syntax(p, names);
    }
    println!(
        "AvailableCommands: {} enum values, {} chained values, {} suffixes {:?}, {} enums, {} chained, {} commands, {} dynamic enums, {} constraints",
        p.enum_values.len(),
        p.chained_subcommand_values.len(),
        p.suffixes.len(),
        p.suffixes,
        p.enums.len(),
        p.chained_subcommands.len(),
        p.command_data.len(),
        p.dynamic_enums.len(),
        p.enum_constraints.len()
    );
    let mut kinds: BTreeMap<String, String> = BTreeMap::new();
    for command in &p.command_data {
        let alias = usize::try_from(command.alias).ok().map(|i| &p.enums[i]);
        println!(
            "  /{} {:?} flags {:#06x} permission {:?} alias {} {:?} chained {:?} overloads {}",
            command.name,
            command.description,
            command.flags,
            command.permission_level,
            command.alias,
            alias.map(|e| (&e.name, e.values.iter().map(|v| &p.enum_values[*v as usize]).collect::<Vec<_>>())),
            command.chained_subcommand_offsets,
            command.overloads.len()
        );
        for overload in &command.overloads {
            for parameter in &overload.parameters {
                let kind = format!("{:?}/{:?} raw {}:{}", parameter.value_type, parameter.enum_type, parameter.value_type, parameter.enum_type.to_raw());
                kinds.entry(kind).or_insert_with(|| format!("/{} {}", command.name, parameter.parameter_name));
            }
            if SHOWN.contains(&command.name.as_str()) {
                println!("    overload chaining {}", overload.chaining);
                for parameter in &overload.parameters {
                    println!("      {parameter:?}");
                }
            }
        }
    }
    println!("  parameter kinds (first use):");
    kinds.iter().for_each(|(kind, first)| println!("    {kind}: {first}"));
    for (i, e) in p.enums.iter().enumerate().filter(|(_, e)| e.values.len() <= 8).take(40) {
        println!("  enum {i} {:?} = {:?}", e.name, e.values.iter().map(|v| &p.enum_values[*v as usize]).collect::<Vec<_>>());
    }
    println!("  dynamic enums: {:?}", p.dynamic_enums);
    println!("  constraints: {:?}", p.enum_constraints.iter().take(5).collect::<Vec<_>>());
    println!("  chained: {:?}", p.chained_subcommands.iter().take(3).collect::<Vec<_>>());
}

fn show(packet: &RawPacket) {
    let Some(name) = dump::name(packet) else { return };
    if packet.id == AvailableCommands::ID {
        return match packet.decode::<AvailableCommands>() {
            Ok(decoded) => commands(&decoded),
            Err(error) => println!("AvailableCommands: <decode error: {error}>"),
        };
    }
    if SYNTAX.get().is_some_and(|names| !names.is_empty()) {
        return;
    }
    let text: String = dump::dump(packet).unwrap_or_else(|| "<no decoder>".into()).chars().take(1500).collect();
    println!("  {name} ({} bytes) {text}", packet.body.len());
    if packet.id == BossEvent::ID {
        // Its layout differs by event type: the bytes tell whether the decoder read all of them.
        println!("    bytes {}", packet.body.iter().map(|b| format!("{b:02x}")).collect::<String>());
    }
}

async fn drain(bot: &mut Bot, wait: Duration) -> Result<(), Box<dyn std::error::Error>> {
    let end = tokio::time::Instant::now() + wait;
    while let Ok(event) = tokio::time::timeout_at(end, bot.next()).await {
        match event {
            Some(BotEvent::Packet(packet)) => show(&packet),
            Some(BotEvent::Disconnected(reason)) => return Err(format!("disconnected: {reason:?}").into()),
            Some(_) => {}
            None => break,
        }
    }
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args().nth(1).ok_or("usage: bds_ui_probe <server> [syntax <command>...]")?;
    let names: Vec<String> = std::env::args().skip(2).skip_while(|word| word == "syntax").collect();
    let only_syntax = !names.is_empty();
    SYNTAX.get_or_init(|| names);
    let config = BotConfig { physics: true, subscribe: PacketFilter::all(), ..BotConfig::default() };
    let mut bot = Bot::connect(Client::builder(server.as_str()).offline("UiProbe"), config).await?;
    println!("## join");
    drain(&mut bot, Duration::from_secs(5)).await?;
    if only_syntax {
        bot.disconnect().await;
        return Ok(());
    }
    for (command, seconds) in COMMANDS {
        println!("## /{command}");
        bot.client().command(command);
        drain(&mut bot, Duration::from_secs(*seconds)).await?;
    }
    bot.disconnect().await;
    Ok(())
}
