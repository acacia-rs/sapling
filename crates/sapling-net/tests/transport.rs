//! An embedder's transport beside the built-in ones: RakNet on a second port, with a socket of its
//! own polled in `poll_ready`. acacia's client logs in over it as over the game port.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use acacia_client::{Client, ConnectError, DisconnectReason, TransportKind};
use acacia_raknet::{Reliability, Server, ServerConfig, ServerEvent};
use bytes::Bytes;
use sapling_net::{ConnKey, NetConfig, NetEvent, SigningKeys, Transport, TransportEvent, listen_with};
use tokio::io::ReadBuf;
use tokio::net::UdpSocket;

const WAIT: Duration = Duration::from_secs(20);

struct SecondPort {
    socket: UdpSocket,
    server: Server,
    keys: HashMap<SocketAddr, ConnKey>,
    addrs: HashMap<ConnKey, SocketAddr>,
    next: ConnKey,
}

impl SecondPort {
    async fn bind() -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server = Server::new(ServerConfig::new(7, "MCPE;second;1;1.0;0;10;7;second;Survival;1;0;0;0;".into()), Instant::now());
        Self { socket, server, keys: HashMap::new(), addrs: HashMap::new(), next: 1 }
    }
}

impl Transport for SecondPort {
    fn name(&self) -> &str {
        "second port"
    }

    fn poll_ready(&mut self, cx: &mut Context<'_>, now: Instant) -> Poll<()> {
        let mut buf = [0u8; 2048];
        let mut read = false;
        loop {
            let mut filled = ReadBuf::new(&mut buf);
            match self.socket.poll_recv_from(cx, &mut filled) {
                Poll::Ready(Ok(from)) => {
                    self.server.handle_datagram(now, from, Bytes::copy_from_slice(filled.filled()));
                    read = true;
                }
                _ => break,
            }
        }
        // Sent here rather than through poll_transmit, which is for the game socket.
        while let Some((to, datagram)) = self.server.poll_transmit(now) {
            let _ = self.socket.try_send_to(&datagram, to);
        }
        if read { Poll::Ready(()) } else { Poll::Pending }
    }

    fn poll_timeout(&self) -> Option<Instant> {
        self.server.poll_timeout()
    }

    fn handle_timeout(&mut self, now: Instant) {
        self.server.handle_timeout(now);
    }

    fn poll_event(&mut self) -> Option<TransportEvent> {
        loop {
            return Some(match self.server.poll_event()? {
                ServerEvent::Connected { addr, .. } => {
                    self.next += 1;
                    self.keys.insert(addr, self.next);
                    self.addrs.insert(self.next, addr);
                    TransportEvent::Connected(self.next)
                }
                ServerEvent::Message(addr, msg) => TransportEvent::Message(*self.keys.get(&addr)?, msg),
                ServerEvent::Disconnected(addr, reason) => match self.keys.remove(&addr) {
                    Some(key) => TransportEvent::Disconnected(key, format!("{reason:?}")),
                    None => continue,
                },
            });
        }
    }

    fn send(&mut self, conn: ConnKey, batch: Bytes, _now: Instant) {
        if let Some(&addr) = self.addrs.get(&conn) {
            self.server.send(addr, batch, Reliability::ReliableOrdered);
        }
    }

    fn close(&mut self, conn: ConnKey, now: Instant) {
        if let Some(&addr) = self.addrs.get(&conn) {
            self.server.close(addr, now);
        }
    }

    fn remote_addr(&self, conn: ConnKey) -> Option<SocketAddr> {
        self.addrs.get(&conn).copied()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_player_logs_in_over_an_embedders_transport() {
    let second = SecondPort::bind().await;
    let addr = second.socket.local_addr().unwrap();
    let cfg = NetConfig { online_mode: false, ..NetConfig::default() };
    let (_listener, outbox, mut inboxes) = listen_with("127.0.0.1:0".parse().unwrap(), cfg, SigningKeys::empty(), 1, 0, vec![Box::new(second)]).await.unwrap();
    let inbox = &mut inboxes[0];

    let erin = tokio::spawn(Client::builder(addr.to_string()).offline("Erin").transport(TransportKind::RakNet).login_timeout(WAIT).connect());
    let event = tokio::time::timeout(WAIT, inbox.recv()).await.expect("a join in time").unwrap();
    let NetEvent::Joined { conn, login, addr: from } = event else { panic!("expected Joined first") };
    assert_eq!((login.identity.display_name.as_str(), from.ip().is_loopback()), ("Erin", true));

    outbox.kick(conn, "bye");
    let kicked = erin.await.unwrap();
    assert!(matches!(kicked, Err(ConnectError::Disconnected(DisconnectReason::Kicked { .. }))), "{:?}", kicked.err());
}
