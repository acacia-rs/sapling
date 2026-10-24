//! The item and block names BDS's commands take: the `Item` and `Block` enums of its AvailableCommands,
//! as `item <name>` and `block <name>` lines, tab-separated: crates/sapling-data/data/command-names.tsv
//! (its README, "Provenance"). A name outside them is BDS's `commands.generic.syntax`.
//! `cargo run -p sapling --example bds_enum_probe -- <server> > command-names.tsv`

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args().nth(1).ok_or("usage: bds_enum_probe <server>")?;
    let config = BotConfig { subscribe: PacketFilter::all(), ..BotConfig::default() };
    let mut bot = Bot::connect(Client::builder(server.as_str()).offline("EnumProbe"), config).await?;
    let listed = commands(&mut bot).await.ok_or("no AvailableCommands")?;
    for (kind, name) in [("item", "Item"), ("block", "Block")] {
        let Some(found) = listed.enums.iter().find(|listed| listed.name == name) else { return Err(format!("no {name} enum").into()) };
        let mut names: Vec<&str> = found.values.iter().filter_map(|&at| listed.enum_values.get(at as usize).map(String::as_str)).collect();
        names.sort_unstable();
        for value in names {
            println!("{kind}\t{value}");
        }
    }
    Ok(())
}
