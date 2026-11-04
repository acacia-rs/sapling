//! The item and block names BDS's commands take: the `Item` and `Block` enums of its AvailableCommands,
//! as `item <name>` and `block <name>` lines, tab-separated: crates/sapling-data/data/command-names.tsv
//! (its README, "Provenance"). A name outside them is BDS's `commands.generic.syntax`. Then each block's
//! `tile.` key, as `tile <name> <key>` lines, that BDS's answers name it by. The `featureRules` enum,
//! `/place featurerule`'s names, comes as `rule <name>` lines.
//! `cargo run -p sapling --example bds_enum_probe -- <server> > command-names.tsv`
//! With more words, prints those instead: `/name` a command's overloads (each parameter as name, value
//! type or enum, `?` if optional), `=Name` an enum's values (static or dynamic), `enums` every enum's name, `commands` every command's name and level.

use std::time::Duration;

use acacia_bot::client::{Client, PacketFilter};
use acacia_bot::proto::{Packet, packets};
use acacia_bot::{Bot, BotConfig, BotEvent};

const WAIT: Duration = Duration::from_secs(20);

/// The next AvailableCommands BDS sends; a change of game mode has it sent again.
async fn commands(bot: &mut Bot) -> Option<packets::AvailableCommands> {
    let mut asked = false;
    tokio::time::timeout(WAIT, async {
        loop {
            match tokio::time::timeout(Duration::from_secs(3), bot.next()).await {
                Ok(Some(BotEvent::Packet(packet))) if packet.id == packets::AvailableCommands::ID => return packet.decode::<packets::AvailableCommands>().ok(),
                Ok(Some(BotEvent::Disconnected(_)) | None) => return None,
                Ok(_) => {}
                Err(_) if !asked => {
                    asked = true;
                    drop(bot.client().command("gamemode creative @s"));
                }
                Err(_) => drop(bot.client().command("op @s")),
            }
        }
    })
    .await
    .ok()?
}

fn values(listed: &packets::AvailableCommands, at: usize) -> Vec<&str> {
    listed.enums[at].values.iter().filter_map(|&value| listed.enum_values.get(value as usize).map(String::as_str)).collect()
}

/// One asked word's lines (main's doc).
fn shown(listed: &packets::AvailableCommands, word: &str) {
    use packets::AvailableCommandsCommandDataItemOverloadsItemParametersItemEnumType as Kind;
    if word == "commands" {
        listed.command_data.iter().for_each(|command| println!("command\t{}\t{}", command.name, command.permission_level));
    } else if word == "enums" {
        listed.enums.iter().for_each(|found| println!("enum\t{}\t{}", found.name, found.values.len()));
        listed.dynamic_enums.iter().for_each(|found| println!("soft\t{}\t{}", found.name, found.values.len()));
    } else if let Some(name) = word.strip_prefix('=') {
        let found = listed.enums.iter().position(|found| found.name == name).map(|at| values(listed, at));
        let soft = listed.dynamic_enums.iter().find(|found| found.name == name).map(|found| found.values.iter().map(String::as_str).collect());
        match found.or(soft) {
            Some(names) => names.iter().for_each(|value| println!("{name}\t{value}")),
            None => println!("{name}\t(no such enum)"),
        }
    } else if let Some(name) = word.strip_prefix('/') {
        let Some(command) = listed.command_data.iter().find(|command| command.name == name) else { return println!("/{name}\t(no such command)") };
        for (n, overload) in command.overloads.iter().enumerate() {
            let parameters: Vec<String> = overload.parameters.iter().map(|p| {
                let kind = match p.enum_type {
                    Kind::Enum => listed.enums.get(p.value_type as usize).map_or(format!("enum#{}", p.value_type), |found| format!("enum:{}({})", found.name, found.values.len())),
                    Kind::SoftEnum => listed.dynamic_enums.get(p.value_type as usize).map_or(format!("soft#{}", p.value_type), |found| format!("soft:{}", found.name)),
                    other => format!("{other:?}:{}", p.value_type),
                };
                format!("{}{}={kind}", p.parameter_name, if p.optional { "?" } else { "" })
            }).collect();
            println!("/{name}\t{n}\t{}", parameters.join(" "));
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args().nth(1).ok_or("usage: bds_enum_probe <server>")?;
    let config = BotConfig { subscribe: PacketFilter::all(), ..BotConfig::default() };
    let mut bot = Bot::connect(Client::builder(server.as_str()).offline("EnumProbe"), config).await?;
    let listed = commands(&mut bot).await.ok_or("no AvailableCommands")?;
    let asked: Vec<String> = std::env::args().skip(2).collect();
    if !asked.is_empty() {
        asked.iter().for_each(|word| shown(&listed, word));
        return Ok(());
    }
    let mut blocks = Vec::new();
    for (kind, name) in [("item", "Item"), ("block", "Block"), ("rule", "featureRules")] {
        let Some(found) = listed.enums.iter().find(|listed| listed.name == name) else { return Err(format!("no {name} enum").into()) };
        let mut names: Vec<&str> = found.values.iter().filter_map(|&at| listed.enum_values.get(at as usize).map(String::as_str)).collect();
        names.sort_unstable();
        for value in names {
            println!("{kind}\t{value}");
            if kind == "block" && value.starts_with("minecraft:") {
                blocks.push(value.to_owned());
            }
        }
    }
    for block in blocks {
        if let Some(key) = tile_key(&mut bot, &block).await {
            println!("tile\t{block}\t{key}");
        }
    }
    Ok(())
}

/// The `tile.` key BDS names `block` by: `/testforblock` of it at an air cell names both blocks
/// (`commands.testforblock.failed.tile`, the wanted one last).
async fn tile_key(bot: &mut Bot, block: &str) -> Option<String> {
    bot.client().command(&format!("testforblock ~ ~8 ~ {block}"));
    let output = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            match bot.next().await? {
                BotEvent::Packet(packet) if packet.id == packets::CommandOutput::ID => return packet.decode::<packets::CommandOutput>().ok(),
                BotEvent::Disconnected(_) => return None,
                _ => {}
            }
        }
    });
    let line = output.await.ok()??.output.into_iter().next()?;
    (line.message_id == "commands.testforblock.failed.tile").then(|| line.parameters.last().cloned()).flatten()
}
