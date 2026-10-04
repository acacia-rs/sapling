//! Shows what a vanilla server tells a joining player about the world's entities, clock, spawn
//! and rules: the ground truth for sapling's mobs and time (docs/DESIGN.md).
//! `cargo run -p sapling --example bds_entity_probe -- <server> > probe.txt` (an offline-mode server)
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

// This probe picks its packets itself and leaves the shared noise filter unused.
#[allow(dead_code)]
#[path = "shared/dump.rs"]
mod dump;

use acacia_bot::client::{Client, PacketFilter};
use acacia_bot::proto::packets::AddEntity;
use acacia_bot::proto::{Packet, packet_name};
use acacia_bot::{Bot, BotConfig, BotEvent};

/// Packets shown in full, the first [`SHOWN`] of each; of the rest only the count.
const WATCHED: &[&str] = &["SetTime", "SyncWorldClocks", "SetSpawnPosition", "SetDifficulty", "GameRulesChanged", "AddItemEntity", "SetEntityData", "UpdateAttributes"];
const SHOWN: usize = 3;
/// Characters of a decoded packet shown.
const LENGTH: usize = 6000;
const PATIENCE: Duration = Duration::from_secs(90);
const WATCH: Duration = Duration::from_secs(25);

async fn connect(server: &str) -> Result<Bot, Box<dyn std::error::Error>> {
    let deadline = Instant::now() + PATIENCE;
    let config = || BotConfig { physics: true, subscribe: PacketFilter::all(), ..BotConfig::default() };
    loop {
        match Bot::connect(Client::builder(server).offline("EntityProbe").chunk_radius(8), config()).await {
            Ok(bot) => return Ok(bot),
            Err(error) if Instant::now() > deadline => return Err(error.into()),
            Err(_) => tokio::time::sleep(Duration::from_secs(2)).await,
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args().nth(1).ok_or("usage: bds_entity_probe <server>")?;
    let mut bot = connect(&server).await?;
    let (mut counts, mut kinds): (BTreeMap<&str, usize>, BTreeMap<String, usize>) = Default::default();
    let end = tokio::time::Instant::now() + WATCH;
    while let Ok(Some(event)) = tokio::time::timeout_at(end, bot.next()).await {
        let BotEvent::Packet(packet) = event else { continue };
        let name = packet_name(packet.id).unwrap_or("?");
        let seen = counts.entry(name).or_default();
        *seen += 1;
        let text = || dump::dump(&packet).unwrap_or_default().chars().take(LENGTH).collect::<String>();
        if packet.id == AddEntity::ID {
            // One of each kind in full: the metadata differs by kind.
            let kind = packet.decode::<AddEntity>().map(|add| add.entity_type).unwrap_or_default();
            let of_kind = kinds.entry(kind).or_default();
            *of_kind += 1;
            if *of_kind == 1 {
                println!("{}", text());
            }
        } else if *seen <= SHOWN && WATCHED.iter().any(|watched| watched.eq_ignore_ascii_case(&name.replace('_', ""))) {
            println!("{}", text());
        }
    }
    bot.disconnect().await;
    println!("## entities by kind: {kinds:?}");
    println!("## packets: {counts:?}");
    Ok(())
}
