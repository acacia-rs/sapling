//! Records how a vanilla server announces the kinds named, grown: for those `bds_mob_recorder`
//! left out of `sapling-data/data/Mobs.bin` because BDS announces them under another name than
//! it summons them by (a zombie villager is `zombie_villager_v2`). Prints one line a kind,
//! `KIND <announced name> <the AddEntity body in hex>`, to be appended to that file as a
//! little-endian length and the body.
//! `tools/remote-drill.sh bds bds_kind_recorder <out> villager zombie_villager`
#[path = "shared/dump.rs"]
#[allow(dead_code)]
mod dump;
#[path = "shared/probe.rs"]
#[allow(dead_code)]
mod probe;

use std::time::Duration;

use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::AddEntity;
use acacia_bot::proto::types::{MetadataDictionaryItemKey as Key, MetadataDictionaryItemValue as Metadata, MetadataFlags1};
use acacia_bot::{Bot, BotEvent};

const SHOWS_WITHIN: Duration = Duration::from_millis(1500);
const TRIES: usize = 8;
const SETUP: &[&str] = &["gamemode creative @s", "time set midnight", "gamerule domobspawning false", "kill @e[type=!player]"];

fn is_baby(add: &AddEntity) -> bool {
    add.metadata.iter().any(|item| item.key == Key::Flags && matches!(&item.value, Metadata::Flags(flags) if flags.contains(MetadataFlags1::BABY)))
}

/// Summons `kind` beside the bot: the announcement of the kind that came, and its body.
async fn summon(bot: &mut Bot, kind: &str) -> Option<(AddEntity, Vec<u8>)> {
    drop(bot.client().command(&format!("summon {kind} ~3 ~ ~")));
    let end = tokio::time::Instant::now() + SHOWS_WITHIN;
    let mut shown = None;
    while shown.is_none() {
        let Ok(Some(event)) = tokio::time::timeout_at(end, bot.next()).await else { break };
        let BotEvent::Packet(packet) = event else { continue };
        if packet.id == AddEntity::ID {
            shown = packet.decode::<AddEntity>().ok().filter(|add| add.entity_type.starts_with(&format!("minecraft:{kind}"))).map(|add| (add, packet.body.to_vec()));
        }
    }
    drop(bot.client().command("kill @e[type=!player]"));
    let _ = tokio::time::timeout(Duration::from_millis(500), async { while bot.next().await.is_some() {} }).await;
    shown
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let server = args.next().ok_or("usage: bds_kind_recorder <server> <kind>...")?;
    let mut bot = probe::connect(&server, "KindActor").await?;
    for line in SETUP {
        drop(bot.client().command(line));
    }
    let _ = tokio::time::timeout(Duration::from_secs(2), async { while bot.next().await.is_some() {} }).await;
    for kind in args {
        let mut grown = None;
        for _ in 0..TRIES {
            grown = summon(&mut bot, &kind).await.filter(|(add, _)| !is_baby(add));
            if grown.is_some() {
                break;
            }
        }
        match grown {
            Some((add, body)) => println!("KIND {} {}", add.entity_type, body.iter().map(|byte| format!("{byte:02x}")).collect::<String>()),
            None => println!("MISSING {kind}"),
        }
    }
    bot.disconnect().await;
    Ok(())
}
