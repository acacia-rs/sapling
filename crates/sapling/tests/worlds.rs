//! Milestone 1 step 5: two bots on a server of two shards. One goes to the world on the other
//! shard and back by command while the other watches it leave its screen and return; in the
//! other world it gets that world's blocks and can still chat and run commands.

mod common;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use acacia_bot::client::PacketFilter;
use acacia_bot::events::ChatKind;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::{CommandOutput, LevelChunk};
use acacia_bot::state::{Entity, Trackers};
use acacia_bot::{Bot, BotConfig, BotEvent, Events};
use acacia_world::BlockAccess;
use common::{GROUND, SPAWN};
use sapling::{Command, Handler, Output, Player, Sender, Server, ShardCtx};

const WAIT: Duration = Duration::from_secs(10);
/// The widest view a shard gives (`ShardConfig::max_view_radius`), in chunks.
const MAX_VIEW: i32 = 8;
/// The chunks of the hub's and the arena's spawn.
const HUB_CHUNK: (i32, i32) = (SPAWN.0 >> 4, SPAWN.1 >> 4);
const ARENA_CHUNK: (i32, i32) = (0, 0);

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

/// A column as a bot was sent it: its position and whether it has no sections.
type Column = (i32, i32, bool);

/// Every LevelChunk the travelling bot was sent since [`columns`] last took them.
static COLUMNS: Mutex<Vec<Column>> = Mutex::new(Vec::new());

fn note(event: &BotEvent) {
    if let BotEvent::Packet(raw) = event {
        if raw.id == LevelChunk::ID {
            let chunk: LevelChunk = raw.decode().expect("decodes");
            COLUMNS.lock().unwrap().push((chunk.x, chunk.z, chunk.highest_subchunk_count == Some(0)));
        }
    }
}

/// Takes in what the bot has waiting and returns the columns it was sent since the last call.
async fn columns(bot: &mut Bot) -> Vec<Column> {
    while let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(50), bot.next()).await {
        note(&event);
    }
    std::mem::take(&mut *COLUMNS.lock().unwrap())
}

fn distance_sq(x: i32, z: i32, centre: (i32, i32)) -> i32 {
    (x - centre.0).pow(2) + (z - centre.1).pow(2)
}

/// The square of the bot's view radius: how far from the hub's spawn the columns it got reach.
fn view_radius_sq(joined: &[Column]) -> i32 {
    let furthest = joined.iter().map(|&(x, z, _)| distance_sq(x, z, HUB_CHUNK)).max().expect("columns");
    assert!((1..=MAX_VIEW * MAX_VIEW).contains(&furthest), "a view of radius² {furthest}");
    furthest
}

/// Checks a move to the world whose spawn is in chunk `to` against the columns the client
/// held: each was sent again, empty if the new view does not reach it, and nothing with
/// sections was sent for such a place.
fn assert_swapped(held: &[Column], after: &[Column], radius_sq: i32, to: (i32, i32)) {
    let far = |x: i32, z: i32| distance_sq(x, z, to) > radius_sq;
    let last: HashMap<(i32, i32), bool> = after.iter().map(|&(x, z, empty)| ((x, z), empty)).collect();
    for &(x, z, _) in held {
        let empty = last.get(&(x, z)).unwrap_or_else(|| panic!("({x}, {z}) was held and not sent again"));
        assert!(*empty || !far(x, z), "({x}, {z}) is outside the new view and still has sections");
    }
    assert!(after.iter().all(|&(x, z, empty)| empty || !far(x, z)), "a column of the old world went out after the move");
    let emptied = held.iter().filter(|(x, z, _)| far(*x, *z)).count();
    assert!(emptied > 0 && emptied < held.len(), "the two views overlap in part: {emptied} of {} outside", held.len());
}

/// The next event `wanted` picks; earlier ones are dropped.
async fn event<T>(bot: &mut Bot, wanted: impl Fn(BotEvent) -> Option<T>) -> T {
    let found = tokio::time::timeout(WAIT, async {
        loop {
            match bot.next().await {
                Some(BotEvent::Disconnected(reason)) => panic!("disconnected: {reason:?}"),
                Some(event) => {
                    note(&event);
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

/// The block a bot stands in.
// TODO: a bot teleported into a world whose columns are still on their way falls for a tick
// (0.0784) before the floor arrives and then stands that far inside it; BDS would correct it,
// sapling does not check movement (docs/DESIGN.md "Open items"). Until then a tenth is forgiven.
fn column(bot: &Bot) -> [i32; 3] {
    bot.movement().and_then(|m| m.position()).expect("movement started").map(|c| (c + 0.1).floor() as i32)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bot_goes_to_a_world_on_another_shard_and_back() {
    let builder = Server::builder().default_world_name("hub").add_platform_on(1, "arena").handler_each(|| Gate).command::<WorldCommand>().command::<Where>();
    let server = common::start(builder).await;
    assert_eq!(server.shard_count(), 2);
    let travelling = BotConfig { subscribe: PacketFilter::none().with(CommandOutput::ID).with(LevelChunk::ID), events: Events::CHAT, ..BotConfig::default() };
    // A view wide enough for the hub's and the arena's to share columns, though not all.
    let mut alice = common::spawn(common::client(&server, "Alice").chunk_radius(MAX_VIEW), travelling).await;
    let watching = BotConfig { trackers: Trackers { entities: true, ..Trackers::default() }, events: Events::CHAT | Events::PLAYERS, ..BotConfig::default() };
    let mut bob = common::spawn(common::client(&server, "Bob"), watching).await;
    both(&mut alice, &mut bob, 40).await;
    let id = alice.client().runtime_entity_id();
    assert_eq!((seen(&bob, "Alice").map(|e| e.runtime_id), hears(&mut bob, ChatKind::Raw).await.as_str()), (Some(id), "Bob joined"));
    assert_eq!(run(&mut alice, "where").await, (r#"hub of ["arena", "hub"], 2 here"#.to_owned(), true));

    // To the arena: a handoff to the other shard's thread. Each world hears of it; Bob loses the entity.
    let held = columns(&mut alice).await;
    assert_eq!(run(&mut alice, "world arena").await, ("to arena".to_owned(), true));
    assert_eq!(hears(&mut bob, ChatKind::Raw).await, "Alice went to arena");
    assert_eq!(hears(&mut alice, ChatKind::Raw).await, "Alice came from hub");
    both(&mut alice, &mut bob, 40).await;
    // Nothing of the hub stays on her screen: what the arena's view does not cover was emptied.
    let (radius_sq, mut held) = (view_radius_sq(&held), (held, columns(&mut alice).await));
    assert_swapped(&held.0, &held.1, radius_sq, ARENA_CHUNK);
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
    held.1.extend(columns(&mut alice).await);
    assert_eq!(run(&mut alice, "world hub").await, ("to hub".to_owned(), true));
    assert_eq!(hears(&mut bob, ChatKind::Raw).await, "Alice came from arena");
    both(&mut alice, &mut bob, 40).await;
    assert_swapped(&held.1, &columns(&mut alice).await, radius_sq, HUB_CHUNK);
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
