//! Resource packs on join: the info, the data info and chunks a client asks for, a few a tick and
//! none while its connection is backed up, the stack, and turning required packs down
//! (research/captures/bds-packs.txt).

use std::sync::Arc;

use crate::common;

use acacia_proto::Packet;
use acacia_proto::packets::{
    ResourcePackChunkData, ResourcePackChunkRequest, ResourcePackClientResponse, ResourcePackClientResponseResponseStatus as PackStatus, ResourcePackDataInfo, ResourcePackStack,
    ResourcePacksInfo,
};
use common::{Harness, all, config, ids, pack_response, platform};
use sapling_net::Outbound;
use sapling_shard::{ResourcePack, ResourcePacks, ShardConfig};

const UUID: &str = "5d9a0a52-5d2f-4b47-9a43-34c0e2b1f001";
const ID: &str = "5d9a0a52-5d2f-4b47-9a43-34c0e2b1f001_1.0.0";

fn with_pack(required: bool) -> Harness {
    let archive: Vec<u8> = (0..250_000u32).map(|i| (i % 251) as u8).collect();
    let pack = ResourcePack::new(UUID, "1.0.0", archive, 3_000_000).unwrap().with_content_key("key").with_cdn_url("https://cdn.example/p.zip");
    let packs = ResourcePacks { packs: vec![pack], required };
    Harness::with(ShardConfig { resource_packs: Arc::new(packs), ..config() }, platform())
}

fn ask(ids: &[&str]) -> ResourcePackClientResponse {
    ResourcePackClientResponse { resourcepackids: Some(ids.iter().map(|id| id.to_string()).collect()), ..pack_response(PackStatus::SendPacks) }
}

#[test]
fn a_client_is_sent_the_chunks_it_asks_for_then_the_stack() {
    let mut h = with_pack(true);
    h.join(1, "Steve");
    let info = &all::<ResourcePacksInfo>(&h.tick(1))[0];
    let pack = &info.texture_packs[0];
    assert!(info.must_accept);
    assert_eq!((pack.version.as_str(), pack.size, pack.content_key.as_str(), pack.content_identity.as_str(), pack.cdn_url.as_str()), ("1.0.0", 3_000_000, "key", UUID, "https://cdn.example/p.zip"));

    h.receive(1, &ask(&[ID, "00000000-0000-0000-0000-000000000000_1.0.0"]));
    let data_info = &all::<ResourcePackDataInfo>(&h.tick(1))[0];
    assert_eq!((data_info.pack_id.as_str(), data_info.max_chunk_size, data_info.chunk_count, data_info.size, data_info.hash.len()), (ID, 102_400, 3, 250_000, 32));

    for index in 0..3 {
        h.receive(1, &ResourcePackChunkRequest { pack_id: ID.into(), chunk_index: index });
    }
    h.congest(1, true);
    assert!(h.tick(1).is_empty(), "nothing while the connection is backed up");
    h.congest(1, false);
    let first = all::<ResourcePackChunkData>(&h.tick(1));
    let last = all::<ResourcePackChunkData>(&h.tick(1));
    let chunks: Vec<(u32, u64, usize)> = first.iter().chain(&last).map(|chunk| (chunk.chunk_index, chunk.progress, chunk.payload.len())).collect();
    assert_eq!((first.len(), chunks), (2, vec![(0, 0, 102_400), (1, 102_400, 102_400), (2, 204_800, 45_200)]));

    h.receive(1, &pack_response(PackStatus::HaveAllPacks));
    let stack = &all::<ResourcePackStack>(&h.tick(1))[0];
    assert_eq!((stack.must_accept, stack.resource_packs[0].uuid.as_str(), stack.game_version.as_str()), (true, UUID, "*"));
}

#[test]
fn turning_down_required_packs_disconnects_and_optional_ones_are_left_out() {
    let mut h = with_pack(true);
    h.join(1, "Steve");
    h.tick(1);
    h.receive(1, &pack_response(PackStatus::Refused));
    h.shard.tick();
    let kicked = std::iter::from_fn(|| h.net.sent.try_recv().ok()).any(|sent| matches!(sent, Outbound::Kick { conn, ref message, .. } if conn.get() == 1 && message == "disconnectionScreen.resourcePack"));
    assert!(kicked);

    let mut h = with_pack(false);
    h.join(1, "Alex");
    assert!(!all::<ResourcePacksInfo>(&h.tick(1))[0].must_accept);
    h.receive(1, &pack_response(PackStatus::Refused));
    let sent = h.tick(1);
    assert_eq!(ids(&sent), [ResourcePackStack::ID]);
    assert!(all::<ResourcePackStack>(&sent)[0].resource_packs.is_empty());
}
