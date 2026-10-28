# NetherNet reach: ICE servers, a port range, HTTPS signaling and pluggable transports

Branch `parity/nethernet` (workstream G9), with acacia branch `parity/nethernet`. What a server behind NAT
needs to be joined over NetherNet, and the transport seam an embedder plugs into. Evidence: acacia's
`docs/research/nethernet-wire.md` (direct connect, captures against BDS 26.5x), a bdsre string search of the
`bds-blocks` build, libwebrtc behaviour as acacia's TURN client already copies it (`turn/mod.rs`), and
dragonfly's `server/listener.go` for the API shape. No vanilla client was available.

## What BDS does

- **Direct connect has no ICE servers.** BDS answers `POST /v1/join/{id}` with host candidates only (its LAN
  and IPv6 addresses; nethernet-wire.md "Answered by testing"). Its binary has no server.properties key
  for STUN or TURN (`str "stun-server"`, `"turn-server"`, `"ice-server"`: no hits). Its only STUN/TURN
  servers are the signaling service's `TurnAuthServers` (strings at 0x140591261, 0x1405725de), used for
  friends-list and Realms joins, which sapling does not serve. libwebrtc's URL parser (`'stun:'` in fns
  0x14a3690d0, 0x14a3695b0) is linked in.
- **MOJ:553** (nethernet-wire.md §3): "for non-loopback joins the server must include a reachable srflx
  candidate", which it leaves to the operator (a forwarded port, as `NetherNet::public_ip` already
  covers).
- So STUN/TURN on the server is a sapling extension, as dragonfly's `NetherNetConfig.Credentials` is.
  It changes nothing for a server without `ice_servers`.

## Design

- **ICE servers** (`NetherNet::ice_servers`, acacia's `IceServer`: `stun:`/`turn:` URLs, TURN credentials).
  Resolved once at `listen` to the game socket's family (`nethernet.rs` `resolve_ice`); TURN over TCP or
  TLS and names that do not resolve are logged and left out. acacia's `Host` gathers on the NetherNet
  socket (`host/gather.rs`):
  - one STUN Binding per STUN server, re-sent every 10 s to hold the NAT mapping (libwebrtc's STUN
    keepalive), retried after 60 s if a server never answers (9 sends on libwebrtc's 250 ms..8 s schedule);
  - one TURN allocation per TURN server, **shared by every peer**: per-peer permissions and channels on one
    allocation, where libwebrtc gives each PeerConnection its own. A lost or failed allocation is retried
    after 60 s. The Allocate's XOR-MAPPED-ADDRESS also becomes a srflx candidate, as libwebrtc's
    `WebRTC-UseTurnServerAsStunServer` (string at 0x14a29e648) does;
  - answers embed host, then srflx (deduplicated, same family as the host candidate), then relay
    candidates; LAN answers trickle them;
  - permissions: every address in the offer's or trickled candidates, plus where the signaling came from
    (the HTTP peer, the LAN sender), since a vanilla client offers only host candidates and behind NAT
    those are private. TURN permissions are per IP. A closed peer's addresses are forgotten unless another
    peer still uses them; a forgotten channel number is reused 15 min later (its 10 min binding plus RFC
    8656's 5 min).
  - Relayed datagrams are routed like direct ones (by ICE ufrag, then by (relay, source)), so RakNet and
    every NetherNet peer still share one socket and one STUN/TURN client.
- **Port range** (`NetherNet::udp_ports`, `PortRange`, written `19133` or `19133-19140` like dragonfly's
  `UDPPorts`): NetherNet gets its own UDP socket on the first free port, and its candidates name it. All
  peers share that socket (dragonfly's single-port mux case); dragonfly's wide range gives each
  connection its own ephemeral port instead, which sapling's routed host does not need. `None` keeps
  NetherNet on the game socket. Signaling stays on the game port's TCP side.
- **HTTPS** (`NetherNet::tls`, PEM chain and key): served on the signaling port beside HTTP, told apart by
  the first byte (`0x16`), with rustls (ring). Without it a TLS hello still gets the alert that sends the
  vanilla client to HTTP. Under HTTPS the client trusts the answer's identity by the certificate
  instead of pinning the host key (nethernet-wire.md §2), so the certificate must match the name players
  join by. Dragonfly leaves HTTPS to a reverse proxy.
- **Transports** (`sapling-net/src/transport.rs`, `Transport`): a sans-IO trait (events, send, close,
  backlog, remote address, timers), with `handle_datagram`/`poll_transmit` for transports fed by the game
  socket and `poll_ready` for ones with I/O of their own; `Framing` says whether batches carry RakNet's
  `0xfe` header and encryption or go plain as NetherNet's. RakNet (`RakNetTransport`, conns keyed by
  address) and NetherNet (`NetherNetTransport`, by peer) implement it; `Links` offers a game-socket
  datagram to NetherNet, then embedders' transports, RakNet last. `ServerBuilder::transport` (or
  `sapling_net::listen_with`) adds one; its players log in, are limited, banned and kicked as any other.
  This is gophertunnel's `Network` level (dragonfly's `ListenNetwork`), below the login; dragonfly's own
  `Listener` sits above it and hands over logged-in connections, which sapling keeps in one place.

## Tests

- acacia: `host::relay_tests` (a client that reaches the host only through its relay: srflx and relay in
  the answer, channels open, a message each way), `turn::client_tests::forgotten_channels_come_round…`,
  `host::route::tests::reads_candidate_addresses…`.
- sapling-net: `tests/nethernet.rs` `answers_carry_stun_and_turn_candidates` (a UDP fake STUN/TURN server;
  the answer over HTTP carries both), `a_port_range_gives_nethernet_its_own_socket` (acacia's client joins
  through it), `tests/signaling_tls.rs` (HTTPS and HTTP on one port), `tests/transport.rs` (RakNet on a
  second port as an embedder's transport; acacia's client logs in and is kicked).

## Not drilled

Only a vanilla client can confirm:
- that it uses a srflx or relay candidate from a direct-connect answer when the host one fails, and that
  libwebrtc accepts relay candidates in a non-trickled answer;
- that it trusts HTTPS signaling with a valid certificate (no TOFU prompt), and what it does when the
  certificate is wrong: fall back to HTTP, or fail;
- that sharing one TURN allocation among peers raises nothing on its side (it sees only the relay
  address);
- STUN keepalive 10 s and the 60 s retries are libwebrtc's and sapling's choices, not read from BDS.
