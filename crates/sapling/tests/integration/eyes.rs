//! A bot throws an eye of ender and it flies toward the stronghold its world's generator locates
//! (research/captures/bds-eye-throw.txt).

use crate::common;
#[path = "common/travel.rs"]
mod travel;

use std::sync::Arc;

use acacia_bot::client::PacketFilter;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::{AddEntity, CommandOutput, MoveEntityDelta};
use acacia_bot::{BotConfig, BotEvent};
use acacia_world::{BlockRegistry, Chunk, Dimension};
use sapling::{ChunkPos, Generated, MemoryStorage, World, WorldStorage};
use sapling_world::{StorageError, StoredColumn};
use travel::{Operators, WAIT, run};

const SEED: i64 = 20261018;
const FEET: [f32; 3] = [0.5, 64.0, 0.5];

/// Columns kept in memory, and the strongholds of `SEED`'s generator.
struct Strongholds {
    columns: MemoryStorage,
    generator: Arc<Generated<MemoryStorage>>,
}

impl WorldStorage for Strongholds {
    fn load_column(&self, pos: ChunkPos, dimension: i32) -> Result<Option<StoredColumn>, StorageError> {
        self.columns.load_column(pos, dimension)
    }
    fn store_column(&self, pos: ChunkPos, dimension: i32, column: &StoredColumn) -> Result<(), StorageError> {
        self.columns.store_column(pos, dimension, column)
    }
    fn locate_stronghold(&self, x: i32, z: i32) -> Option<[i32; 3]> {
        self.generator.locate_stronghold(x, z)
    }
}

/// Stone from y 48 to 63 in the four columns round the origin, over the void.
fn world(generator: Arc<Generated<MemoryStorage>>) -> World {
    let columns = MemoryStorage::default();
    let registry = BlockRegistry::vanilla();
    let stone = registry.find("minecraft:stone", "").unwrap();
    for (x, z) in [(-1, -1), (-1, 0), (0, -1), (0, 0)] {
        let mut chunk = Chunk::empty(x, z, Dimension::from_id(0, registry.air_id()));
        chunk.fill_section(3, stone);
        columns.insert(0, chunk);
    }
    World::with_storage(Arc::new(Strongholds { columns, generator }))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_thrown_eye_flies_toward_the_located_stronghold() {
    let generator = Arc::new(Generated::with_areas(MemoryStorage::default(), SEED, 1));
    let builder = crate::common::builder().bind("127.0.0.1:0".parse().unwrap()).online_mode(false).world(world(generator.clone()), FEET).handler(Operators).vanilla_commands();
    let server = builder.start().await.expect("the server starts");
    let subscribed = PacketFilter::none().with(AddEntity::ID).with(MoveEntityDelta::ID).with(CommandOutput::ID);
    let mut alice = common::spawn(common::client(&server, "Alice"), BotConfig { subscribe: subscribed, ..BotConfig::default() }).await;
    assert!(run(&mut alice, "replaceitem entity @s slot.weapon.mainhand 0 ender_eye 2").await.1);
    alice.wait_ticks(5).await.expect("connected");
    alice.use_item();

    let eye = tokio::time::timeout(WAIT, async {
        loop {
            if let Some(BotEvent::Packet(raw)) = alice.next().await
                && raw.id == AddEntity::ID
                && let Some(eye) = raw.decode::<AddEntity>().ok().filter(|added| added.entity_type == "minecraft:eye_of_ender_signal")
            {
                return eye;
            }
        }
    });
    let eye = eye.await.expect("an eye thrown");
    let mut moves = Vec::new();
    let flight = tokio::time::timeout(WAIT, async {
        while moves.len() < 20 {
            if let Some(BotEvent::Packet(raw)) = alice.next().await
                && raw.id == MoveEntityDelta::ID
                && let Ok(moved) = raw.decode::<MoveEntityDelta>()
                && moved.runtime_entity_id == eye.runtime_id
            {
                moves.push([moved.x, moved.z]);
            }
        }
    });
    flight.await.expect("the eye flies");
    let from = [eye.position.x, eye.position.z];
    assert_eq!(from, [FEET[0] + 0.5, FEET[2] + 0.5], "half a block past the eyes");
    let [sx, _, sz] = generator.locate_stronghold(FEET[0].floor() as i32, FEET[2].floor() as i32).expect("seed 20261018 has strongholds");
    let [x, z] = moves.last().unwrap().map(|c| c.expect("moved on both axes"));
    let (flown, toward) = ([x - from[0], z - from[1]], [sx as f32 - from[0], sz as f32 - from[1]]);
    let cosine = (flown[0] * toward[0] + flown[1] * toward[1]) / (flown[0].hypot(flown[1]) * toward[0].hypot(toward[1]));
    assert!(cosine > 0.9999, "flew {flown:?}, the stronghold is {toward:?} off");
    assert_eq!(alice.disconnect_reason(), None);
    alice.disconnect().await;
    server.shutdown().await;
}
