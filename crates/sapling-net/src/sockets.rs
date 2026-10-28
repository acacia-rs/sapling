//! The game's UDP sockets: the one `bind` names and, with [`crate::NetConfig::bind_v6`], an
//! IPv6-only one beside it, as BDS listens on `server-port` and `server-portv6` ("IPv4 supported,
//! port: 19182" / "IPv6 supported, port: 19183", research/captures/bds-query.txt). Both carry
//! RakNet, NetherNet's datagrams and the query; a reply leaves by the socket of its address's family.

use std::future::poll_fn;
use std::io;
use std::net::SocketAddr;
use std::task::Poll;

use socket2::{Domain, Protocol, Socket, Type};
use tokio::io::ReadBuf;
use tokio::net::{TcpListener, UdpSocket};

/// Tries for a UDP and TCP pair on one free port before giving up.
const PAIR_ATTEMPTS: u32 = 16;

pub(crate) struct Sockets {
    main: UdpSocket,
    v6: Option<UdpSocket>,
}

impl Sockets {
    /// Binds `addr`, with NetherNet a TCP listener on the same port number, and `v6` if given.
    pub async fn bind(addr: SocketAddr, v6: Option<SocketAddr>, with_tcp: bool) -> io::Result<(Self, Option<TcpListener>)> {
        let (main, tcp) = bind_main(addr, with_tcp).await?;
        let v6 = v6.map(bind_v6).transpose()?;
        Ok((Self { main, v6 }, tcp))
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.main.local_addr()
    }

    /// The IPv4 and IPv6 ports the pong names; the main port twice without an IPv6 socket.
    pub fn ports(&self) -> [u16; 2] {
        let port = |socket: &UdpSocket| socket.local_addr().map_or(0, |addr| addr.port());
        [port(&self.main), self.v6.as_ref().map_or(port(&self.main), port)]
    }

    /// The local address of the socket `from` reached.
    pub fn reached(&self, from: SocketAddr) -> io::Result<SocketAddr> {
        self.socket_for(from).local_addr()
    }

    fn socket_for(&self, peer: SocketAddr) -> &UdpSocket {
        match &self.v6 {
            Some(v6) if peer.is_ipv6() => v6,
            _ => &self.main,
        }
    }

    fn all(&self) -> impl Iterator<Item = &UdpSocket> {
        std::iter::once(&self.main).chain(&self.v6)
    }

    /// The next datagram on either socket.
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        poll_fn(|cx| {
            for socket in self.all() {
                let mut read = ReadBuf::new(&mut *buf);
                if let Poll::Ready(received) = socket.poll_recv_from(cx, &mut read) {
                    return Poll::Ready(received.map(|from| (read.filled().len(), from)));
                }
            }
            Poll::Pending
        })
        .await
    }

    /// A datagram already waiting on either socket.
    pub fn try_recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        let mut last = Err(io::ErrorKind::WouldBlock.into());
        for socket in self.all() {
            last = socket.try_recv_from(buf);
            if last.is_ok() {
                break;
            }
        }
        last
    }

    pub async fn send_to(&self, datagram: &[u8], to: SocketAddr) -> io::Result<usize> {
        self.socket_for(to).send_to(datagram, to).await
    }
}

async fn bind_main(addr: SocketAddr, with_tcp: bool) -> io::Result<(UdpSocket, Option<TcpListener>)> {
    if !with_tcp {
        return Ok((UdpSocket::bind(addr).await?, None));
    }
    let mut attempts = if addr.port() == 0 { PAIR_ATTEMPTS } else { 1 };
    loop {
        let socket = UdpSocket::bind(addr).await?;
        attempts -= 1;
        match TcpListener::bind(socket.local_addr()?).await {
            Ok(tcp) => return Ok((socket, Some(tcp))),
            Err(error) if attempts == 0 => return Err(error),
            Err(_) => {}
        }
    }
}

/// IPv6 only, so it never takes the IPv4 port the main socket may share with it.
fn bind_v6(addr: SocketAddr) -> io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_only_v6(true)?;
    socket.set_nonblocking(true)?;
    socket.bind(&addr.into())?;
    UdpSocket::from_std(socket.into())
}
