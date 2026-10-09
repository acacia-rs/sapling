//! Chunk sending past the join: the blob cache, a moving view, and worlds that load from storage.

mod common;

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use acacia_proto::packets::{
    ClientCacheBlobStatus, ClientCacheMissResponse, ClientCacheStatus, LevelChunk,
    NetworkChunkPublisherUpdate, PlayStatus, Subchunk, SubchunkRequest,
};
use acacia_proto::types::{SubChunkEntryItemResult, Vec3i8, Vec3li};
use acacia_proto::{RawPacket, xxh64};
use common::{Harness, all, config, input_at, platform};
use sapling_shard::ShardConfig;
use sapling_world::{
    BlockRegistry, Chunk, ChunkPos, Dimension, MemoryStorage, StorageError, StoredColumn, World,
    WorldStorage,
};

fn request(sections: &[(i8, i8, i8)]) -> SubchunkRequest {
    let requests = sections
        .iter()
        .map(|&(x, y, z)| Vec3i8 { x, y, z })
        .collect();
    SubchunkRequest {
        dimension: 0,
        requests,
        origin: Vec3li { x: 0, y: 0, z: 0 },
    }
}

/// Ticks until nothing more is sent, returning everything that was.
fn drain(h: &mut Harness, conn: u64) -> Vec<RawPacket> {
    let mut sent = Vec::new();
    loop {
        let more = h.tick(conn);
        if more.is_empty() {
            return sent;
        }
        sent.extend(more);
    }
}

#[test]
fn a_client_with_a_blob_cache_gets_hashes_then_the_blobs_it_misses() {
    let mut h = Harness::new();
    h.join_to_radius(1, 4);
    h.receive(1, &ClientCacheStatus { enabled: true });
    let chunks = all::<LevelChunk>(&drain(&mut h, 1));
    assert_eq!(chunks.len(), 49);
    assert!(
        chunks
            .iter()
            .all(|c| c.cache_enabled && c.blobs.len() == 1 && c.payload[..] == [0])
    );
    let (platform, void) = (chunks[0].blobs[0], chunks.last().unwrap().blobs[0]);
    assert_eq!(
        platform, void,
        "biomes are the same everywhere, so every column names one blob"
    );

    h.receive(1, &request(&[(0, 4, 0), (0, 5, 0)]));
    let answer: Subchunk = h.tick(1)[0].decode().unwrap();
    let (stone, air) = (&answer.entries[0], &answer.entries[1]);
    assert!(answer.cache_enabled);
    assert_eq!(
        (
            stone.result,
            stone.payload.as_deref(),
            air.result,
            air.blob_id
        ),
        (
            SubChunkEntryItemResult::Success,
            Some(&[][..]),
            SubChunkEntryItemResult::SuccessAllAir,
            None
        )
    );
    let section = stone.blob_id.expect("a section hash");

    // One reply per status, in the order asked, also when nothing is missing.
    h.receive(
        1,
        &ClientCacheBlobStatus {
            missing: vec![section, platform, 12345],
            have: vec![],
        },
    );
    h.receive(
        1,
        &ClientCacheBlobStatus {
            missing: vec![],
            have: vec![platform],
        },
    );
    h.receive(
        1,
        &ClientCacheBlobStatus {
            missing: vec![section],
            have: vec![],
        },
    );
    let replies = all::<ClientCacheMissResponse>(&h.tick(1));
    let hashes: Vec<Vec<u64>> = replies
        .iter()
        .map(|r| r.blobs.iter().map(|b| b.hash).collect())
        .collect();
    assert_eq!(hashes, [vec![section, platform], vec![], vec![]]);
    assert!(
        replies[0]
            .blobs
            .iter()
            .all(|blob| xxh64(&blob.payload) == blob.hash)
    );

    let registry = BlockRegistry::vanilla();
    let mut decoded = Chunk::empty(0, 0, Dimension::overworld(registry.air_id()));
    decoded
        .set_sub_chunk_mapped(4, &replies[0].blobs[0].payload, &|hash| {
            registry.runtime_id_from_hash(hash).unwrap()
        })
        .unwrap();
    assert_eq!(
        registry.get(decoded.block(0, 64, 0)).unwrap().name,
        "minecraft:stone"
    );
    decoded.set_biomes(&replies[0].blobs[1].payload);
    assert_eq!(decoded.biome(0, 64, 0), Some(1));
}

#[test]
fn the_cache_stays_off_when_the_client_or_the_server_says_so() {
    for (client, server) in [(false, true), (true, false)] {
        let mut h = Harness::with(
            ShardConfig {
                blob_cache: server,
                ..config()
            },
            platform(),
        );
        h.join_to_radius(1, 2);
        h.receive(1, &ClientCacheStatus { enabled: client });
        let chunks = all::<LevelChunk>(&drain(&mut h, 1));
        assert!(
            chunks.len() == 13
                && chunks
                    .iter()
                    .all(|c| !c.cache_enabled && c.blobs.is_empty())
        );
        h.receive(1, &request(&[(0, 4, 0)]));
        h.receive(
            1,
            &ClientCacheBlobStatus {
                missing: vec![1],
                have: vec![],
            },
        );
        let sent = h.tick(1);
        let answer: Subchunk = sent[0].decode().unwrap();
        assert!(
            !answer.cache_enabled
                && answer.entries[0].blob_id.is_none()
                && answer.entries[0]
                    .payload
                    .as_ref()
                    .is_some_and(|p| !p.is_empty())
        );
        assert_eq!(sent.len(), 1, "no reply to a status nobody should send");
    }
}

#[test]
fn the_view_follows_the_player() {
    let mut h = Harness::new();
    h.join_to_radius(1, 4);
    let first: HashSet<(i32, i32)> = all::<LevelChunk>(&drain(&mut h, 1))
        .iter()
        .map(|c| (c.x, c.z))
        .collect();
    assert_eq!(first.len(), 49);

    // Forty blocks east in steps a player can make (tests/players.rs has the step that is refused).
    (1..=4).for_each(|step| h.receive(1, &input_at([0.5 + 10.0 * step as f32, 65.0, 0.5])));
    let sent = drain(&mut h, 1);
    let publisher: NetworkChunkPublisherUpdate = sent[0].decode().unwrap();
    assert_eq!(
        (
            publisher.coordinates.x,
            publisher.coordinates.y,
            publisher.coordinates.z,
            publisher.radius
        ),
        (40, 65, 0, 64)
    );
    let entered: Vec<(i32, i32)> = all::<LevelChunk>(&sent)
        .iter()
        .map(|c| (c.x, c.z))
        .collect();
    let in_view = |&(x, z): &(i32, i32)| (x - 2).pow(2) + z * z <= 16;
    let expected = (-2..=6)
        .flat_map(|x| (-4..=4).map(move |z| (x, z)))
        .filter(|p| in_view(p) && !first.contains(p))
        .count();
    assert_eq!(entered.len(), expected);
    assert!(entered.iter().all(|p| in_view(p) && !first.contains(p)));
    assert_eq!(entered[0], (4, -1), "nearest first");
    assert_eq!(sent.len(), 1 + expected, "nothing else: no second spawn");

    // Staying in the same chunk sends nothing.
    h.receive(1, &input_at([41.5, 65.0, 3.5]));
    assert!(h.tick(1).is_empty());
}

/// Storage that holds nothing, counts loads, and lets none finish until opened.
#[derive(Clone, Default)]
struct Gated {
    open: Arc<(Mutex<bool>, Condvar)>,
    loads: Arc<AtomicUsize>,
}

impl Gated {
    fn open(&self) {
        *self.open.0.lock().unwrap() = true;
        self.open.1.notify_all();
    }
}

impl WorldStorage for Gated {
    fn load_column(
        &self,
        pos: ChunkPos,
        dimension: i32,
    ) -> Result<Option<StoredColumn>, StorageError> {
        let mut open = self.open.0.lock().unwrap();
        while !*open {
            open = self.open.1.wait(open).unwrap();
        }
        self.loads.fetch_add(1, Ordering::SeqCst);
        MemoryStorage::default().load_column(pos, dimension)
    }

    fn store_column(&self, _: ChunkPos, _: i32, _: &StoredColumn) -> Result<(), StorageError> {
        Ok(())
    }
}

/// Ticks, pausing between, until `conn` was sent `count` columns in total.
fn tick_until_columns(h: &mut Harness, conn: u64, count: usize) -> Vec<RawPacket> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut sent = Vec::new();
    while all::<LevelChunk>(&sent).len() < count {
        assert!(
            Instant::now() < deadline,
            "only {} of {count} columns arrived",
            all::<LevelChunk>(&sent).len()
        );
        sent.extend(h.tick(conn));
        std::thread::sleep(Duration::from_millis(1));
    }
    sent
}

#[test]
fn a_shard_keeps_ticking_while_storage_is_slow() {
    let storage = Gated::default();
    let mut h = Harness::with(config(), World::with_storage(Arc::new(storage.clone())));
    h.join_to_radius(1, 4);
    let started = Instant::now();
    let sent = h.tick(1);
    assert_eq!(
        sent.len(),
        2,
        "the radius and the publisher update, no columns: {:?}",
        common::ids(&sent)
    );
    h.receive(1, &request(&[(0, 4, 0)]));
    // A vanilla client disconnects on ChunkNotFound: the answer waits for the column.
    (0..50).for_each(|_| assert!(h.tick(1).is_empty()));
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "ticks waited on storage"
    );
    assert_eq!(storage.loads.load(Ordering::SeqCst), 0);

    storage.open();
    let sent = tick_until_columns(&mut h, 1, 49);
    let columns: HashSet<(i32, i32)> = all::<LevelChunk>(&sent)
        .iter()
        .map(|c| (c.x, c.z))
        .collect();
    assert_eq!(columns.len(), 49);
    let answers = all::<Subchunk>(&sent);
    assert_eq!(
        (answers.len(), answers[0].entries[0].result),
        (1, SubChunkEntryItemResult::SuccessAllAir)
    );
    // The player spawned once, after every column within three chunks.
    let spawn = sent
        .iter()
        .position(|p| p.is::<PlayStatus>())
        .expect("a spawn");
    let before: HashSet<(i32, i32)> = all::<LevelChunk>(&sent[..spawn])
        .iter()
        .map(|c| (c.x, c.z))
        .collect();
    let near = (-3..=3)
        .flat_map(|x| (-3..=3).map(move |z| (x, z)))
        .filter(|(x, z)| x * x + z * z <= 9);
    assert!(near.clone().count() == 29 && near.into_iter().all(|p| before.contains(&p)));
    assert_eq!(all::<PlayStatus>(&sent).len(), 1);
    assert_eq!(storage.loads.load(Ordering::SeqCst), 49);
}

#[test]
fn columns_stay_loaded_while_seen_and_are_dropped_after() {
    let storage = Gated::default();
    storage.open();
    let mut h = Harness::with(config(), World::with_storage(Arc::new(storage.clone())));
    h.join_to_radius(1, 2);
    tick_until_columns(&mut h, 1, 13);
    (0..250).for_each(|_| drop(h.tick(1)));
    assert_eq!(
        storage.loads.load(Ordering::SeqCst),
        13,
        "nothing is reloaded under a player"
    );

    h.leave(1);
    (0..150).for_each(|_| h.shard.tick());
    h.join_to_radius(2, 2);
    tick_until_columns(&mut h, 2, 13);
    assert_eq!(
        storage.loads.load(Ordering::SeqCst),
        26,
        "the columns were unloaded and read again"
    );
}
