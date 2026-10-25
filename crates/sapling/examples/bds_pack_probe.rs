//! Records a server's resource pack negotiation as a fresh client sees it: ResourcePacksInfo, the
//! stack, each pack's data info and every chunk's index, offset and size, then whether the bytes
//! match the hash, until StartGame. The ground truth for sapling's pack serving (`sapling-shard`
//! `packs.rs`); BDS with tools/bds-packs-setup.sh's packs.
//! `cargo run -p sapling --example bds_pack_probe -- <server>` (an offline-mode server)
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use acacia_bot::proto::{Packet, packets};
use acacia_client::{Client, Event, MemoryPackStore, PacketFilter};
use sha2::{Digest, Sha256};

/// Commands whose syntax is printed from AvailableCommands.
const SHOWN_COMMANDS: [&str; 3] = ["function", "schedule", "scriptevent"];

/// A parameter as `name:type`, an enum's values or a soft enum's name in its place.
fn parameter(list: &packets::AvailableCommands, p: &packets::AvailableCommandsCommandDataItemOverloadsItemParametersItem) -> String {
    use packets::AvailableCommandsCommandDataItemOverloadsItemParametersItemEnumType as Kind;
    let kind = match p.enum_type {
        Kind::Enum => list.enums.get(usize::from(p.value_type)).map_or("?".into(), |e| {
            let values: Vec<&str> = e.values.iter().map(|&v| list.enum_values.get(v as usize).map_or("?", String::as_str)).collect();
            format!("{}[{}]", e.name, values.join("|"))
        }),
        Kind::SoftEnum => list.dynamic_enums.get(usize::from(p.value_type)).map_or("?".into(), |e| format!("soft {} {:?}", e.name, e.values)),
        other => format!("type {} ({other:?})", p.value_type),
    };
    format!("{}{}:{kind}", p.parameter_name, if p.optional { "?" } else { "" })
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::env::args().nth(1).ok_or("usage: bds_pack_probe <server>")?;
    let store = std::sync::Arc::new(MemoryPackStore::default());
    let builder = Client::builder(server).offline("PackProbe").filter(PacketFilter::all()).pack_store(store).login_timeout(Duration::from_secs(120));
    let mut client = builder.connect().await?;
    let start = Instant::now();
    let mut chunks: BTreeMap<String, Vec<(u32, u64, Vec<u8>)>> = BTreeMap::new();
    let mut hashes: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    while let Some(event) = tokio::time::timeout(Duration::from_secs(120), client.recv()).await? {
        let Event::Packet(packet) = event else { continue };
        let ms = start.elapsed().as_millis();
        match packet.id {
            packets::ResourcePacksInfo::ID => println!("{ms:>6} {:?}", packet.decode::<packets::ResourcePacksInfo>()?),
            packets::ResourcePackStack::ID => println!("{ms:>6} {:?}", packet.decode::<packets::ResourcePackStack>()?),
            packets::ResourcePackDataInfo::ID => {
                let info = packet.decode::<packets::ResourcePackDataInfo>()?;
                println!(
                    "{ms:>6} ResourcePackDataInfo pack_id={} max_chunk_size={} chunk_count={} size={} hash_len={} premium={} type={:?}",
                    info.pack_id, info.max_chunk_size, info.chunk_count, info.size, info.hash.len(), info.is_premium, info.pack_type
                );
                hashes.insert(info.pack_id.clone(), info.hash.to_vec());
            }
            packets::ResourcePackChunkData::ID => {
                let chunk = packet.decode::<packets::ResourcePackChunkData>()?;
                println!("{ms:>6} ResourcePackChunkData pack_id={} index={} progress={} len={}", chunk.pack_id, chunk.chunk_index, chunk.progress, chunk.payload.len());
                chunks.entry(chunk.pack_id).or_default().push((chunk.chunk_index, chunk.progress, chunk.payload.to_vec()));
            }
            packets::StartGame::ID => println!("{ms:>6} StartGame"),
            packets::AvailableCommands::ID => {
                let list = packet.decode::<packets::AvailableCommands>()?;
                for command in list.command_data.iter().filter(|c| SHOWN_COMMANDS.contains(&c.name.as_str())) {
                    for overload in &command.overloads {
                        let parameters: Vec<String> = overload.parameters.iter().map(|p| parameter(&list, p)).collect();
                        println!("{ms:>6} /{} {}", command.name, parameters.join(" "));
                    }
                }
                break;
            }
            _ => {}
        }
    }
    for (pack, mut parts) in chunks {
        parts.sort_by_key(|(index, ..)| *index);
        let bytes: Vec<u8> = parts.iter().flat_map(|(.., payload)| payload.clone()).collect();
        let matches = hashes.get(&pack).is_some_and(|hash| hash[..] == Sha256::digest(&bytes)[..]);
        let zip = bytes.starts_with(b"PK\x03\x04");
        println!("## {pack}: {} bytes, sha256 matches: {matches}, zip: {zip}", bytes.len());
    }
    client.close();
    Ok(())
}
