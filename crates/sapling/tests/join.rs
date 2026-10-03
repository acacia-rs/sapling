//! Milestone 1 step 1: acacia's client and bot join a sapling server over loopback RakNet.
//! The bot runs physics on the terrain it decoded, so standing still proves the chunks arrived.

use std::time::Duration;

use acacia_bot::{Bot, BotConfig};
use acacia_client::{Client, ClientBuilder, DisconnectReason, Event, TransportKind};
use acacia_world::BlockAccess;
use sapling::{Handler, Player, Server, ShardCtx};
use tokio::sync::mpsc;

const WAIT: Duration = Duration::from_secs(10);

struct Report(mpsc::UnboundedSender<String>);

impl Handler for Report {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        let _ = self.0.send(format!("join {} ({} online)", player.name, ctx.player_count()));
    }

    fn on_quit(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        let _ = self.0.send(format!("quit {} ({} online)", player.name, ctx.player_count()));
    }
}

async fn start() -> (Server, mpsc::UnboundedReceiver<String>) {
    let (reports, events) = mpsc::unbounded_channel();
    let builder = Server::builder().bind("127.0.0.1:0".parse().unwrap()).online_mode(false).handler(Report(reports));
    (builder.start().await.expect("the server starts"), events)
}

fn client(server: &Server, name: &str) -> ClientBuilder {
    Client::builder(server.local_addr().to_string()).offline(name).transport(TransportKind::RakNet)
}

async fn next(events: &mut mpsc::UnboundedReceiver<String>) -> String {
    tokio::time::timeout(WAIT, events.recv()).await.expect("a handler call in time").expect("the server is running")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_client_spawns_stays_and_leaves() {
    let (server, mut events) = start().await;
    let mut client = client(&server, "Tester").connect().await.expect("the client spawns");
    assert_eq!(next(&mut events).await, "join Tester (1 online)");

    let stay = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match client.recv().await {
                Some(Event::Packet(_)) => {}
                ended => return ended,
            }
        }
    });
    let ended = stay.await;
    assert!(ended.is_err(), "the connection ended while idle: {ended:?}");

    client.close();
    let closed = tokio::time::timeout(WAIT, async {
        loop {
            match client.recv().await {
                Some(Event::Packet(_)) => {}
                Some(Event::Disconnected(reason)) => return Some(reason),
                None => return None,
            }
        }
    });
    assert_eq!(closed.await.expect("the client closes"), Some(DisconnectReason::LocalClose));
    assert_eq!(next(&mut events).await, "quit Tester (0 online)");
    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_physics_bot_stands_on_the_platform() {
    let (server, mut events) = start().await;
    let config = BotConfig { physics: true, ..BotConfig::default() };
    let mut bot = Bot::connect(client(&server, "Walker"), config).await.expect("the bot spawns");

    // Movement starts once the bot has decoded ground under its feet (well before its 5 s fallback).
    let started = tokio::time::timeout(Duration::from_secs(3), async {
        while !bot.movement().is_some_and(|m| m.is_started()) {
            bot.wait_ticks(1).await.expect("connected");
        }
    });
    started.await.expect("terrain under the spawn arrives");
    assert_eq!(next(&mut events).await, "join Walker (1 online)");

    let world = bot.world().expect("physics bots track terrain");
    let (view, registry) = (world.view().expect("a chunk view"), world.registry().expect("a block registry"));
    let name = |x, y, z| registry.get(view.block(x, y, z)).map(|s| s.name);
    assert_eq!((name(0, 64, 0), name(-8, 64, 7), name(0, 65, 0), name(8, 64, 0)), (Some("minecraft:stone"), Some("minecraft:stone"), Some("minecraft:air"), Some("minecraft:air")));

    bot.wait_ticks(80).await.expect("connected for 4 more seconds");
    let movement = bot.movement().expect("physics enabled");
    let feet = movement.position().expect("movement started");
    assert!(movement.on_ground(), "the bot is falling: {feet:?}");
    assert!((feet[0] - 0.5).abs() < 0.01 && (feet[1] - 65.0).abs() < 0.01 && (feet[2] - 0.5).abs() < 0.01, "the bot moved to {feet:?}");
    assert_eq!((movement.corrections, bot.disconnect_reason()), (0, None));

    bot.disconnect().await;
    assert!(matches!(bot.disconnect_reason(), Some(DisconnectReason::LocalClose)));
    assert_eq!(next(&mut events).await, "quit Walker (0 online)");
    server.shutdown().await;
}
