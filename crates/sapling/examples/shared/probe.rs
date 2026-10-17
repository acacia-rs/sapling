//! An actor bot that does things on a server and a watcher bot that stands by, both printing the
//! packets a probe looks at with the milliseconds since it started.

use std::future::Future;
use std::time::{Duration, Instant};

use acacia_bot::client::{Client, PacketFilter};
use acacia_bot::interact::Face;
use acacia_bot::proto::{Packet, packets};
use acacia_bot::{Bot, BotConfig, BotEvent};
use acacia_world::BlockAccess;

use crate::dump;

/// Characters of a decoded packet shown.
const SHOWN: usize = 600;
/// How long a step's consequences are watched for, unless the step says otherwise.
pub const SETTLE: Duration = Duration::from_millis(1200);

pub struct Probe {
    pub start: Instant,
    pub actor: Bot,
    pub watcher: Bot,
    /// The packet names printed.
    pub shown: &'static [&'static str],
}

fn show(start: Instant, shown: &[&str], who: &str, event: Option<BotEvent>) -> Result<(), String> {
    match event {
        Some(BotEvent::Packet(packet)) => {
            let name = acacia_bot::proto::packet_name(packet.id).unwrap_or("?");
            if shown.contains(&name) {
                let text: String = dump::dump(&packet).unwrap_or_default().chars().take(SHOWN).collect();
                println!("{:>7} {who:<7} {name} {text}", start.elapsed().as_millis());
            }
            Ok(())
        }
        Some(BotEvent::Disconnected(reason)) => Err(format!("{who} disconnected: {reason:?}")),
        Some(_) => Ok(()),
        None => Err(format!("{who} is gone")),
    }
}

pub async fn connect(server: &str, name: &str) -> Result<Bot, Box<dyn std::error::Error>> {
    let trackers = acacia_bot::state::Trackers { entities: true, ..Default::default() };
    let config = BotConfig { physics: true, subscribe: PacketFilter::all(), trackers, ..BotConfig::default() };
    // Wide enough that both bots see every drill, which spread over a hundred blocks.
    Ok(Bot::connect(Client::builder(server).offline(name).chunk_radius(8), config).await?)
}

impl Probe {
    /// Runs one of the actor's actions while the watcher keeps printing.
    pub async fn during<T>(&mut self, action: impl FnOnce(&mut Bot) -> std::pin::Pin<Box<dyn Future<Output = T> + '_>>) -> Result<T, String> {
        let (start, shown) = (self.start, self.shown);
        let action = action(&mut self.actor);
        tokio::pin!(action);
        loop {
            tokio::select! {
                result = &mut action => return Ok(result),
                event = self.watcher.next() => show(start, shown, "watcher", event)?,
            }
        }
    }

    pub async fn settle(&mut self, wait: Duration) -> Result<(), String> {
        let end = tokio::time::Instant::now() + wait;
        loop {
            tokio::select! {
                event = self.actor.next() => show(self.start, self.shown, "actor", event)?,
                event = self.watcher.next() => show(self.start, self.shown, "watcher", event)?,
                _ = tokio::time::sleep_until(end) => return Ok(()),
            }
        }
    }

    /// Drops what both bots were sent so far without printing it: setup noise.
    pub async fn quiet(&mut self) {
        for bot in [&mut self.actor, &mut self.watcher] {
            while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(5), bot.next()).await {}
        }
    }

    /// Runs commands as setup: each is given a moment, and nothing they cause is printed but a
    /// command that failed.
    pub async fn setup(&mut self, lines: &[String]) -> Result<(), String> {
        for line in lines {
            self.actor.client().command(line);
            self.actor.wait_ticks(4).await.map_err(|e| e.to_string())?;
        }
        self.actor.wait_ticks(10).await.map_err(|e| e.to_string())?;
        while let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(5), self.actor.next()).await {
            let BotEvent::Packet(packet) = event else { continue };
            let failed = packet.decode::<packets::CommandOutput>().ok().filter(|answer| packet.id == packets::CommandOutput::ID && answer.success_count == 0);
            if let Some(answer) = failed {
                let said: Vec<String> = answer.output.iter().map(|line| format!("{} {:?}", line.message_id, line.parameters)).collect();
                println!("## setup failed ({}): {}", lines.join(" | ").chars().take(200).collect::<String>(), said.join("; "));
            }
        }
        self.quiet().await;
        Ok(())
    }

    /// A command whose consequences are the point, watched for `wait`.
    pub async fn command(&mut self, line: &str, wait: Duration) -> Result<(), String> {
        println!("## /{line}");
        self.actor.client().command(line);
        self.settle(wait).await
    }

    pub fn name(&self, [x, y, z]: [i32; 3]) -> String {
        let world = self.actor.world().expect("a world tracker");
        let (Some(view), Some(registry)) = (world.view(), world.registry()) else { return "?".into() };
        registry.get(view.block(x, y, z)).map_or("?".into(), |state| format!("{} [{}]", state.name, state.properties))
    }

    pub async fn break_block(&mut self, label: &str, pos: [i32; 3], wait: Duration) -> Result<(), String> {
        println!("## {label}: break {pos:?} ({})", self.name(pos));
        let result = self.during(|bot| Box::pin(bot.break_block(pos))).await?;
        self.settle(wait).await?;
        println!("## -> {result:?}, now {}", self.name(pos));
        Ok(())
    }

    pub async fn place_block(&mut self, label: &str, against: [i32; 3], face: Face, wait: Duration) -> Result<(), String> {
        let target = face.adjacent(against);
        println!("## {label}: place against {against:?} ({}) face {face:?}", self.name(against));
        let result = self.during(|bot| Box::pin(bot.place_block(against, face))).await?;
        self.settle(wait).await?;
        println!("## -> {result:?}, {target:?} is now {}", self.name(target));
        Ok(())
    }

    pub async fn click(&mut self, label: &str, pos: [i32; 3], face: Face, wait: Duration) -> Result<(), String> {
        println!("## {label}: click {pos:?} ({}) face {face:?}", self.name(pos));
        let result = self.during(|bot| Box::pin(bot.use_item_on_block(pos, face))).await?;
        self.settle(wait).await?;
        println!("## -> {result:?}, {pos:?} is now {}, {:?} is now {}", self.name(pos), face.adjacent(pos), self.name(face.adjacent(pos)));
        Ok(())
    }
}
