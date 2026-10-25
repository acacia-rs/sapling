//! Logins checked against the access lists: refused before the game hears of them, and a player
//! already in the game dropped when a ban names them.

use std::time::Duration;

use acacia_client::{Client, ConnectError, DisconnectReason, TransportKind};
use sapling_net::access::{AllowEntry, Ban, Terms};
use sapling_net::{AccessLists, Inbox, NetConfig, NetEvent, PlayerLimit, SigningKeys, listen};

const WAIT: Duration = Duration::from_secs(10);

/// The reason and screen of the kick that ended a login.
fn kicked(result: Result<Client, ConnectError>) -> (String, String) {
    match result {
        Err(ConnectError::Disconnected(DisconnectReason::Kicked { reason, message })) => (reason, message),
        Err(other) => panic!("expected a kick, got {other:?}"),
        Ok(_) => panic!("expected a kick, but the client spawned"),
    }
}

async fn next(inbox: &mut Inbox) -> NetEvent {
    tokio::time::timeout(WAIT, inbox.recv()).await.expect("an event in time").expect("the listener is running")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn banned_and_unlisted_logins_are_refused_and_a_new_ban_drops_a_player() {
    let lists = AccessLists::in_memory();
    let cfg = NetConfig { online_mode: false, max_players: PlayerLimit::new(1), access: Some(lists.clone()), ..NetConfig::default() };
    let (listener, _outbox, mut inbox) = listen("127.0.0.1:0".parse().unwrap(), cfg, SigningKeys::empty()).await.unwrap();
    let client = |name: &str| Client::builder(listener.local_addr().to_string()).offline(name).transport(TransportKind::RakNet).login_timeout(WAIT).connect();

    lists.ban(Ban { name: "mallory".into(), xuid: String::new(), terms: Terms::new("griefing", "Alice", None) });
    assert_eq!(kicked(client("Mallory").await), ("Kicked".into(), "You are banned from this server.\nReason: griefing".into()));
    lists.set_allowlist_enabled(true);
    assert_eq!(kicked(client("Bob").await), ("NotAllowed".into(), "disconnectionScreen.notAllowed".into()));
    assert!(inbox.try_recv().is_err(), "nothing reaches the game");

    lists.allow(AllowEntry::named("Bob"));
    lists.allow(AllowEntry { ignores_player_limit: true, ..AllowEntry::named("Carol") });
    let bob = tokio::spawn(client("Bob"));
    let NetEvent::Joined { conn, .. } = next(&mut inbox).await else { panic!("Bob joins") };
    let carol = tokio::spawn(client("Carol"));
    // Carol joins a full server: she ignores the limit.
    while !matches!(next(&mut inbox).await, NetEvent::Joined { conn: other, .. } if other != conn) {}
    carol.abort();

    lists.ban(Ban { name: "Bob".into(), xuid: String::new(), terms: Terms::new("spam", "Alice", Some(Duration::from_secs(3600))) });
    let left = loop {
        if let NetEvent::Left { conn: gone, reason } = next(&mut inbox).await
            && gone == conn
        {
            break reason;
        }
    };
    assert_eq!(left, "banned");
    assert_eq!(kicked(bob.await.unwrap()).1, "You are banned from this server.\nReason: spam\nYour ban ends in 60 minutes.");
}
