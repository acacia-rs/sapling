//! The time of day as BDS 1.26.52 sends it: no SetTime, but one registry of world clocks at the
//! join and the state of each now and then (`sapling` example `bds_entity_probe`). The ids are
//! the ones BDS sent.

use acacia_proto::packets::{
    SyncWorldClocks, SyncWorldClocksContent as Content, SyncWorldClocksContentInitializeRegistry as Registry, SyncWorldClocksContentSyncState as State,
    SyncWorldClocksPayloadType as Kind,
};
use acacia_proto::types::{SyncWorldClockStateData, TimeMarkerData, WorldClockData};

const OVERWORLD: u64 = 7194480507151251734;
const DAY: i32 = 24000;
const MARKERS: [(u64, &str, i32); 6] = [
    (2625810911898139949, "minecraft:sunrise", 23000),
    (4918950784056990566, "minecraft:night", 13000),
    (6827470627776846754, "minecraft:noon", 6000),
    (11262090321339182944, "minecraft:midnight", 18000),
    (13638948813458750018, "minecraft:day", 1000),
    (16664792990819124822, "minecraft:sunset", 12000),
];

/// The clock a client is given when it joins. `time` is the running time of day counter;
/// `paused` when the daylight cycle is off.
pub fn world_clocks(time: i32, paused: bool) -> SyncWorldClocks {
    let time_markers = MARKERS.map(|(id, name, time)| TimeMarkerData { id, name: name.into(), time, period: Some(DAY) }).into();
    let clocks = vec![WorldClockData { id: OVERWORLD, name: "minecraft:overworld".into(), time, paused, time_markers }];
    SyncWorldClocks { payload_type: Kind::InitializeRegistry, content: Content::InitializeRegistry(Registry { clocks }) }
}

/// The clock's state, for a client that has [`world_clocks`].
pub fn clock_state(time: i32, paused: bool) -> SyncWorldClocks {
    let sync_states = vec![SyncWorldClockStateData { clock_id: OVERWORLD, time, paused }];
    SyncWorldClocks { payload_type: Kind::SyncState, content: Content::SyncState(State { sync_states }) }
}

#[cfg(test)]
mod tests {
    use acacia_proto::Packet;
    use bytes::BytesMut;

    use super::*;

    #[test]
    fn both_clock_packets_survive_the_wire() {
        for packet in [world_clocks(13789, false), clock_state(14046, true)] {
            let mut out = BytesMut::new();
            packet.encode(&mut out);
            assert_eq!(SyncWorldClocks::decode(&mut &out[..]).unwrap(), packet);
        }
    }
}
