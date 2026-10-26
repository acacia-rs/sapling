//! The access lists on a running server: an operator bans, pardons, kicks and turns the allowlist
//! on, and logins are turned away before they reach the world.

use crate::common;
#[path = "common/travel.rs"]
mod travel;

use acacia_bot::client::PacketFilter;
use acacia_bot::proto::Packet;
use acacia_bot::proto::packets::CommandOutput;
use acacia_bot::{Bot, BotConfig, BotEvent};
use acacia_client::{ConnectError, DisconnectReason};
use sapling::{AccessLists, Handler, Permission, Player, Server, ShardCtx};
use travel::{WAIT, run};

/// Alice is the operator.
struct Alice;

impl Handler for Alice {
    fn on_join(&mut self, ctx: &mut ShardCtx<'_>, player: &Player) {
        if player.name == "Alice" {
            ctx.set_permission(player.id, Permission::Operator);
        }
    }
}

/// The reason and screen of the kick that turned a login away.
async fn refused(server: &Server, name: &str) -> (String, String) {
    match common::client(server, name).connect().await {
        Err(ConnectError::Disconnected(DisconnectReason::Kicked { reason, message })) => (reason, message),
        Err(other) => panic!("expected a kick, got {other:?}"),
        Ok(_) => panic!("{name} was let in"),
    }
}

/// The screen a bot in the game was kicked with.
async fn kicked(bot: &mut Bot) -> String {
    let ended = tokio::time::timeout(WAIT, async {
        loop {
            match bot.next().await {
                Some(BotEvent::Disconnected(DisconnectReason::Kicked { message, .. })) => return message,
                Some(BotEvent::Disconnected(other)) => panic!("disconnected otherwise: {other:?}"),
                Some(_) => {}
                None => panic!("the connection ended without a kick"),
            }
        }
    });
    ended.await.expect("kicked in time")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn operators_ban_kick_and_allowlist_players() {
    let lists = AccessLists::in_memory();
    let server = common::start(common::builder().handler(Alice).vanilla_commands().access(lists.clone())).await;
    let outputs = BotConfig { subscribe: PacketFilter::none().with(CommandOutput::ID), ..BotConfig::default() };
    let mut alice = common::spawn(common::client(&server, "Alice"), outputs).await;
    let mut bob = common::spawn(common::client(&server, "Bob"), BotConfig::default()).await;

    assert_eq!(run(&mut alice, "ban Bob cheating").await, ("commands.ban.success".into(), true));
    assert_eq!(kicked(&mut bob).await, "You are banned from this server.\nReason: cheating");
    assert_eq!(refused(&server, "bob").await, ("Kicked".into(), "You are banned from this server.\nReason: cheating".into()));
    assert_eq!(run(&mut alice, "ban bob").await, ("commands.ban.failed".into(), false));
    assert_eq!(run(&mut alice, "banlist").await, ("commands.banlist.players".into(), true));
    assert_eq!(lists.bans()[0].terms.source, "Alice");
    assert_eq!(run(&mut alice, "pardon BOB").await, ("commands.unban.success".into(), true));
    assert_eq!(run(&mut alice, "pardon Bob").await, ("commands.unban.failed".into(), false));

    assert_eq!(run(&mut alice, "whitelist on").await, ("commands.allowlist.enabled".into(), true));
    assert_eq!(run(&mut alice, "allowlist add Carol").await, ("commands.allowlist.add.success".into(), true));
    assert_eq!(refused(&server, "Dave").await, ("NotAllowed".into(), "disconnectionScreen.notAllowed".into()));
    let mut carol = common::spawn(common::client(&server, "Carol"), BotConfig::default()).await;
    assert_eq!(run(&mut alice, "op Carol").await, ("commands.op.success".into(), true));
    assert_eq!(run(&mut alice, "op Carol").await, ("commands.op.failed".into(), false));
    assert_eq!(run(&mut alice, "deop Carol").await, ("commands.deop.success".into(), true));
    assert_eq!(run(&mut alice, "kick Alice").await, ("commands.kick.not.yourself".into(), false));
    assert_eq!(run(&mut alice, "kick Nobody").await, ("commands.kick.not.found".into(), false));
    assert_eq!(run(&mut alice, "kick Carol go home").await, ("commands.kick.success.reason".into(), true));
    assert_eq!(kicked(&mut carol).await, "go home");
    assert_eq!(run(&mut alice, "allowlist remove carol").await, ("commands.allowlist.remove.success".into(), true));
    assert_eq!(run(&mut alice, "allowlist off").await, ("commands.allowlist.disabled".into(), true));

    // Not loopback: that would turn Alice away too.
    assert_eq!(run(&mut alice, "ban-ip 203.0.113.9 proxies").await, ("commands.banip.success".into(), true));
    assert_eq!(lists.ip_bans()[0].terms.reason, "proxies");
    assert_eq!(run(&mut alice, "ban-ip Nobody").await, ("commands.banip.invalid".into(), false));
    assert_eq!(run(&mut alice, "pardon-ip 203.0.113.9").await, ("commands.unbanip.success".into(), true));
    assert_eq!(run(&mut alice, "pardon-ip banana").await, ("commands.unbanip.invalid".into(), false));
    assert!(lists.ip_bans().is_empty() && lists.bans().is_empty() && lists.allowlist().is_empty());
    alice.disconnect().await;
    server.shutdown().await;
}
