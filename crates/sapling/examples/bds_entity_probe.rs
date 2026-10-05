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
use acacia_bot::proto::types::MetadataDictionaryItemKey as Key;
use acacia_bot::proto::{Packet, packet_name};
use acacia_bot::{Bot, BotConfig, BotEvent};

/// Packets shown in full, the first [`SHOWN`] of each; of the rest only the count.
const WATCHED: &[&str] = &["SetTime", "SyncWorldClocks", "SetSpawnPosition", "SetDifficulty", "GameRulesChanged", "AddItemEntity", "SetEntityData", "UpdateAttributes", "AddPainting", "MobEquipment", "MobArmorEquipment"];
const SHOWN: usize = 3;
/// Characters of a decoded packet shown.
const LENGTH: usize = 6000;
const PATIENCE: Duration = Duration::from_secs(90);
const WATCH: Duration = Duration::from_secs(25);

fn text_of(add: &AddEntity, key: Key) -> String {
    add.metadata.iter().find(|item| item.key == key).map(|item| format!("{:?}", item.value)).unwrap_or_default()
}

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
    let (mut counts, mut kinds, mut named): (BTreeMap<&str, usize>, BTreeMap<String, usize>, BTreeMap<String, usize>) = Default::default();
    let end = tokio::time::Instant::now() + WATCH;
    while let Ok(Some(event)) = tokio::time::timeout_at(end, bot.next()).await {
        let BotEvent::Packet(packet) = event else { continue };
        let name = packet_name(packet.id).unwrap_or("?");
        let seen = counts.entry(name).or_default();
        *seen += 1;
        let text = || dump::dump(&packet).unwrap_or_default().chars().take(LENGTH).collect::<String>();
        if packet.id == AddEntity::ID {
            // One of each kind in full: the metadata differs by kind.
            let Ok(add) = packet.decode::<AddEntity>() else { continue };
            *named.entry(format!("{} {:?} wool {}", add.entity_type, text_of(&add, Key::Nametag), text_of(&add, Key::Color))).or_default() += 1;
            let of_kind = kinds.entry(add.entity_type).or_default();
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
    println!("## entities by kind, name tag and colour: {named:#?}");
    println!("## packets: {counts:?}");
    Ok(())
}
