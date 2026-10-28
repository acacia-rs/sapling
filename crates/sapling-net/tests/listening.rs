//! What the listener answers without a login: the GameSpy4 query and the RakNet ping on an IPv6
//! socket beside the main one.

use std::net::SocketAddr;
use std::time::Duration;

use acacia_client::{Client, TransportKind};
use sapling_net::{Motd, NetConfig, NetEvent, SigningKeys, listen};
use tokio::net::UdpSocket;

const WAIT: Duration = Duration::from_secs(10);
const RAKNET_MAGIC: [u8; 16] = [0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78];

async fn ask(socket: &UdpSocket, to: SocketAddr, datagram: &[u8]) -> Vec<u8> {
    socket.send_to(datagram, to).await.unwrap();
    let mut buf = vec![0; 2048];
    let (len, _) = tokio::time::timeout(WAIT, socket.recv_from(&mut buf)).await.expect("an answer in time").unwrap();
    buf.truncate(len);
    buf
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_query_tells_the_name_map_and_players() {
    let motd = Motd { name: "hub".into(), sub_name: "Spawn".into(), ..Motd::default() };
    let cfg = NetConfig { online_mode: false, query: true, motd, ..NetConfig::default() };
    let (listener, _outbox, mut inbox) = listen("127.0.0.1:0".parse().unwrap(), cfg, SigningKeys::empty()).await.unwrap();
    let addr = listener.local_addr();
    let joining = tokio::spawn(Client::builder(addr.to_string()).offline("Alice").transport(TransportKind::RakNet).login_timeout(WAIT).connect());
    let joined = tokio::time::timeout(WAIT, inbox.recv()).await.unwrap();
    assert!(matches!(joined, Some(NetEvent::Joined { .. })));

    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let hand = ask(&socket, addr, &[0xFE, 0xFD, 9, 0, 0, 0, 1]).await;
    let token: i32 = std::str::from_utf8(&hand[5..hand.len() - 1]).unwrap().parse().unwrap();
    let mut stat = vec![0xFE, 0xFD, 0, 0, 0, 0, 1];
    stat.extend_from_slice(&token.to_be_bytes());
    let port = addr.port().to_le_bytes();
    let mut basic = b"\0\0\0\0\x01hub\0SMP\0Spawn\x001\x00100\0".to_vec();
    basic.extend_from_slice(&[port[0], port[1]]);
    basic.extend_from_slice(b"127.0.0.1\0");
    assert_eq!(ask(&socket, addr, &stat).await, basic);
    stat.extend_from_slice(&[0; 4]);
    assert!(ask(&socket, addr, &stat).await.ends_with(b"\x01player_\0\0Alice\0\0"));
    joining.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_ipv6_socket_answers_pings_and_both_ports_are_advertised() {
    let Ok(probe) = UdpSocket::bind("[::1]:0").await else {
        eprintln!("no IPv6 loopback here; skipped");
        return;
    };
    let cfg = NetConfig { online_mode: false, bind_v6: Some("[::1]:0".parse().unwrap()), ..NetConfig::default() };
    let (listener, _outbox, _inbox) = listen("127.0.0.1:0".parse().unwrap(), cfg, SigningKeys::empty()).await.unwrap();
    let mut ping = vec![0x01];
    ping.extend_from_slice(&7u64.to_be_bytes());
    ping.extend_from_slice(&RAKNET_MAGIC);
    ping.extend_from_slice(&42u64.to_be_bytes());
    let pong_of = |reply: Vec<u8>| String::from_utf8(reply[35..].to_vec()).unwrap();

    let v4 = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let pong = pong_of(ask(&v4, listener.local_addr(), &ping).await);
    let fields: Vec<&str> = pong.split(';').collect();
    let port_v6: u16 = fields[11].parse().unwrap();
    assert_eq!(fields[10], listener.local_addr().port().to_string());
    assert_ne!(port_v6, listener.local_addr().port());
    assert_eq!(pong_of(ask(&probe, SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], port_v6)), &ping).await), pong);
}
