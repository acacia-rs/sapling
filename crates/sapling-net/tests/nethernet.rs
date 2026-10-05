//! acacia's client joining a bare listener over NetherNet: by direct connect (HTTP signaling) and
//! as a LAN world (signals in discovery packets). No game answers, as in `login.rs`.

use std::net::SocketAddr;
use std::time::Duration;

use acacia_client::{Client, ClientBuilder, ConnectError, DisconnectReason, LanServer, LanServerData, TransportKind};
use acacia_nethernet::lan::LanPacket;
use p384::ecdsa::SigningKey;
use sapling_net::{Inbox, Lan, Listener, NetConfig, NetEvent, NetherNet, Outbox, SigningKeys, listen};
use tokio::net::UdpSocket;

const WAIT: Duration = Duration::from_secs(20);

async fn start(online_mode: bool) -> (Listener, Outbox, Inbox) {
    let nethernet = NetherNet { lan: Some(Lan { port: 0 }), ..NetherNet::new(SigningKey::from_slice(&[9; 48]).unwrap()) };
    let cfg = NetConfig { online_mode, nethernet: Some(nethernet), ..NetConfig::default() };
    listen("127.0.0.1:0".parse().unwrap(), cfg, SigningKeys::empty()).await.unwrap()
}

fn client(addr: SocketAddr, name: &str, transport: TransportKind) -> ClientBuilder {
    Client::builder(addr.to_string()).offline(name).transport(transport).login_timeout(WAIT)
}

async fn next(inbox: &mut Inbox) -> NetEvent {
    tokio::time::timeout(WAIT, inbox.recv()).await.expect("an event in time").expect("the listener is running")
}

fn kicked(result: Result<Client, ConnectError>) -> bool {
    matches!(result, Err(ConnectError::Disconnected(DisconnectReason::Kicked { .. })))
}

/// One discovery request to the listener's LAN socket, answered with its advert.
async fn discover(listener: &Listener) -> LanServer {
    let advert = listener.lan().expect("LAN is on");
    let addr: SocketAddr = ([127, 0, 0, 1], advert.addr.port()).into();
    let local_id = 77;
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    socket.send_to(&LanPacket::Request.encode(local_id), addr).await.unwrap();
    let mut buf = [0u8; 2048];
    let (len, _) = tokio::time::timeout(WAIT, socket.recv_from(&mut buf)).await.expect("a discovery answer").unwrap();
    let (LanPacket::Response(data), network_id) = LanPacket::decode(&buf[..len]).unwrap() else { panic!("expected a Response") };
    assert_eq!(network_id, advert.network_id);
    LanServer { addr, network_id, data: LanServerData::decode(&data).unwrap(), local_id }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn direct_connect_and_raknet_share_the_port() {
    let (listener, outbox, mut inbox) = start(false).await;
    let addr = listener.local_addr();
    let alice = tokio::spawn(client(addr, "Alice", TransportKind::NetherNet).connect());
    let NetEvent::Joined { conn, login, addr: from } = next(&mut inbox).await else { panic!("expected Joined first") };
    assert_eq!((login.identity.display_name.as_str(), login.authenticated), ("Alice", false));
    assert!(from.ip().is_loopback(), "{from}");

    let bob = tokio::spawn(client(addr, "Bob", TransportKind::RakNet).connect());
    let joined = loop {
        match next(&mut inbox).await {
            NetEvent::Joined { conn, login, .. } => break (conn, login.identity.display_name.clone()),
            NetEvent::Packet { conn: from, .. } => assert_eq!(from, conn),
            NetEvent::Left { reason, .. } => panic!("nobody should leave: {reason}"),
            NetEvent::Rerouted { .. } => panic!("one shard: nobody is rerouted"),
        }
    };
    assert_eq!(joined.1, "Bob");

    outbox.kick(conn, "bye");
    outbox.kick(joined.0, "bye");
    assert!(kicked(alice.await.unwrap()) && kicked(bob.await.unwrap()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn online_mode_refuses_an_offer_without_the_services_token() {
    let (listener, _outbox, mut inbox) = start(true).await;
    let result = client(listener.local_addr(), "Mallory", TransportKind::NetherNet).connect().await;
    assert!(matches!(result, Err(ConnectError::NetherNet(acacia_nethernet::Error::Signaling { code: 37 }))), "{:?}", result.err());
    assert!(inbox.try_recv().is_err(), "nothing reaches the game");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_lan_client_discovers_and_joins() {
    let (listener, outbox, mut inbox) = start(false).await;
    let server = discover(&listener).await;
    assert_eq!((server.data.server_name.as_str(), server.data.player_count, server.data.accepts_self_signed_auth), ("sapling", 0, true));

    let carol = tokio::spawn(Client::builder("lan").offline("Carol").lan(server).login_timeout(WAIT).connect());
    let NetEvent::Joined { conn, login, .. } = next(&mut inbox).await else { panic!("expected Joined first") };
    assert_eq!(login.identity.display_name, "Carol");
    assert_eq!(discover(&listener).await.data.player_count, 1);
    outbox.kick(conn, "bye");
    assert!(kicked(carol.await.unwrap()));
}
