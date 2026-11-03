//! What bds_command_probe prints of a packet: CommandOutput in full, GameRulesChanged as `rule=value`, and the
//! drill's own packets.
use std::time::Instant;

use acacia_bot::BotEvent;
use acacia_bot::proto::{Packet, packets};

use crate::{clock, dump, maps, parity, protocol, structures};

/// What a `*` drill leaves out: the packets that come whatever a command does.
const EVERY_TICK: &[&str] = &["MovePlayer", "MoveEntityAbsolute", "MoveEntityDelta", "SetEntityMotion", "SetEntityData", "UpdateAttributes", "NetworkChunkPublisherUpdate", "LevelChunk", "SubChunk", "NetworkStackLatency", "CurrentStructureFeature"];

/// Characters of a drill's own packets shown.
const SHOWN: usize = 12000;

pub fn show(start: Instant, shown: &[&str], who: &str, event: Option<BotEvent>) -> Result<(), String> {
    let packet = match event {
        Some(BotEvent::Packet(packet)) => packet,
        Some(BotEvent::Disconnected(reason)) => return Err(format!("{who} disconnected: {reason:?}")),
        Some(_) => return Ok(()),
        None => return Err(format!("{who} is gone")),
    };
    let ms = clock::tick_ms(start);
    if packet.id == packets::CommandOutput::ID {
        let answer = packet.decode::<packets::CommandOutput>().map_err(|e| e.to_string())?;
        let lines: Vec<String> = answer.output.iter().map(|line| format!("{} success={} {:?}", line.message_id, line.success, line.parameters)).collect();
        println!("{ms:>7} {who:<7} CommandOutput success_count={} data={:?} {}", answer.success_count, answer.data, lines.join(" | "));
    } else if packet.id == packets::GameRulesChanged::ID {
        let rules = packet.decode::<packets::GameRulesChanged>().map_err(|e| e.to_string())?;
        let shown: Vec<String> = rules.rules.iter().map(|rule| format!("{}={:?}", rule.name, rule.value)).collect();
        println!("{ms:>7} {who:<7} GameRulesChanged {}", shown.join(" "));
    } else if let Some(rows) = parity::listed(&packet).filter(|_| shown.contains(&"AvailableCommands")) {
        rows.iter().for_each(|row| println!("{ms:>7} {who:<7} {row}"));
    } else if let Some(rows) = maps::pixels(&packet).filter(|_| shown.contains(&"MapPixels")) {
        let text: String = dump::dump(&packet).unwrap_or_default().split(", pixels").next().unwrap_or_default().to_owned();
        println!("{ms:>7} {who:<7} ClientboundMapItemData {text}");
        rows.iter().for_each(|row| println!("{ms:>7} {who:<7} {row}"));
    } else if let Some(text) = protocol::properties(&packet).filter(|_| shown.contains(&"Properties")) {
        println!("{ms:>7} {who:<7} {text}");
    } else if let Some(cells) = structures::block_names(&packet).filter(|_| shown.contains(&"UpdateSubchunkBlocks")) {
        println!("{ms:>7} {who:<7} {cells}");
    } else if let Some(name) = acacia_bot::proto::packet_name(packet.id).filter(|name| ["Text", "SetTime", "SyncWorldClocks"].contains(name) || shown.contains(name)) {
        let text: String = dump::dump(&packet).unwrap_or_default().chars().take(SHOWN).collect();
        println!("{ms:>7} {who:<7} {name} {text}");
    } else if acacia_bot::proto::packet_name(packet.id).is_none() && (shown.contains(&"Unknown") || shown.contains(&"*")) {
        println!("{ms:>7} {who:<7} Unknown id {} {} bytes", packet.id, packet.body.len());
    } else if let Some(name) = acacia_bot::proto::packet_name(packet.id).filter(|name| shown.contains(&"*") && !EVERY_TICK.contains(name)) {
        println!("{ms:>7} {who:<7} {name} {} bytes", packet.body.len());
    }
    Ok(())
}
