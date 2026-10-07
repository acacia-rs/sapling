//! Runs command lines as an operator bot while a second operator watches, and prints what each
//! was answered: CommandOutput in full, Text, GameRulesChanged as `rule=value`. The ground truth
//! for sapling's vanilla commands (`sapling::vanilla`).
//! `cargo run -p sapling --example bds_command_probe -- <server> [command...]` (offline mode,
//! operators by default); without commands it runs the /gamerule drill.
#[path = "shared/dump.rs"]
#[allow(dead_code)]
mod dump;
#[path = "shared/probe.rs"]
#[allow(dead_code)]
mod probe;

use std::time::{Duration, Instant};

use acacia_bot::BotEvent;
use acacia_bot::proto::{Packet, packets};
use probe::{SETTLE, connect};

const GAMERULE: &[&str] = &[
    "gamerule",
    "gamerule dodaylightcycle",
    "gamerule dodaylightcycle false",
    "gamerule dodaylightcycle false",
    "gamerule DoDaylightCycle true",
    "gamerule randomtickspeed",
    "gamerule randomtickspeed 5",
    "gamerule randomtickspeed true",
    "gamerule randomtickspeed -3",
    "gamerule dodaylightcycle 3",
    "gamerule nosuchrule true",
    "gamerule spawnradius 2000000000",
    "gamerule playerssleepingpercentage 150",
    "gamerule playerssleepingpercentage -1",
    "gamerule playerssleepingpercentage 100",
    "gamerule randomtickspeed 1",
    "gamerule randomtickspeed 4097",
    "gamerule spawnradius 128",
    "gamerule spawnradius -1",
    "gamerule functioncommandlimit 0",
    "gamerule functioncommandlimit 10001",
    "gamerule maxcommandchainlength -1",
    "gamerule DoDaylightCycle",
    "gamerule playerwaypoints",
    "gamerule playerwaypoints banana",
    "gamerule playerwaypoints off",
    "gamerule playerwaypoints 0",
    "gamerule playerwaypoints everyone",
    "gamerule locatorbar",
    "gamerule locatorbar false",
    "gamerule locatorbar true",
    "gamerule dodaylightcycle true extra",
    "gamerule",
];

fn show(start: Instant, who: &str, event: Option<BotEvent>) -> Result<(), String> {
    let packet = match event {
        Some(BotEvent::Packet(packet)) => packet,
        Some(BotEvent::Disconnected(reason)) => return Err(format!("{who} disconnected: {reason:?}")),
        Some(_) => return Ok(()),
        None => return Err(format!("{who} is gone")),
    };
    let ms = start.elapsed().as_millis();
    if packet.id == packets::CommandOutput::ID {
        let answer = packet.decode::<packets::CommandOutput>().map_err(|e| e.to_string())?;
        let lines: Vec<String> = answer.output.iter().map(|line| format!("{} success={} {:?}", line.message_id, line.success, line.parameters)).collect();
        println!("{ms:>7} {who:<7} CommandOutput success_count={} data={:?} {}", answer.success_count, answer.data, lines.join(" | "));
    } else if packet.id == packets::GameRulesChanged::ID {
        let rules = packet.decode::<packets::GameRulesChanged>().map_err(|e| e.to_string())?;
        let shown: Vec<String> = rules.rules.iter().map(|rule| format!("{}={:?}", rule.name, rule.value)).collect();
        println!("{ms:>7} {who:<7} GameRulesChanged {}", shown.join(" "));
    } else if matches!(acacia_bot::proto::packet_name(packet.id), Some("Text" | "SetTime" | "SyncWorldClocks")) {
        println!("{ms:>7} {who:<7} {}", dump::dump(&packet).unwrap_or_default());
    }
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let server = args.next().ok_or("usage: bds_command_probe <server> [command...]")?;
    let given: Vec<String> = args.collect();
    let lines: Vec<String> = if given.is_empty() { GAMERULE.iter().map(|line| line.to_string()).collect() } else { given };
    let (mut actor, mut watcher) = (connect(&server, "CmdActor").await?, connect(&server, "CmdWatcher").await?);
    watcher.wait_ticks(60).await?;
    actor.wait_ticks(60).await?;
    for bot in [&mut actor, &mut watcher] {
        while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(5), bot.next()).await {}
    }
    let start = Instant::now();
    for line in &lines {
        println!("## /{line}");
        actor.client().command(line);
        let end = tokio::time::Instant::now() + SETTLE;
        loop {
            tokio::select! {
                event = actor.next() => show(start, "actor", event)?,
                event = watcher.next() => show(start, "watcher", event)?,
                _ = tokio::time::sleep_until(end) => break,
            }
        }
    }
    actor.disconnect().await;
    watcher.disconnect().await;
    Ok(())
}
