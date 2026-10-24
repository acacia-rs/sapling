//! A sapling server on the sapling-leveldb fixture world, and physics bots that join it.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use acacia_bot::proto::Packet;
use acacia_bot::{Bot, BotConfig, BotEvent};
use acacia_client::{Client, ClientBuilder, TransportKind};
use sapling::{Server, ServerBuilder, World};

/// The fixture's spawn block column and the y of the grass under it.
pub const SPAWN: (i32, i32) = (21, 205);
pub const GROUND: i32 = 71;
/// A walk BDS let the same bot make on this terrain.
pub const GOAL: (i32, i32) = (45, 213);

pub fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../sapling-leveldb/tests/fixtures/world")
}

/// Starts `builder` offline on a free port with the fixture as its world.
pub async fn start(builder: ServerBuilder) -> Server {
    let opened = World::open_folder_in_memory(&fixture()).expect("the fixture opens");
    assert_eq!(opened.spawn, [SPAWN.0 as f32 + 0.5, GROUND as f32 + 1.0, SPAWN.1 as f32 + 0.5]);
    builder.bind("127.0.0.1:0".parse().unwrap()).online_mode(false).world(opened.world, opened.spawn).start().await.expect("the server starts")
}

pub fn client(server: &Server, name: &str) -> ClientBuilder {
    Client::builder(server.local_addr().to_string()).offline(name).transport(TransportKind::RakNet)
}

/// The next `T` the bot is sent; other packets are dropped.
pub async fn packet<T: Packet>(bot: &mut Bot) -> T {
    let found = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match bot.next().await {
                Some(BotEvent::Packet(packet)) if packet.id == T::ID => return packet.decode::<T>().expect("the bot decodes what sapling sent"),
                Some(BotEvent::Disconnected(reason)) => panic!("disconnected: {reason:?}"),
                Some(_) => {}
                None => panic!("the connection is gone"),
            }
        }
    });
    found.await.unwrap_or_else(|_| panic!("no {} in time", T::NAME))
}

/// Connects a physics bot and waits until it stands on terrain it decoded.
pub async fn spawn(client: ClientBuilder, config: BotConfig) -> Bot {
    let mut bot = Bot::connect(client, BotConfig { physics: true, ..config }).await.expect("the bot spawns");
    let started = tokio::time::timeout(Duration::from_secs(5), async {
        while !bot.movement().is_some_and(|m| m.is_started()) {
            bot.wait_ticks(1).await.expect("connected");
        }
    });
    started.await.expect("terrain under the spawn arrives");
    bot
}
