//! Dimension changes and Nether portals: an operator bot that answers a dimension change as a client
//! does, while a watcher stays behind. Prints the actor's packets with the milliseconds since the
//! drill started (details for those `DETAILED` names), what the watcher hears of the actor, and the
//! portal blocks near the actor.
//! `cargo run -p sapling --example bds_portal_probe -- <server> <drill> [full|none|early]`
//! Drills: see `drill` in shared/portal_drill.rs. The answer to a ChangeDimension: `full` (default)
//! starts the loading screen at once and ends it with a DimensionChangeAck when the server sends
//! its PlayerAction (or a PlayStatus), `early` ends it at once, `none` sends nothing.
#[path = "shared/dump.rs"]
#[allow(dead_code)]
mod dump;
#[path = "shared/portal_drill.rs"]
mod portal;
#[path = "shared/probe.rs"]
#[allow(dead_code)]
mod probe;

use std::time::{Duration, Instant};

use acacia_bot::interact::Face;
use acacia_bot::proto::packets::{ChangeDimension, MovePlayer, MovePlayerMode, PlayStatus, PlayerAction, ServerboundLoadingScreen, UpdateBlock, UpdateSubchunkBlocks};
use acacia_bot::proto::types::{Action, BlockCoordinates};
use acacia_bot::proto::{Packet, RawPacket};
use acacia_bot::{Bot, BotEvent};
use acacia_world::{BlockAccess, BlockRegistry};

/// Printed in full; other packets by name only.
const DETAILED: &[&str] = &[
    "ChangeDimension", "PlayStatus", "MovePlayer", "SetSpawnPosition", "NetworkChunkPublisherUpdate", "ChunkRadiusUpdate", "Respawn", "UpdateAttributes", "SetTime", "StopSound",
    "CommandOutput", "Text", "SetPlayerGameType", "UpdateAbilities", "UpdateAdventureSettings", "CorrectPlayerMovePrediction", "LevelEvent", "LevelSoundEvent", "RemoveEntity", "AddPlayer",
    "GameRulesChanged", "PlayerList", "ClientboundCloseForm", "LevelEventGeneric", "EntityEvent", "MobEffect", "SetHealth", "CameraInstruction", "ContainerClose", "PlayerAction",
    "PlayerLocation", "Event", "AddPlayer", "InventoryContent", "InventorySlot",
];
/// Blocks whose UpdateBlock is printed; others are tallied.
const BLOCKS_SHOWN: &[&str] = &["portal", "obsidian", "fire", "air"];
/// Counted per dimension and printed as a tally at the next other packet.
const TALLIED: &[&str] = &[
    "LevelChunk", "Subchunk", "ClientCacheMissResponse", "MoveEntityDelta", "SetEntityMotion", "SyncWorldClocks", "NetworkStackLatency", "CurrentStructureFeature", "SetEntityData",
    "MoveEntity", "UpdateAttributes", "InventoryContent",
];
/// What the watcher shows: what it is told of the actor.
const WATCHED: &[&str] = &["RemoveEntity", "AddPlayer", "PlayerList", "MovePlayer", "LevelEventGeneric", "PlayerLocation", "Text"];
const SHOWN: usize = 900;

#[derive(Clone, Copy, PartialEq)]
enum Answer {
    Full,
    Early,
    Silent,
}

struct Run {
    start: Instant,
    answer: Answer,
    /// The loading screen of the dimension change under way, until it is ended.
    screen: Option<Option<u32>>,
    tally: Vec<(String, usize)>,
}

impl Run {
    fn ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    fn flush_tally(&mut self) {
        if !self.tally.is_empty() {
            let shown: Vec<String> = self.tally.drain(..).map(|(name, n)| format!("{name} x{n}")).collect();
            println!("{:>7} actor   .. {}", self.ms(), shown.join(", "));
        }
    }

    fn count(&mut self, name: String) {
        match self.tally.iter_mut().find(|(seen, _)| *seen == name) {
            Some((_, n)) => *n += 1,
            None => self.tally.push((name, 1)),
        }
    }

    fn end_screen(&mut self, actor: &Bot) {
        if let Some(id) = self.screen.take() {
            actor.client().send(&ServerboundLoadingScreen { r#type: 2, loading_screen_id: id });
            let at = BlockCoordinates { x: 0, y: 0, z: 0 };
            let runtime_entity_id = actor.state().player.runtime_entity_id;
            actor.client().send(&PlayerAction { runtime_entity_id, action: Action::DimensionChangeAck, position: at.clone(), result_position: at, face: 0 });
            println!("{:>7} actor   >> loading screen end {id:?} + DimensionChangeAck", self.ms());
        }
    }

    /// Prints one actor packet and answers a dimension change.
    fn actor_packet(&mut self, actor: &Bot, packet: &RawPacket) {
        let name = acacia_bot::proto::packet_name(packet.id).unwrap_or("?");
        if TALLIED.contains(&name) {
            let dimension = match name {
                "LevelChunk" => packet.decode::<acacia_bot::proto::packets::LevelChunk>().map(|c| c.dimension).unwrap_or(-1),
                "Subchunk" => packet.decode::<acacia_bot::proto::packets::Subchunk>().map(|c| c.dimension).unwrap_or(-1),
                _ => return self.count(name.to_string()),
            };
            return self.count(format!("{name}[dim {dimension}]"));
        }
        if packet.id == UpdateBlock::ID {
            if let Ok(update) = packet.decode::<UpdateBlock>() {
                let registry = BlockRegistry::vanilla();
                let block = registry.runtime_id_from_hash(update.block_runtime_id).and_then(|id| registry.get(id)).map_or("?".into(), |s| format!("{} [{}]", s.name, s.properties));
                if !BLOCKS_SHOWN.iter().any(|shown| block.contains(shown)) {
                    return self.count(format!("UpdateBlock {}", block.split(' ').next().unwrap_or("?")));
                }
                self.flush_tally();
                let p = &update.position;
                println!("{:>7} actor   UpdateBlock {} {} {} layer {} flags {:?} {block}", self.ms(), p.x, p.y, p.z, update.layer, update.flags);
            }
            return;
        }
        if packet.id == UpdateSubchunkBlocks::ID {
            let Ok(update) = packet.decode::<UpdateSubchunkBlocks>() else { return };
            let registry = BlockRegistry::vanilla();
            let name = |hash: u32| registry.runtime_id_from_hash(hash).and_then(|id| registry.get(id)).map_or("?".into(), |s| format!("{}[{}]", s.name.trim_start_matches("minecraft:"), s.properties));
            let cells = |blocks: &[acacia_bot::proto::types::BlockUpdate]| blocks.iter().map(|b| format!("{} {} {} {} f{}", b.position.x, b.position.y, b.position.z, name(b.runtime_id), b.flags)).collect::<Vec<_>>().join("; ");
            if update.blocks.iter().chain(&update.extra).all(|b| !BLOCKS_SHOWN.iter().any(|shown| name(b.runtime_id).contains(shown))) {
                return self.count("UpdateSubchunkBlocks[other]".into());
            }
            self.flush_tally();
            println!("{:>7} actor   UpdateSubchunkBlocks at {} {} {}: {} | extra: {}", self.ms(), update.x, update.y, update.z, cells(&update.blocks), cells(&update.extra));
            return;
        }
        if packet.id == MovePlayer::ID && packet.decode::<MovePlayer>().is_ok_and(|moved| moved.mode == MovePlayerMode::Normal) {
            return self.count("MovePlayer[normal]".into());
        }
        self.flush_tally();
        let text = if DETAILED.contains(&name) { dump::dump(packet).unwrap_or_default().chars().take(SHOWN).collect() } else { String::new() };
        println!("{:>7} actor   {name} {text}", self.ms());
        if packet.id == ChangeDimension::ID && self.answer != Answer::Silent {
            let id = packet.decode::<ChangeDimension>().map(|change| change.loading_screen_id).unwrap_or(None);
            actor.client().send(&ServerboundLoadingScreen { r#type: 1, loading_screen_id: id });
            println!("{:>7} actor   >> loading screen start {id:?}", self.ms());
            self.screen = Some(id);
            if self.answer == Answer::Early {
                self.end_screen(actor);
            }
        }
        if packet.id == PlayStatus::ID || packet.id == PlayerAction::ID {
            self.end_screen(actor);
        }
    }

    async fn settle(&mut self, actor: &mut Bot, watcher: &mut Bot, wait: Duration) -> Result<(), String> {
        let end = tokio::time::Instant::now() + wait;
        loop {
            tokio::select! {
                event = actor.next() => match event {
                    Some(BotEvent::Packet(packet)) => self.actor_packet(actor, &packet),
                    Some(BotEvent::Disconnected(reason)) => return Err(format!("actor disconnected: {reason:?}")),
                    Some(_) => {}
                    None => return Err("actor is gone".into()),
                },
                event = watcher.next() => match event {
                    Some(BotEvent::Packet(packet)) => {
                        let name = acacia_bot::proto::packet_name(packet.id).unwrap_or("?");
                        let normal = packet.id == MovePlayer::ID && packet.decode::<MovePlayer>().is_ok_and(|moved| moved.mode == MovePlayerMode::Normal);
                        if WATCHED.contains(&name) && !normal {
                            let text: String = dump::dump(&packet).unwrap_or_default().chars().take(300).collect();
                            println!("{:>7} watcher {name} {text}", self.ms());
                        }
                    }
                    Some(BotEvent::Disconnected(reason)) => return Err(format!("watcher disconnected: {reason:?}")),
                    Some(_) => {}
                    None => return Err("watcher is gone".into()),
                },
                _ = tokio::time::sleep_until(end) => break,
            }
        }
        self.flush_tally();
        Ok(())
    }
}

/// The portal blocks within `reach` of the actor's feet in the world it is shown, as runs of one axis.
fn portals(actor: &Bot, reach: i32) {
    let feet = actor.movement().and_then(|movement| movement.position()).map(|at| at.map(|c| c.floor() as i32));
    let world = actor.world();
    let (Some(feet), Some(view), Some(registry)) = (feet, world.and_then(|w| w.view()), world.and_then(|w| w.registry())) else { return println!("## portals: no view") };
    let mut found = Vec::new();
    for x in feet[0] - reach..=feet[0] + reach {
        for z in feet[2] - reach..=feet[2] + reach {
            for y in (feet[1] - reach).max(-64)..=(feet[1] + reach).min(319) {
                if let Some(state) = registry.get(view.block(x, y, z)).filter(|state| state.name == "minecraft:portal") {
                    found.push(([x, y, z], state.properties.to_string()));
                }
            }
        }
    }
    println!("## portals near {feet:?} in dimension {}: {}", actor.state().player.dimension, found.len());
    let lowest = |axis: usize| found.iter().map(|(p, _)| p[axis]).min();
    let highest = |axis: usize| found.iter().map(|(p, _)| p[axis]).max();
    if let (Some(x0), Some(y0), Some(z0), Some(x1), Some(y1), Some(z1)) = (lowest(0), lowest(1), lowest(2), highest(0), highest(1), highest(2)) {
        println!("## portal box {x0} {y0} {z0} .. {x1} {y1} {z1}, states {:?}", found.iter().map(|(_, s)| s.as_str()).collect::<std::collections::BTreeSet<_>>());
        for (y, x) in [(y0 - 1, x0 - 1), (y1 + 1, x1 + 1)] {
            let name = registry.get(view.block(x, y, z0)).map_or("?", |s| s.name);
            println!("## frame corner {x} {y} {z0}: {name}");
        }
    }
}

fn position(actor: &Bot) {
    let feet = actor.movement().and_then(|movement| movement.position());
    println!("## at {feet:?} dimension {}", actor.state().player.dimension);
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let server = args.next().ok_or("usage: bds_portal_probe <server> <drill> [full|none|early]")?;
    let name = args.next().unwrap_or_else(|| "travel".into());
    let lines = portal::drill(&name).ok_or(format!("no drill {name}"))?;
    let answer = match args.next().as_deref() {
        Some("none") => Answer::Silent,
        Some("early") => Answer::Early,
        _ => Answer::Full,
    };
    let (mut actor, mut watcher) = (probe::connect(&server, "PortalActor").await?, probe::connect(&server, "PortalWatcher").await?);
    watcher.wait_ticks(60).await?;
    actor.wait_ticks(60).await?;
    for bot in [&mut actor, &mut watcher] {
        while let Ok(Some(_)) = tokio::time::timeout(Duration::from_millis(5), bot.next()).await {}
    }
    let mut run = Run { start: Instant::now(), answer, screen: None, tally: Vec::new() };
    for line in &lines {
        println!("## {line} at {}", run.ms());
        let wait = match portal::step(line) {
            portal::Step::Command(command, wait) => {
                actor.client().command(command);
                wait
            }
            portal::Step::Light(pos, face, wait) => {
                let face = if face == "up" { Face::Up } else { Face::North };
                println!("## lit: {:?}", actor.use_item_on_block(pos, face).await);
                wait
            }
            portal::Step::Wait(wait) => wait,
            portal::Step::Portals(reach) => {
                portals(&actor, reach);
                Duration::ZERO
            }
            portal::Step::Where => {
                position(&actor);
                Duration::ZERO
            }
        };
        run.settle(&mut actor, &mut watcher, wait).await?;
    }
    actor.disconnect().await;
    watcher.disconnect().await;
    Ok(())
}
