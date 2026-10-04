//! Milestone 1 step 2: a physics bot joins a server whose world is a vanilla world folder (the
//! sapling-leveldb fixture), stands on the terrain it was sent, checks that terrain against the
//! save, and walks across it. Run with the client blob cache on and off.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use acacia_bot::pathfind::{Goal, GotoOpts, NavStatus, Navigator};
use acacia_bot::{Bot, BotConfig};
use acacia_client::{BlobStore, DisconnectReason, MemoryBlobStore};
use acacia_world::{BlockAccess, BlockRegistry, Chunk};
use bytes::Bytes;
use common::{GOAL, GROUND, SPAWN, fixture};
use sapling::Server;
use sapling_leveldb::{ChunkKey, LevelDbWorld};

async fn start() -> Server {
    common::start(Server::builder()).await
}

/// A blob store that counts what the server had to send.
struct Counting {
    inner: MemoryBlobStore,
    delivered: AtomicUsize,
}

impl BlobStore for Counting {
    fn has(&self, id: u64) -> bool {
        self.inner.has(id)
    }

    fn get(&self, id: u64) -> Option<Bytes> {
        self.inner.get(id)
    }

    fn insert(&self, id: u64, blob: &Bytes) {
        self.delivered.fetch_add(1, Ordering::SeqCst);
        self.inner.insert(id, blob);
    }

    fn keeps_payloads(&self) -> bool {
        true
    }
}

async fn join(server: &Server, name: &str, store: Option<Arc<Counting>>) -> Bot {
    let client = common::client(server, name);
    let client = match store {
        Some(store) => client.blob_store(store),
        None => client.without_blob_cache(),
    };
    common::spawn(client, BotConfig::default()).await
}

/// The saved chunks around the spawn chunk, straight from the fixture.
fn saved_chunks() -> Vec<Chunk> {
    let world = LevelDbWorld::open_in_memory(&fixture()).unwrap();
    let (cx, cz) = (SPAWN.0 >> 4, SPAWN.1 >> 4);
    let around = (-1..=1).flat_map(|dx| (-1..=1).map(move |dz| ChunkKey::new(cx + dx, cz + dz, 0)));
    around.map(|key| world.load_column(key).unwrap().expect("a fixture column").chunk).collect()
}

/// Blocks where the bot's decoded terrain differs from the save, bottom to top of the dimension.
fn differences(bot: &Bot, saved: &[Chunk]) -> usize {
    let view = bot.world().and_then(|w| w.view()).expect("a chunk view");
    let mut differences = 0;
    for chunk in saved {
        for (x, z) in (0..16).flat_map(|x| (0..16).map(move |z| (chunk.x * 16 + x, chunk.z * 16 + z))) {
            differences += (-64..320).filter(|&y| view.block(x, y, z) != chunk.block(x, y, z) || view.liquid(x, y, z) != chunk.liquid(x, y, z)).count();
        }
    }
    differences
}

/// Joins, compares the terrain with the save, and stands still on it.
async fn arrive(server: &Server, name: &str, store: Option<Arc<Counting>>) -> Bot {
    let mut bot = join(server, name, store).await;
    let saved = saved_chunks();
    let matched = tokio::time::timeout(Duration::from_secs(10), async {
        while differences(&bot, &saved) > 0 {
            bot.wait_ticks(2).await.expect("connected");
        }
    });
    assert!(matched.await.is_ok(), "{} blocks still differ from the save", differences(&bot, &saved));

    let registry = BlockRegistry::vanilla();
    let view = bot.world().and_then(|w| w.view()).expect("a chunk view");
    let under = registry.get(view.block(SPAWN.0, GROUND, SPAWN.1)).expect("a known block");
    assert!(under.is_solid() && under.name == "minecraft:grass_block", "under the spawn: {}", under.name);
    assert_eq!(registry.get(view.block(SPAWN.0, -64, SPAWN.1)).unwrap().name, "minecraft:bedrock");

    bot.wait_ticks(40).await.expect("connected");
    let movement = bot.movement().expect("physics enabled");
    let feet = movement.position().expect("movement started");
    assert!(movement.on_ground(), "the bot is falling: {feet:?}");
    assert!((feet[1] - (GROUND as f32 + 1.0)).abs() < 0.01, "the bot stands at {feet:?}");
    assert_eq!((movement.corrections, bot.disconnect_reason()), (0, None));
    bot
}

/// Walks to [`GOAL`], one chunk east and south, and checks the view moved along.
async fn walk(bot: &mut Bot) {
    let (cx, cz) = (SPAWN.0 >> 4, SPAWN.1 >> 4);
    let has = |bot: &Bot, x: i32, z: i32| bot.world().and_then(|w| w.view()).is_some_and(|v| v.chunk(x, z).is_some());
    let radius = (1..40).find(|r| !has(bot, cx + r, cz)).expect("the view ends") - 1;
    assert!(radius >= 2, "a view of {radius} chunks");
    assert!(!has(bot, cx + radius + 1, cz + 1));

    let mut nav = Navigator::new(Goal::XZ(GOAL.0, GOAL.1), GotoOpts::default());
    let arrived = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            match nav.tick(bot).await {
                Ok(NavStatus::Arrived) => return Ok(()),
                Ok(_) => {}
                Err(error) => return Err(error.to_string()),
            }
        }
    });
    assert_eq!(arrived.await.expect("the walk ends in time"), Ok(()));
    bot.wait_ticks(20).await.expect("connected");
    let [x, _, z] = bot.block_position().expect("movement started");
    assert_eq!((x, z, bot.disconnect_reason()), (GOAL.0, GOAL.1, None));
    assert!(bot.movement().is_some_and(|m| m.on_ground() && m.corrections == 0));
    let row: Vec<i32> = (-12..14).filter(|&x| has(bot, x, cz + 1)).collect();
    assert!(has(bot, cx + radius + 1, cz + 1), "the column that came into view was sent: radius {radius}, row {row:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bot_walks_the_map_without_the_blob_cache() {
    let server = start().await;
    let mut bot = arrive(&server, "Plain", None).await;
    walk(&mut bot).await;
    bot.disconnect().await;
    assert!(matches!(bot.disconnect_reason(), Some(DisconnectReason::LocalClose)));
    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bot_walks_the_map_through_the_blob_cache() {
    let server = start().await;
    let store = Arc::new(Counting { inner: MemoryBlobStore::with_payloads(), delivered: AtomicUsize::new(0) });
    let mut bot = arrive(&server, "Cached", Some(store.clone())).await;
    walk(&mut bot).await;
    bot.disconnect().await;
    let delivered = store.delivered.load(Ordering::SeqCst);
    // 9 saved columns of at least 9 sections each were compared, all through blobs.
    assert!(delivered > 81, "only {delivered} blobs were delivered");

    // A second join with the same store already holds every blob around the spawn.
    let mut again = arrive(&server, "Again", Some(store.clone())).await;
    assert_eq!(store.delivered.load(Ordering::SeqCst), delivered, "blobs the client holds were sent again");
    again.disconnect().await;
    server.shutdown().await;
}
