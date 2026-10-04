//! What the probe examples share: a packet's name and its decoded text.

use acacia_bot::proto::{Packet, RawPacket, packets};

/// Terrain, clock and ambient traffic that says nothing about what a probe looks at.
const NOISE: &[&str] = &[
    "LevelChunk",
    "Subchunk",
    "NetworkChunkPublisherUpdate",
    "ClientCacheMissResponse",
    "SyncWorldClocks",
    "CurrentStructureFeature",
    "SetTime",
    "UpdateBlock",
    "LevelSoundEvent",
    "LevelEvent",
];

/// The packet's name, unless it is noise.
pub fn name(packet: &RawPacket) -> Option<&'static str> {
    let name = acacia_bot::proto::packet_name(packet.id).unwrap_or("?");
    (!NOISE.iter().any(|noise| noise.eq_ignore_ascii_case(name))).then_some(name)
}

macro_rules! dumper {
    ($($t:ident),* $(,)?) => {
        /// The packet decoded and printed with `Debug`; `None` for an id without a decoder.
        pub fn dump(p: &RawPacket) -> Option<String> {
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
