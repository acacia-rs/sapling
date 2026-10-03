//! Regenerates `crates/sapling-data/data/*.bin` from the BDS join recorded in acacia-testserver
//! (see sapling-data's README.md). `cargo run -p extract-data -- [--list | --show Name,Name]`
//! `--list` prints the recorded sequence, `--show` the named packets decoded (pretty Debug; redirect it).
use std::path::Path;

use acacia_proto::packets::{self, PlayStatus, SetEntityData, UpdateAttributes};
use acacia_proto::{Packet, RawPacket, packet_name};
use acacia_testserver::{Script, Step};

/// The first recorded packet of each of these, saved as `<name>.bin`.
const TEMPLATES: &[&str] = &[
    "JigsawStructureData",
    "VoxelShapes",
    "FeatureRegistry",
    "CameraPresets",
    "TrimData",
    "StartGame",
    "ItemRegistry",
    "BiomeDefinitionList",
    "AvailableEntityIdentifiers",
    "CreativeContent",
    "UpdateAbilities",
    "SetEntityData",
];

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let list = args.iter().any(|a| a == "--list");
    let show: Vec<&str> = args.iter().position(|a| a == "--show").and_then(|i| args.get(i + 1)).map_or(Vec::new(), |s| s.split(',').collect());
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/sapling-data/data");
    let mut written: Vec<&str> = Vec::new();
    let mut spawned = false;
    for step in Script::bds_spawn().steps {
        let packet = match step {
            Step::Wait(id) => {
                if list {
                    println!("WAIT {}", packet_name(id).unwrap_or("?"));
                }
                continue;
            }
            Step::Send { packet, .. } => RawPacket::parse(packet).expect("recorded packets have headers"),
        };
        let name = packet_name(packet.id).unwrap_or("?");
        if list {
            println!("send {name} ({} bytes)", packet.body.len());
        }
        if show.contains(&name) {
            println!("{name}: {}", dump(&packet).unwrap_or_default());
        }
        spawned |= packet.id == PlayStatus::ID && written.contains(&"StartGame");
        // The last UpdateAttributes before spawn is the full list; of the rest the first is the
        // player's own (later SetEntityData are other entities).
        let keep = if packet.id == UpdateAttributes::ID { !spawned } else { TEMPLATES.contains(&name) && !written.contains(&name) };
        if !keep {
            continue;
        }
        assert!(packet.id != SetEntityData::ID || written.contains(&"StartGame"));
        written.push(name);
        if !list && show.is_empty() {
            println!("{name}.bin: {} bytes", packet.body.len());
            std::fs::write(dir.join(format!("{name}.bin")), &packet.body)?;
        }
    }
    assert!(TEMPLATES.iter().all(|t| written.contains(t)), "the recording lacks a template packet");
    Ok(())
}

macro_rules! dumper {
    ($($t:ident),* $(,)?) => {
        fn dump(p: &RawPacket) -> Option<String> {
            $(if p.id == <packets::$t>::ID {
                return Some(match p.decode::<packets::$t>() {
                    Ok(v) => format!("{v:#?}"),
                    Err(e) => format!("<decode error: {e}>"),
                });
            })*
            None
        }
    };
}
acacia_proto::for_each_packet!(dumper);
