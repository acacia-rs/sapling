//! A player's items are kept by the shard players join on, whichever shard they leave from: a
//! bot is given diamonds in a world on the second shard, leaves there, and has them on its return.

mod common;

use std::time::Duration;

use acacia_bot::{Bot, BotConfig};
use sapling::{Command, Handler, Item, Output, Player, Sender, Server, ShardCtx};

const DIAMOND: &str = "minecraft:diamond";

/// Gives everyone who enters the arena three diamonds.
struct Prize;

impl Handler for Prize {
    fn on_world_change(&mut self, ctx: &mut ShardCtx<'_>, player: &Player, _from: &str, to: &str) {
        if to == "arena" {
            assert_eq!(ctx.give(player.id, &Item::new(DIAMOND, 3)), 0);
        }
    }
}

#[derive(Command)]
#[command(name = "arena")]
struct ToArena;

impl Command for ToArena {
    fn run(self, ctx: &mut ShardCtx<'_>, sender: &Sender, out: &mut Output) {
        let went = sender.player().is_some_and(|player| ctx.transfer(player.id, "arena"));
        out.print(format!("to the arena: {went}"));
    }
}

/// How many of an item the bot's own copy of its inventory holds.
fn held(bot: &Bot, name: &str) -> u32 {
    let state = bot.state();
    state.inventory.main.iter().filter(|stack| !stack.is_empty() && state.items.name(stack.network_id) == Some(name)).map(|stack| u32::from(stack.count)).sum()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn what_a_player_gains_on_another_shard_is_there_when_it_joins_again() {
    let builder = Server::builder().default_world_name("hub").add_platform_on(1, "arena").handler_each(|| Prize).command::<ToArena>();
    let server = common::start(builder).await;
    assert_eq!(server.shard_count(), 2);

    let mut alice = common::spawn(common::client(&server, "Alice"), BotConfig::default()).await;
    alice.wait_ticks(20).await.expect("connected");
    assert_eq!(held(&alice, DIAMOND), 0);
    alice.client().command("arena");
    alice.wait_ticks(60).await.expect("connected");
    assert_eq!(held(&alice, DIAMOND), 3, "given on the arena's shard");
    alice.disconnect().await;
    // The arena's shard hands the items to the hub's, whose storage takes them on its own thread.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut again = common::spawn(common::client(&server, "Alice"), BotConfig::default()).await;
    again.wait_ticks(20).await.expect("connected");
    assert_eq!(held(&again, DIAMOND), 3, "kept by the shard she joins on");
    assert_eq!(again.disconnect_reason(), None);
    again.disconnect().await;
    server.shutdown().await;
}
