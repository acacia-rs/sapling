//! The join sequence, packet by packet (harness: common/mod.rs).

mod common;

use acacia_proto::Packet;
use acacia_proto::packets::{
    AvailableEntityIdentifiers, BiomeDefinitionList, ChunkRadiusUpdate, CreativeContent, ItemRegistry, LevelChunk, NetworkChunkPublisherUpdate,
    PlayStatus, PlayStatusStatus, RequestChunkRadius, ResourcePackClientResponseResponseStatus as PackStatus, ResourcePackStack, ResourcePacksInfo,
    SetEntityData, SetLocalPlayerAsInitialized, StartGame, Subchunk, SubchunkRequest, UpdateAbilities, UpdateAttributes,
};
use acacia_proto::types::{SubChunkEntryItemResult, Vec3i8, Vec3li};
use common::{Harness, all, ids, pack_response};

#[test]
fn a_player_is_walked_through_the_join_sequence() {
    let mut h = Harness::new();
    h.join(1, "Steve");
    assert_eq!(ids(&h.tick(1)), [ResourcePacksInfo::ID]);
    assert!(h.tick(1).is_empty(), "the shard waits for the client");

    h.receive(1, &pack_response(PackStatus::HaveAllPacks));
    assert_eq!(ids(&h.tick(1)), [ResourcePackStack::ID]);

    h.receive(1, &pack_response(PackStatus::Completed));
    let sent = h.tick(1);
    use acacia_proto::packets::{CameraPresets, FeatureRegistry, JigsawStructureData, TrimData, VoxelShapes};
    let expected = [
        JigsawStructureData::ID,
        VoxelShapes::ID,
        StartGame::ID,
        ItemRegistry::ID,
        BiomeDefinitionList::ID,
        AvailableEntityIdentifiers::ID,
        FeatureRegistry::ID,
        CameraPresets::ID,
        CreativeContent::ID,
        TrimData::ID,
        UpdateAbilities::ID,
        SetEntityData::ID,
        UpdateAttributes::ID,
    ];
    assert_eq!(ids(&sent), expected);
    let start: StartGame = sent[2].decode().unwrap();
    assert_eq!((start.runtime_entity_id, start.world_name.as_str()), (1, "test"));
    assert_eq!((start.player_position.x, start.player_position.y, start.player_position.z), (0.5, 65.0 + sapling_data::EYE_HEIGHT, 0.5));
    assert!(h.tick(1).is_empty(), "no chunks before the client asks for a radius");

    // Radius 9 is capped to the configured 4: 49 columns in the circle, sent 16 a tick.
    h.receive(1, &RequestChunkRadius { chunk_radius: 9, max_radius: 9 });
    let sent = h.tick(1);
    assert_eq!(sent[0].decode::<ChunkRadiusUpdate>().unwrap().chunk_radius, 4);
    assert_eq!(sent[1].decode::<NetworkChunkPublisherUpdate>().unwrap().radius, 64);
    let chunks: Vec<LevelChunk> = sent[2..].iter().map(|p| p.decode().unwrap()).collect();
    assert_eq!(chunks.len(), 16);
    assert_eq!((chunks[0].x, chunks[0].z, chunks[0].highest_subchunk_count, chunks[0].cache_enabled), (0, 0, Some(9), false));

    // The player spawns with the 29 columns within three chunks sent; the rest follow.
    let sent = h.tick(1);
    assert_eq!((all::<LevelChunk>(&sent).len(), sent.len()), (16, 17));
    assert_eq!(sent.last().unwrap().decode::<PlayStatus>().unwrap().status, PlayStatusStatus::PlayerSpawn);
    assert_eq!(ids(&h.tick(1)), [LevelChunk::ID; 16]);
    assert_eq!(ids(&h.tick(1)), [LevelChunk::ID]);
    assert!(h.tick(1).is_empty());
    assert!(h.log().is_empty(), "not joined until the client has loaded");

    let requests = vec![Vec3i8 { x: 0, y: 4, z: 0 }, Vec3i8 { x: 3, y: 4, z: 0 }, Vec3i8 { x: 0, y: 30, z: 0 }];
    h.receive(1, &SubchunkRequest { dimension: 0, requests, origin: Vec3li { x: 0, y: 0, z: 0 } });
    let sent = h.tick(1);
    let answer: Subchunk = sent[0].decode().unwrap();
    let results: Vec<_> = answer.entries.iter().map(|e| e.result).collect();
    assert_eq!(results, [SubChunkEntryItemResult::Success, SubChunkEntryItemResult::SuccessAllAir, SubChunkEntryItemResult::YIndexOutOfBounds]);
    assert!(!answer.cache_enabled);

    h.receive(1, &SetLocalPlayerAsInitialized { runtime_entity_id: 1 });
    assert!(h.tick(1).is_empty());
    assert_eq!(h.log(), ["join Steve (1 online)"]);

    h.leave(1);
    h.tick(1);
    assert_eq!(h.log(), ["join Steve (1 online)", "quit Steve (0 online)"]);
}

#[test]
fn players_get_their_own_ids_and_leaving_early_is_not_a_quit() {
    let mut h = Harness::new();
    h.join(1, "Alex");
    h.join(2, "Steve");
    h.shard.tick();
    while h.net.sent.try_recv().is_ok() {}
    for conn in [1, 2] {
        h.receive(conn, &pack_response(PackStatus::HaveAllPacks));
    }
    h.shard.tick();
    while h.net.sent.try_recv().is_ok() {}
    h.receive(2, &pack_response(PackStatus::Completed));
    let start: StartGame = h.tick(2)[2].decode().unwrap();
    assert_eq!((start.runtime_entity_id, start.entity_id), (2, 2));

    // Out-of-order packets are ignored: Alex has no StartGame yet.
    h.receive(1, &RequestChunkRadius { chunk_radius: 4, max_radius: 4 });
    h.receive(1, &SetLocalPlayerAsInitialized { runtime_entity_id: 1 });
    assert!(h.tick(1).is_empty());

    h.leave(1);
    h.shard.tick();
    assert!(h.log().is_empty(), "a player who never finished joining does not quit either");
}
