//! acacia's client logging in to a bare listener: no game answers, so `connect()` never spawns
//! and only what the listener reports and refuses is checked.

use std::net::SocketAddr;
use std::time::Duration;

use acacia_client::{Client, ClientBuilder, ConnectError, DisconnectReason, TransportKind};
use sapling_net::{Inbox, Listener, NetConfig, NetEvent, Outbox, SigningKeys, listen};

const WAIT: Duration = Duration::from_secs(10);

async fn start(cfg: NetConfig) -> (Listener, Outbox, Inbox) {
    listen("127.0.0.1:0".parse().unwrap(), cfg, SigningKeys::empty()).await.unwrap()
}

fn offline() -> NetConfig {
    NetConfig { online_mode: false, ..NetConfig::default() }
}

fn client(addr: SocketAddr, name: &str) -> ClientBuilder {
    Client::builder(addr.to_string()).offline(name).transport(TransportKind::RakNet).login_timeout(WAIT)
}

async fn next(inbox: &mut Inbox) -> NetEvent {
    tokio::time::timeout(WAIT, inbox.recv()).await.expect("an event in time").expect("the listener is running")
}

fn kick_reason(result: Result<Client, ConnectError>) -> String {
    match result {
        Err(ConnectError::Disconnected(DisconnectReason::Kicked { reason, .. })) => reason,
        Err(other) => panic!("expected a kick, got {other:?}"),
        Ok(_) => panic!("expected a kick, but the client spawned"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_login_is_reported_once_and_its_identity_stays_reserved() {
    let (listener, outbox, mut inbox) = start(offline()).await;
    let addr = listener.local_addr();
    let first = tokio::spawn(client(addr, "Alice").connect());
    let NetEvent::Joined { conn, login, .. } = next(&mut inbox).await else { panic!("expected Joined first") };
    assert_eq!((login.identity.display_name.as_str(), login.authenticated), ("Alice", false));

    assert_eq!(kick_reason(client(addr, "Alice").connect().await), "LoggedInOtherLocation");

    // The client's ClientCacheStatus follows LoginSuccess; then the game's kick ends the session.
    assert!(matches!(next(&mut inbox).await, NetEvent::Packet { conn: from, .. } if from == conn));
    outbox.kick(conn, "bye");
    assert!(matches!(next(&mut inbox).await, NetEvent::Left { conn: gone, .. } if gone == conn));
    assert_eq!(kick_reason(first.await.unwrap()), "Kicked");

    // The name is free again.
    let again = tokio::spawn(client(addr, "Alice").connect());
    assert!(matches!(next(&mut inbox).await, NetEvent::Joined { conn: new, .. } if new != conn));
    again.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn online_mode_refuses_self_signed_logins() {
    let (listener, _outbox, mut inbox) = start(NetConfig::default()).await;
    assert_eq!(kick_reason(client(listener.local_addr(), "Mallory").connect().await), "NotAuthenticated");
    assert!(inbox.try_recv().is_err(), "nothing reaches the game");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_full_server_and_a_slow_login_are_turned_away() {
    let (listener, _outbox, mut inbox) = start(NetConfig { max_players: 0, ..offline() }).await;
    assert_eq!(kick_reason(client(listener.local_addr(), "Bob").connect().await), "ServerFull");
    assert!(inbox.try_recv().is_err());

    let (listener, _outbox, mut inbox) = start(NetConfig { login_timeout: Duration::ZERO, ..offline() }).await;
    let result = client(listener.local_addr(), "Carol").connect().await;
    assert!(matches!(result, Err(ConnectError::Disconnected(DisconnectReason::Transport(_)))), "{:?}", result.err());
    assert!(inbox.try_recv().is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropping_the_outbox_closes_the_listener_and_its_players() {
    let (listener, outbox, mut inbox) = start(offline()).await;
    let joining = tokio::spawn(client(listener.local_addr(), "Dave").connect());
    assert!(matches!(next(&mut inbox).await, NetEvent::Joined { .. }));
    drop(outbox);
    tokio::time::timeout(WAIT, listener.closed()).await.expect("the driver stops");
    assert_eq!(kick_reason(joining.await.unwrap()), "Kicked");
}
