//! What a server tells a player as it joins, and the one already in: the entity property
//! registries (SyncEntityProperty, printed as their NBT's hex) and the scoreboard (SetScore,
//! SetDisplayObjective, SetScoreboardIdentity). A keeper gives itself and the name `Late` scores
//! before `Late` joins; `Late` sets its own, leaves and joins again.
//! `cargo run -p sapling --example bds_join_probe -- <server>` (offline mode, operators by default).
#[path = "shared/dump.rs"]
#[allow(dead_code)]
mod dump;
#[path = "shared/probe.rs"]
#[allow(dead_code)]
mod probe;

use std::time::{Duration, Instant};

use acacia_bot::proto::{Packet, RawPacket, packets};
use acacia_bot::{Bot, BotEvent};
use probe::{clock, connect};

const SHOWN: &[&str] = &["SyncEntityProperty", "SetScore", "SetDisplayObjective", "RemoveObjective", "SetScoreboardIdentity"];
const SETUP: &[&str] = &[
    "scoreboard objectives remove obj",
    "scoreboard objectives add obj dummy",
    "scoreboard objectives setdisplay sidebar obj",
    "scoreboard players set @s obj 7",
    "scoreboard players set Late obj 3",
];

fn show(start: Instant, who: &str, packet: &RawPacket) {
    let Some(name) = acacia_bot::proto::packet_name(packet.id).filter(|name| SHOWN.contains(name)) else { return };
    let text = if packet.id == packets::SyncEntityProperty::ID {
        packet.body.iter().map(|byte| format!("{byte:02x}")).collect()
    } else {
        dump::dump(packet).unwrap_or_default()
    };
    println!("{:>7} {who:<7} {name} {text}", clock::tick_ms(start));
}

/// Prints both bots' shown packets for `wait`.
async fn watch(start: Instant, keeper: &mut Bot, late: Option<&mut Bot>, wait: Duration) -> Result<(), String> {
    let end = tokio::time::Instant::now() + clock::scaled(wait);
    let mut late = late;
    loop {
        let late_next = async {
            match late.as_deref_mut() {
                Some(bot) => bot.next().await,
                None => std::future::pending().await,
            }
        };
        tokio::select! {
            event = keeper.next() => match event {
                Some(BotEvent::Packet(packet)) => show(start, "keeper", &packet),
                Some(BotEvent::Disconnected(reason)) => return Err(format!("keeper disconnected: {reason:?}")),
                Some(_) => {}
                None => return Err("keeper is gone".into()),
            },
            event = late_next => if let Some(BotEvent::Packet(packet)) = event { show(start, "late", &packet) },
            _ = tokio::time::sleep_until(end) => return Ok(()),
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args().nth(1).ok_or("usage: bds_join_probe <server>")?;
    let start = Instant::now();
    clock::mark_start();
    println!("## keeper joins");
    let mut keeper = connect(&server, "Keeper").await?;
    keeper.wait_ticks(60).await?;
    watch(start, &mut keeper, None, Duration::from_millis(500)).await?;
    for line in SETUP {
        println!("## /{line}");
        let _ = keeper.client().command(line);
        watch(start, &mut keeper, None, Duration::from_millis(800)).await?;
    }
    for round in 1..=2 {
        println!("## late joins ({round})");
        let mut late = connect(&server, "Late").await?;
        watch(start, &mut keeper, Some(&mut late), Duration::from_secs(4)).await?;
        if round == 1 {
            println!("## late: /scoreboard players set @s obj 4");
            let _ = late.client().command("scoreboard players set @s obj 4");
            watch(start, &mut keeper, Some(&mut late), Duration::from_secs(1)).await?;
        }
        println!("## late leaves");
        late.disconnect().await;
        watch(start, &mut keeper, None, Duration::from_secs(2)).await?;
    }
    println!("## /scoreboard objectives remove obj");
    let _ = keeper.client().command("scoreboard objectives remove obj");
    watch(start, &mut keeper, None, Duration::from_secs(1)).await?;
    keeper.disconnect().await;
    Ok(())
}
