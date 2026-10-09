//! Sets every vanilla block named bare (`/setblock x y z minecraft:chest`) and prints the state the
//! server answers with, as `name<TAB>properties`, or `#none name` for a block it would not set.
//! The ground truth for `sapling-data/data/block-defaults.tsv`.
//! `cargo run -p sapling --example bds_defaults_probe -- <server>` (offline mode, an operator).
#[path = "shared/dump.rs"]
#[allow(dead_code)]
mod dump;
#[path = "shared/probe.rs"]
#[allow(dead_code)]
mod probe;

use std::collections::HashMap;
use std::time::Duration;

use acacia_bot::BotEvent;
use acacia_bot::proto::{Packet, packets};
use acacia_world::BlockRegistry;

/// The blocks are set in a square this wide around the spawn, layer over layer upwards from `FLOOR`,
/// `GAP` apart every way so that none is set beside another (a fence would join it).
const SIDE: i32 = 32;
const FLOOR: i32 = -40;
const GAP: i32 = 2;
/// Layers apart, so that what one layer's blocks do lands between them.
const LAYER_GAP: i32 = 4;

fn pos(index: usize) -> [i32; 3] {
    let (i, row) = (index as i32, SIDE / GAP);
    let (layer, cell) = (i / (row * row), i % (row * row));
    [
        cell % row * GAP - SIDE / 2,
        FLOOR + layer * LAYER_GAP,
        cell / row * GAP - SIDE / 2,
    ]
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args()
        .nth(1)
        .ok_or("usage: bds_defaults_probe <server>")?;
    let registry = BlockRegistry::vanilla();
    let mut names: Vec<&str> = (0..registry.len() as u32)
        .filter_map(|id| registry.get(id))
        .map(|state| state.name)
        .collect();
    names.sort_unstable();
    names.dedup();
    let mut bot = probe::connect(&server, "Defaults").await?;
    bot.wait_ticks(60).await?;
    let top = pos(names.len() - 1)[1];
    for y in (FLOOR..=top).step_by(LAYER_GAP as usize) {
        bot.client().command(&format!(
            "fill {} {y} {} {} {y} {} air",
            -SIDE / 2,
            -SIDE / 2,
            SIDE / 2 - 1,
            SIDE / 2 - 1
        ));
    }
    bot.wait_ticks(20).await?;
    let mut seen: HashMap<[i32; 3], Vec<u32>> = HashMap::new();
    let mut watch = |event: Option<BotEvent>| -> Result<(), String> {
        match event {
            Some(BotEvent::Packet(packet)) if packet.id == packets::UpdateBlock::ID => {
                let update = packet
                    .decode::<packets::UpdateBlock>()
                    .map_err(|e| e.to_string())?;
                let at = [update.position.x, update.position.y, update.position.z];
                seen.entry(at).or_default().push(update.block_runtime_id);
                Ok(())
            }
            // Several changes to one sub-chunk in a tick come together.
            Some(BotEvent::Packet(packet)) if packet.id == packets::UpdateSubchunkBlocks::ID => {
                let updates = packet
                    .decode::<packets::UpdateSubchunkBlocks>()
                    .map_err(|e| e.to_string())?;
                for update in updates.blocks {
                    let at = [update.position.x, update.position.y, update.position.z];
                    seen.entry(at).or_default().push(update.runtime_id);
                }
                Ok(())
            }
            Some(BotEvent::Disconnected(reason)) => Err(format!("disconnected: {reason:?}")),
            None => Err("the bot is gone".into()),
            Some(_) => Ok(()),
        }
    };
    for (index, name) in names.iter().enumerate() {
        let [x, y, z] = pos(index);
        bot.client()
            .command(&format!("setblock {x} {y} {z} {name}"));
        let end = tokio::time::Instant::now() + Duration::from_millis(50);
        loop {
            tokio::select! {
                event = bot.next() => watch(event)?,
                _ = tokio::time::sleep_until(end) => break,
            }
        }
    }
    let end = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        tokio::select! {
            event = bot.next() => watch(event)?,
            _ = tokio::time::sleep_until(end) => break,
        }
    }
    bot.disconnect().await;
    for (index, name) in names.iter().enumerate() {
        let state = seen
            .get(&pos(index))
            .into_iter()
            .flatten()
            .filter_map(|hash| registry.runtime_id_from_hash(*hash))
            .filter_map(|id| registry.get(id))
            .find(|state| state.name == *name);
        match state {
            Some(state) => println!("{name}\t{}", state.properties),
            None => println!("#none {name}"),
        }
    }
    Ok(())
}
