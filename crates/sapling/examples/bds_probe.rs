//! Shows what a vanilla server tells one player about another: an observer bot prints every packet
//! it receives while an actor bot joins, walks, sneaks, sprints, swings, chats and leaves. The
//! ground truth for sapling's player packets (docs/DESIGN.md "Step 3 as built").
//! `cargo run -p sapling --example bds_probe -- <server> > probe.txt` (an offline-mode server)
use std::time::{Duration, Instant};

use acacia_bot::client::{Client, PacketFilter};
use acacia_bot::proto::{Packet, RawPacket, packets};
use acacia_bot::{ActionError, Bot, BotConfig, BotEvent};

/// Terrain and clock traffic that says nothing about the other player.
const NOISE: &[&str] = &[
    "level_chunk",
    "subchunk",
    "network_chunk_publisher_update",
    "client_cache_miss_response",
    "sync_world_clocks",
    "current_structure_feature",
    "set_time",
    "update_block",
];
/// Characters of a decoded packet shown from each end.
const SHOWN: usize = 1500;

async fn connect(server: &str, name: &str, subscribe: PacketFilter) -> Result<Bot, Box<dyn std::error::Error>> {
    let config = BotConfig { physics: true, subscribe, ..BotConfig::default() };
    Ok(Bot::connect(Client::builder(server).offline(name), config).await?)
}

fn mark(bot: &Bot, label: &str) {
    println!("## actor {label}: feet {:?}", bot.movement().and_then(|m| m.position()));
}

async fn act(mut bot: Bot) -> Result<(), ActionError> {
    bot.wait_ticks(60).await?;
    mark(&bot, "stands");
    bot.controls().expect("physics").forward = 1.0;
    bot.wait_ticks(40).await?;
    bot.controls().expect("physics").stop();
    bot.wait_ticks(20).await?;
    mark(&bot, "walked");
    bot.controls().expect("physics").sneak = true;
    bot.wait_ticks(20).await?;
    bot.controls().expect("physics").sneak = false;
    bot.wait_ticks(20).await?;
    mark(&bot, "sneaked");
    let controls = bot.controls().expect("physics");
    (controls.forward, controls.sprint, controls.yaw) = (1.0, true, 90.0);
    bot.wait_ticks(20).await?;
    bot.controls().expect("physics").stop();
    bot.wait_ticks(20).await?;
    mark(&bot, "sprinted");
    bot.swing();
    bot.wait_ticks(20).await?;
    mark(&bot, "swung");
    bot.client().chat("hello from the probe");
    bot.wait_ticks(40).await?;
    mark(&bot, "chatted");
    bot.disconnect().await;
    println!("## actor left");
    Ok(())
}

fn show(start: Instant, packet: &RawPacket) {
    let name = acacia_bot::proto::packet_name(packet.id).unwrap_or("?");
    if NOISE.iter().any(|n| name.eq_ignore_ascii_case(&n.replace('_', ""))) {
        return;
    }
    let text = dump(packet).unwrap_or_else(|| "<no decoder>".into());
    let chars: Vec<char> = text.chars().collect();
    let text = if chars.len() > 2 * SHOWN {
        let (head, tail): (String, String) = (chars[..SHOWN].iter().collect(), chars[chars.len() - SHOWN..].iter().collect());
        format!("{head} ...({} chars)... {tail}", chars.len() - 2 * SHOWN)
    } else {
        text
    };
    let edge = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let body = &packet.body;
    let (first, last) = (edge(&body[..body.len().min(24)]), edge(&body[body.len().saturating_sub(24)..]));
    println!("{:>7} {name} ({} bytes, {first}..{last}) {text}", start.elapsed().as_millis(), body.len());
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args().nth(1).ok_or("usage: bds_probe <server>")?;
    let mut observer = connect(&server, "ProbeWatcher", PacketFilter::all()).await?;
    observer.wait_ticks(100).await?;
    // What the observer's own join queued is not about the actor.
    while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(5), observer.next()).await {}
    println!("## observer in: feet {:?}", observer.movement().and_then(|m| m.position()));

    let start = Instant::now();
    let actor = connect(&server, "ProbeActor", PacketFilter::none()).await?;
    let acting = act(actor);
    tokio::pin!(acting);
    let mut end: Option<tokio::time::Instant> = None;
    loop {
        tokio::select! {
            event = observer.next() => match event {
                Some(BotEvent::Packet(packet)) => show(start, &packet),
                Some(BotEvent::Disconnected(reason)) => return Err(format!("observer disconnected: {reason:?}").into()),
                Some(_) => {}
                None => break,
            },
            result = &mut acting, if end.is_none() => {
                result?;
                end = Some(tokio::time::Instant::now() + Duration::from_secs(3));
            }
            _ = tokio::time::sleep_until(end.unwrap_or_else(tokio::time::Instant::now)), if end.is_some() => break,
        }
    }
    observer.disconnect().await;
    Ok(())
}

macro_rules! dumper {
    ($($t:ident),* $(,)?) => {
        fn dump(p: &RawPacket) -> Option<String> {
            $(if p.id == <packets::$t>::ID {
                return Some(match p.decode::<packets::$t>() {
                    Ok(v) => format!("{v:?}"),
                    Err(e) => format!("<decode error: {e}>"),
                });
            })*
            None
        }
    };
}
acacia_bot::proto::for_each_packet!(dumper);
