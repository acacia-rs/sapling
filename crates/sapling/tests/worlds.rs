//! Milestone 1 step 5: two bots on a server of two shards. One goes to the world on the other
//! shard and back by command while the other watches it leave its screen and return; in the
//! other world it gets that world's blocks and can still chat and run commands.

mod common;

use std::time::Duration;

use acacia_bot::client::PacketFilter;
use acacia_bot::events::ChatKind;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::CommandOutput;
use acacia_bot::state::{Entity, Trackers};
use acacia_bot::{Bot, BotConfig, BotEvent, Events};
use acacia_world::BlockAccess;
use common::{GROUND, SPAWN};
use sapling::{Command, Handler, Output, Player, Sender, Server, ShardCtx};

const WAIT: Duration = Duration::from_secs(10);

/// Announces who comes and goes, to the worlds it concerns.
struct Gate;

impl Handler for Gate {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        ctx.broadcast_all(&format!("{} joined", player.name));
    }

    fn on_quit(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        ctx.broadcast_all(&format!("{} left from {}", player.name, ctx.world()));
    }

    fn on_world_change(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, from: &str, to: &str) {
        ctx.broadcast_in(from, &format!("{} went to {to}", player.name));
        ctx.broadcast_in(to, &format!("{} came from {from}", player.name));
    }
}

#[derive(Command)]
#[command(name = "world")]
struct WorldCommand {
    name: String,
}

impl Command for WorldCommand {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        match sender.player().is_some_and(|player| ctx.transfer(player.id, &self.name)) {
            true => out.print(format!("to {}", self.name)),
            false => out.error("no way there"),
        }
    }
}

#[derive(Command)]
#[command(name = "where")]
struct Where;

impl Command for Where {
    fn run(self, ctx: &mut ShardCtx<'_>, _: &Sender, out: &mut Output) {
        out.print(format!("{} of {:?}, {} here", ctx.world(), ctx.worlds(), ctx.player_count()));
    }
}

async fn both(a: &mut Bot, b: &mut Bot, ticks: u32) {
    let (a, b) = tokio::join!(a.wait_ticks(ticks), b.wait_ticks(ticks));
    a.and(b).expect("both stay connected");
}

/// The next event `wanted` picks; earlier ones are dropped.
async fn event<T>(bot: &mut Bot, wanted: impl Fn(BotEvent) -> Option<T>) -> T {
    let found = tokio::time::timeout(WAIT, async {
        loop {
            match bot.next().await {
                Some(BotEvent::Disconnected(reason)) => panic!("disconnected: {reason:?}"),
                Some(event) => {
                    if let Some(found) = wanted(event) {
                        return found;
                    }
                }
                None => panic!("the connection is gone"),
            }
        }
    });
    found.await.expect("the event arrives in time")
}

/// The next chat line of `kind` the bot is shown.
async fn hears(bot: &mut Bot, kind: ChatKind) -> String {
    event(bot, |e| if let BotEvent::Chat(line) = e { (line.kind == kind).then_some(line.message) } else { None }).await
}

/// Runs `line` and returns the (message, success) of its answer.
async fn run(bot: &mut Bot, line: &str) -> (String, bool) {
    bot.client().command(line);
    let output: CommandOutput = event(bot, |e| if let BotEvent::Packet(raw) = e { (raw.id == CommandOutput::ID).then(|| raw.decode().expect("decodes")) } else { None }).await;
    (output.output[0].message_id.clone(), output.output[0].success)
}

fn seen<'a>(watcher: &'a Bot, name: &str) -> Option<&'a Entity> {
    watcher.state().entities.players().find(|e| e.username.as_deref() == Some(name))
}

/// The block a bot's own copy of the world has at `pos`.
fn block(bot: &Bot, [x, y, z]: [i32; 3]) -> &'static str {
    let world = bot.world().expect("a physics bot tracks the world");
    let (view, registry) = (world.view().expect("chunks"), world.registry().expect("a registry"));
    registry.get(view.block(x, y, z)).expect("a known block").name
}

fn column(bot: &Bot) -> [i32; 3] {
    bot.movement().and_then(|m| m.position()).expect("movement started").map(|c| c.floor() as i32)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bot_goes_to_a_world_on_another_shard_and_back() {
    let builder = Server::builder().default_world_name("hub").add_platform_on(1, "arena").handler_each(|| Gate).command::<WorldCommand>().command::<Where>();
    let server = common::start(builder).await;
    assert_eq!(server.shard_count(), 2);
    let travelling = BotConfig { subscribe: PacketFilter::none().with(CommandOutput::ID), events: Events::CHAT, ..BotConfig::default() };
    let mut alice = common::spawn(common::client(&server, "Alice"), travelling).await;
    let watching = BotConfig { trackers: Trackers { entities: true, ..Trackers::default() }, events: Events::CHAT | Events::PLAYERS, ..BotConfig::default() };
    let mut bob = common::spawn(common::client(&server, "Bob"), watching).await;
    both(&mut alice, &mut bob, 40).await;
    let id = alice.client().runtime_entity_id();
    assert_eq!((seen(&bob, "Alice").map(|e| e.runtime_id), hears(&mut bob, ChatKind::Raw).await.as_str()), (Some(id), "Bob joined"));
    assert_eq!(run(&mut alice, "where").await, (r#"hub of ["arena", "hub"], 2 here"#.to_owned(), true));

    // To the arena: a handoff to the other shard's thread. Each world hears of it; Bob loses the entity.
    assert_eq!(run(&mut alice, "world arena").await, ("to arena".to_owned(), true));
    assert_eq!(hears(&mut bob, ChatKind::Raw).await, "Alice went to arena");
    assert_eq!(hears(&mut alice, ChatKind::Raw).await, "Alice came from hub");
    both(&mut alice, &mut bob, 40).await;
    assert!(seen(&bob, "Alice").is_none() && bob.state().player_list.by_name("Alice").is_some(), "gone from the world, not from the server");
    // She stands on the arena's platform, which her client was sent, and followed one teleport.
    assert_eq!((column(&alice), block(&alice, [0, 64, 0]), block(&alice, [0, 65, 0])), ([0, 65, 0], "minecraft:stone", "minecraft:air"));
    assert_eq!(alice.movement().expect("physics").teleports, 1);

    // The other shard hears her: a command, and chat that stays in her world.
    assert_eq!(run(&mut alice, "where").await, (r#"arena of ["arena", "hub"], 1 here"#.to_owned(), true));
    assert_eq!(run(&mut alice, "world nowhere").await, ("no way there".to_owned(), false));
    alice.client().chat("anyone in the arena?");
    assert_eq!(hears(&mut alice, ChatKind::Chat).await, "anyone in the arena?");
    both(&mut alice, &mut bob, 10).await;
    bob.client().chat("only the hub hears this");
    assert_eq!(hears(&mut bob, ChatKind::Chat).await, "only the hub hears this");

    // And back: the hub's terrain again, and each other.
    assert_eq!(run(&mut alice, "world hub").await, ("to hub".to_owned(), true));
    assert_eq!(hears(&mut bob, ChatKind::Raw).await, "Alice came from arena");
    both(&mut alice, &mut bob, 40).await;
    assert_eq!(seen(&bob, "Alice").map(|e| e.runtime_id), Some(id), "the same entity id on every shard");
    assert_eq!(column(&alice), [SPAWN.0, GROUND + 1, SPAWN.1]);
    assert_ne!(block(&alice, [SPAWN.0, GROUND, SPAWN.1]), "minecraft:air");
    assert_eq!((alice.movement().expect("physics").teleports, alice.disconnect_reason(), bob.disconnect_reason()), (2, None, None));

    // She leaves from the arena: the hub's shard unlists her and hears where she left.
    assert_eq!(run(&mut alice, "world arena").await, ("to arena".to_owned(), true));
    both(&mut alice, &mut bob, 40).await;
    alice.disconnect().await;
    let left = event(&mut bob, |e| if let BotEvent::PlayerLeft(p) = e { Some(p.username) } else { None }).await;
    assert_eq!(left, "Alice");
    let mut last = hears(&mut bob, ChatKind::Raw).await;
    while last != "Alice left from arena" {
        last = hears(&mut bob, ChatKind::Raw).await;
    }
    assert_eq!(bob.state().player_list.len(), 1);
    bob.disconnect().await;
    server.shutdown().await;
}
