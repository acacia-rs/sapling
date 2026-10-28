//! acacia's client joining a bare listener over NetherNet: by direct connect (HTTP signaling) and
//! as a LAN world (signals in discovery packets). No game answers, as in `login.rs`.

use std::net::SocketAddr;
use std::time::Duration;

use acacia_client::{Client, ClientBuilder, ConnectError, DisconnectReason, LanServer, LanServerData, TransportKind};
use acacia_nethernet::lan::LanPacket;
use acacia_nethernet::turn::stun::{Attr, Class, Message, Method};
use p384::ecdsa::SigningKey;
use sapling_net::{IceServer, Inbox, Lan, Listener, NetConfig, NetEvent, NetherNet, Outbox, SigningKeys, listen};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};

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
async fn a_port_range_gives_nethernet_its_own_socket() {
    let nethernet = NetherNet { udp_ports: Some("0".parse().unwrap()), ..NetherNet::new(SigningKey::from_slice(&[9; 48]).unwrap()) };
    let cfg = NetConfig { online_mode: false, nethernet: Some(nethernet), ..NetConfig::default() };
    let (listener, outbox, mut inbox) = listen("127.0.0.1:0".parse().unwrap(), cfg, SigningKeys::empty()).await.unwrap();
    let port = listener.nethernet_port().expect("NetherNet is on");
    assert_ne!(port, listener.local_addr().port());
    let offer = offer().await;
    assert!(post_offer(listener.local_addr(), &offer).await.contains(&format!(" 127.0.0.1 {port} typ host ")));

    let dave = tokio::spawn(client(listener.local_addr(), "Dave", TransportKind::NetherNet).connect());
    let NetEvent::Joined { conn, .. } = next(&mut inbox).await else { panic!("expected Joined first") };
    outbox.kick(conn, "bye");
    assert!(kicked(dave.await.unwrap()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn answers_carry_stun_and_turn_candidates() {
    let ice = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let ice_addr = ice.local_addr().unwrap();
    tokio::spawn(fake_ice_server(ice));
    let url = |scheme: &str| format!("{scheme}:{ice_addr}").parse().unwrap();
    let server = IceServer { username: "u".into(), password: "p".into(), urls: vec![url("stun"), url("turn")] };
    let nethernet = NetherNet { ice_servers: vec![server], ..NetherNet::new(SigningKey::from_slice(&[9; 48]).unwrap()) };
    let cfg = NetConfig { online_mode: false, nethernet: Some(nethernet), ..NetConfig::default() };
    let (listener, _outbox, _inbox) = listen("127.0.0.1:0".parse().unwrap(), cfg, SigningKeys::empty()).await.unwrap();
    let (offer, deadline) = (offer().await, tokio::time::Instant::now() + WAIT);
    loop {
        let answer = post_offer(listener.local_addr(), &offer).await;
        if answer.contains(" 203.0.113.1 40000 typ srflx ") && answer.contains(" 127.0.0.1 49152 typ relay raddr 203.0.113.1 rport 40000 ") {
            break;
        }
        assert!(tokio::time::Instant::now() < deadline, "no gathered candidates: {answer}");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// A STUN binding and a TURN allocation that always succeed, unauthenticated.
async fn fake_ice_server(socket: UdpSocket) {
    let mut buf = [0u8; 2048];
    while let Ok((len, from)) = socket.recv_from(&mut buf).await {
        let Ok(request) = Message::decode(&buf[..len]) else { continue };
        let mut reply = Message::new(Class::Success, request.method, request.transaction_id).with(Attr::XorMappedAddress("203.0.113.1:40000".parse().unwrap()));
        if request.method == Method::Allocate {
            reply = reply.with(Attr::XorRelayedAddress("127.0.0.1:49152".parse().unwrap())).with(Attr::Lifetime(600));
        }
        let _ = socket.send_to(&reply.encode(None, true), from).await;
    }
}

async fn offer() -> String {
    let identity = acacia_nethernet::Identity::self_signed(SigningKey::from_slice(&[5; 48]).unwrap());
    acacia_nethernet::Connection::offer("127.0.0.1:4000".parse().unwrap(), &identity, std::time::Instant::now()).unwrap().1
}

/// `POST /v1/join/1` with `offer`; the whole response.
async fn post_offer(addr: SocketAddr, offer: &str) -> String {
    let mut tcp = TcpStream::connect(addr).await.unwrap();
    let request = format!("POST /v1/join/1 HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/sdp\r\nContent-Length: {}\r\n\r\n{offer}", offer.len());
    tcp.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    tcp.read_to_string(&mut response).await.unwrap();
    response
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
