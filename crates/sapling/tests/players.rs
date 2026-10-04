//! Milestone 1 step 3: two physics bots on the fixture world. One watches with acacia-bot's entity
//! tracker while the other turns, walks, sneaks, chats and leaves.

mod common;

use std::cell::Cell;
use std::time::Duration;

use acacia_bot::events::ChatKind;
use acacia_bot::pathfind::{Goal, GotoOpts, NavStatus, Navigator};
use acacia_bot::proto::types::MetadataFlags1;
use acacia_bot::state::{Entity, Trackers};
use acacia_bot::text::strip_formatting;
use acacia_bot::{Bot, BotConfig, BotEvent, Events};
use common::{GOAL, GROUND, SPAWN};
use sapling::{ChatOutcome, Handler, Player, Server, ShardCtx};

const WAIT: Duration = Duration::from_secs(10);

struct Hub;

impl Handler for Hub {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        ctx.broadcast(&format!("{} joined", player.name));
    }

    fn on_chat(&mut self, _: &mut ShardCtx<'_>, _: &Player, message: &str) -> ChatOutcome {
        if message == "shh" { ChatOutcome::Cancel } else { ChatOutcome::Allow }
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

fn seen<'a>(watcher: &'a Bot, name: &str) -> Option<&'a Entity> {
    watcher.state().entities.players().find(|e| e.username.as_deref() == Some(name))
}

fn feet(bot: &Bot) -> [f32; 3] {
    bot.movement().and_then(|m| m.position()).expect("movement started")
}

fn distance(entity: &Entity, feet: [f32; 3]) -> f32 {
    let at = entity.feet();
    [at.x - feet[0], at.y - feet[1], at.z - feet[2]].iter().map(|d| d * d).sum::<f32>().sqrt()
}

/// Walks `walker` to [`GOAL`]; returns where `watcher` saw it, tick by tick.
async fn walk_watched(walker: &mut Bot, watcher: &mut Bot) -> Vec<[f32; 2]> {
    let name = walker.client().display_name().to_owned();
    let mut nav = Navigator::new(Goal::XZ(GOAL.0, GOAL.1), GotoOpts::default());
    let arrived = Cell::new(false);
    let walk = async {
        let result = loop {
            match nav.tick(walker).await {
                Ok(NavStatus::Arrived) => break Ok(()),
                Ok(_) => {}
                Err(error) => break Err(error.to_string()),
            }
        };
        arrived.set(true);
        result
    };
    let watch = async {
        let mut track = Vec::new();
        while !arrived.get() {
            watcher.wait_ticks(1).await.expect("connected");
            track.extend(seen(watcher, &name).map(|e| [e.position.x, e.position.z]));
        }
        track
    };
    let (result, track) = tokio::time::timeout(Duration::from_secs(30), async { tokio::join!(walk, watch) }).await.expect("the walk ends in time");
    assert_eq!(result, Ok(()));
    track
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_bots_see_each_other_move_and_chat() {
    let server = common::start(Server::builder().handler(Hub)).await;
    let mut alice = common::spawn(common::client(&server, "Alice"), BotConfig::default()).await;
    let watching = BotConfig { trackers: Trackers { entities: true, ..Trackers::default() }, events: Events::CHAT | Events::PLAYERS, ..BotConfig::default() };
    let mut bob = common::spawn(common::client(&server, "Bob"), watching).await;
    both(&mut alice, &mut bob, 40).await;

    // Listed and announced: the list first, then what the hub's handler broadcast.
    let listed = event(&mut bob, |e| if let BotEvent::PlayerJoined(p) = e { Some(p.username) } else { None }).await;
    assert_eq!(listed, "Alice");
    let announced = event(&mut bob, |e| if let BotEvent::Chat(m) = e { Some((m.kind, m.message)) } else { None }).await;
    assert_eq!(announced, (ChatKind::Raw, "Bob joined".to_owned()));
    for bot in [&alice, &bob] {
        let list = &bot.state().player_list;
        assert!(list.len() == 2 && list.by_name("Alice").is_some() && list.by_name("Bob").is_some());
    }

    // AddPlayer placed Alice where she stands: on the spawn block (its position is the feet).
    let spawn = [SPAWN.0 as f32 + 0.5, GROUND as f32 + 1.0, SPAWN.1 as f32 + 0.5];
    let added = seen(&bob, "Alice").expect("Bob has Alice as an entity");
    assert!((added.position.x - spawn[0]).abs() < 0.01 && (added.position.z - spawn[2]).abs() < 0.01, "{:?}", added.position);
    assert_eq!((added.runtime_id, bob.state().entities.players().count()), (alice.client().runtime_entity_id(), 1));

    // A turn is a move: from then on Bob has her eye position, as MovePlayer carries it.
    alice.controls().expect("physics").yaw = 90.0;
    both(&mut alice, &mut bob, 10).await;
    let turned = seen(&bob, "Alice").unwrap();
    assert!(distance(turned, feet(&alice)) < 0.01 && (turned.yaw - 90.0).abs() < 0.01 && turned.on_ground, "{turned:?} vs {:?}", feet(&alice));

    let track = walk_watched(&mut alice, &mut bob).await;
    both(&mut alice, &mut bob, 20).await;
    let arrived = seen(&bob, "Alice").unwrap();
    assert!(distance(arrived, feet(&alice)) < 0.01, "{:?} vs {:?}", arrived.feet(), feet(&alice));
    assert_eq!((feet(&alice)[0].floor() as i32, feet(&alice)[2].floor() as i32), GOAL);
    // Bob saw the walk as a walk: many positions, none more than a sprint-jump tick apart.
    let steps: Vec<f32> = track.windows(2).map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt()).collect();
    let moving = steps.iter().filter(|s| **s > 0.0).count();
    assert!(moving > 40 && steps.iter().all(|s| *s < 1.5), "{moving} moving ticks, longest step {:?}", steps.iter().cloned().fold(0.0, f32::max));

    alice.controls().expect("physics").sneak = true;
    both(&mut alice, &mut bob, 10).await;
    assert!(seen(&bob, "Alice").unwrap().metadata.flags().contains(MetadataFlags1::SNEAKING));
    alice.controls().expect("physics").sneak = false;
    both(&mut alice, &mut bob, 10).await;
    assert!(!seen(&bob, "Alice").unwrap().metadata.flags().contains(MetadataFlags1::SNEAKING));

    // Chat under the verified name; a cancelled line never arrives.
    alice.client().chat("shh");
    alice.client().chat("hello §cBob");
    let line = event(&mut bob, |e| if let BotEvent::Chat(m) = e { Some((m.kind, m.sender, m.message)) } else { None }).await;
    assert_eq!((line.0, line.1.as_deref().map(strip_formatting), line.2.as_str()), (ChatKind::Chat, Some("Alice".to_owned()), "hello Bob"));

    for bot in [&alice, &bob] {
        let movement = bot.movement().expect("physics");
        assert_eq!((movement.corrections, movement.teleports, bot.disconnect_reason()), (0, 0, None));
    }
    alice.disconnect().await;
    let left = event(&mut bob, |e| if let BotEvent::PlayerLeft(p) = e { Some(p.username) } else { None }).await;
    assert_eq!(left, "Alice");
    bob.wait_ticks(5).await.expect("connected");
    assert!(seen(&bob, "Alice").is_none() && bob.state().player_list.len() == 1);
    bob.disconnect().await;
    server.shutdown().await;
}
