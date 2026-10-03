# acacia → sapling: server-side gap analysis

Source: acacia worktree `C:/Users/Hexay/acacia/acacia-gaps`, read 2026-10-03. Paths are relative to `crates/`.
Login verification facts come from gophertunnel `master` (MIT), Cloudburst Protocol 3.0, PocketMine (behaviour
only) and the live OpenID/JWKS endpoints, all fetched 2026-10-03.

Legend: **A** = add to acacia (generic, network-free, reusable). **S** = sapling owns it (policy, I/O, data, gameplay).

---

## 1. RakNet server (`acacia-raknet/src/server.rs`)

### What exists
- Sans-IO `Server`, which has:
  - unconnected ping → pong with the MOTD (`server.rs:135-138`);
  - OCR1 and OCR2 (`:139-155`);
  - ConnectionRequest → Accepted, and NewIncomingConnection → `Connected` (`:168-176`);
  - one `Conn` per `SocketAddr` in a `HashMap` (`:54`).
- `Conn` is shared with the client: reliability, splits, ordering, 10 ms ACK tick, connected ping/pong, and an idle timeout of 10 s by default (`conn.rs`, `server.rs:24-33`).
- The receive side has sane caps: 8192 parts per split, 64 concurrent splits, an order backlog of 8192, and a reliable window of 65536 (`reliability/recv.rs:9-13`).
- The only tokio drivers live in `acacia-mitm/src/raknet.rs:33-150` and `acacia-testserver/src/server.rs`. Both are single-task loops over one socket.

### Gaps (ordered by severity)
| # | Gap | Evidence | Fix |
|---|---|---|---|
| R1 | **Remote panic or corruption from a small MTU.** OCR2's client-chosen `mtu` is only clamped from above (`mtu.min(max_mtu)`). Below 28 bytes, `self.mtu - UDP_OVERHEAD` underflows a `u16`: a panic in debug, a wrap to about 65.5k in release. Below about 45 bytes, `room - UNSPLIT_HEADER_LEN` and `room - MAX_FRAME_HEADER_LEN` underflow a `usize`. | `server.rs:148-152`, `conn.rs:58-60`, `reliability/send.rs:107-115` | **A**: clamp to `[576, max_mtu]` (BDS uses a 400–1492 range) and reject OCR2 below the minimum. |
| R2 | **No security cookie, and no spoofing protection.** `reply_1` always writes "no cookie". A spoofed OCR2 from a player's address drops and recreates that player's peer. Every OCR2 also allocates a `Conn` that lives for the full idle timeout. | `wire/offline.rs:142-149`, `server.rs:147-153` | **A**: an HMAC(addr, rotating secret) cookie in Reply1, verified in OCR2. The client already parses cookies (`offline.rs:79,93-96`). Answer OCR2 for an existing *connected* peer with `ID_ALREADY_CONNECTED` instead of resetting it. |
| R3 | **Per-peer memory amplification.** Each new split id allocates `vec![None; count]` with `count ≤ 8192`, about 256 KiB of `Option<Bytes>`. 64 split ids give about 16 MiB per peer from 64 tiny datagrams, and filled data can reach about 64×8192×MTU. There is no per-peer byte budget. | `reliability/recv.rs:106-117` | **A**: cap the total reassembly bytes per peer (a few MiB) and the split count (BDS uses about 512), and allocate lazily. |
| R4 | **No congestion control or send window.** `pack` drains every pending frame immediately, and `poll_transmit` hands everything out. A burst of a few MB (the initial chunk radius, resource packs) goes out at line rate, which causes loss, a NACK storm and full resends. | `send.rs:128-151`, `conn.rs:107-116` | **A**: a datagram window (RakNet-style sliding window or a simple AIMD on ACK/NACK) plus pacing. Expose queue depth per peer so the server can throttle chunk sending. |
| R5 | **RTO never backs off and resends never give up.** RTO is clamped to 200 ms–3 s with no exponential growth. Lost datagrams resend forever until the 10 s idle timeout. | `send.rs:12-14,44-46,176-186` | **A**: double RTO per retransmit and cap retries. |
| R6 | **O(N) scans on the hot path.** `poll_transmit` runs `find_map` over every peer *per datagram* (O(N·D) to drain). `poll_timeout` and `handle_timeout` iterate all peers. With 500 players at about 20 datagrams per tick, that is millions of iterations per tick. | `server.rs:90-111` | **A**: a ready set/queue of peers with pending output, and a timer heap or wheel. **S**: shard peers across N sockets/tasks (`SO_REUSEPORT` on Linux), or one `Server` per core keyed by address hash. |
| R7 | **No admission control.** `ID_NO_FREE_INCOMING_CONNECTIONS`, `ID_CONNECTION_BANNED` and `ID_IP_RECENTLY_CONNECTED` are defined but unused. There are no max peers, no per-IP limit, no handshake deadline separate from idle, and no rate limit on pings. The pong is larger than the ping, a 3–5× amplification. | `offline.rs:12-16`, `server.rs:132-160` | **A**: a `ServerConfig.max_peers`, a handshake timeout, and hooks or a callback for ban / per-IP decisions. **S**: the policy itself (ban list, rate limits). |
| R8 | **Missing per-peer observability.** `Conn::rtt()` exists but `Server` doesn't expose it. `Connected` carries no client GUID. There are no per-peer stats (resends, queue length) and no `Server::peers()`. | `conn.rs:50-52`, `server.rs:37-42`, `offline.rs:158-165` (the GUID is parsed, then discarded at `server.rs:148`) | **A**: `rtt(peer)`, `Connected { addr, guid, mtu }`, and a stats struct. |
| R9 | Send is fixed to ordering channel 0. | `conn.rs:62-65` | Fine for Bedrock; nothing to do. |
| R10 | There is no MOTD builder. The pong is a raw string; mitm's fallback shows the format. | `acacia-mitm/src/raknet.rs:171` | **S**, or a small **A** helper `Motd { .. }.to_string()`. |
| R11 | `close()` emits no `Disconnected` event. That is documented, but the driver must remember it. | `server.rs:79-84` | Doc only. |

**Verdict:** the protocol behaviour is correct and fine for tens of LAN players. It is **not fit for hundreds of
public players**: it has a remote crash (R1), trivially spoofable resets and memory blow-up (R2, R3), and no congestion control (R4). It is close to quadratic when draining output (R6). R1–R3 are small patches. R4 and R6 are the real work.
BDS 26.60 drops RakNet (DESIGN.md:16,97), but Geyser and most third-party clients and proxies still speak it, so it remains
the primary transport for a while.

---

## 2. Session server side (`acacia-session/src/server.rs`)

### What exists
- `ServerConnection` holds a `BatchCodec` plus *one* pending codec switch, applied after the batch that announced it (`server.rs:16-54`).
- `start_compression(&NetworkSettings)` (`:57-61`).
- `start_encryption(client_key)` builds the ES384 ServerToClientHandshake, salt and derived key (`:64-68`; `acacia-auth/src/login/handshake.rs:26-31`).
- `plaintext_handshake` for NetherNet (`:72-74`).
- `read_login` splits the Login packet and extracts the client-data JWT's x5u (`:84-91`).
- `BatchCodec` frames 0xfe batches. It handles deflate/snappy with a per-batch `0xff` "uncompressed" id, a 16 MiB decompress cap, and AES-256-CTR plus the SHA-256 checksum (`batch.rs`, `compression.rs`, `crypto.rs`). This matches the gophertunnel wire format exactly.

Users: `acacia-testserver/src/peer.rs:76-112` (the full but scripted handshake) and `acacia-mitm/src/relay.rs:59,140,197-203`.

### Gaps
| # | Gap | Evidence | Fix |
|---|---|---|---|
| S1 | **No server login state machine.** Each user re-implements the handshake inline (testserver, mitm). Nothing checks the protocol in RequestNetworkSettings, sends PlayStatus LoginFailedClient/Server, enforces packet order or a login deadline, or handles ClientToServerHandshake → PlayStatus(LoginSuccess). | `testserver/src/peer.rs:76-98` | **A**: a network-free `server::Handshake` mirroring the client `Session`. It should drive packets through LoginSuccess, then hand over a `ServerConnection` plus a `VerifiedLogin`. Order: RequestNetworkSettings(protocol) → NetworkSettings (uncompressed; compression on after) → Login → verify (§3) → S2C handshake (RakNet: encrypt after; NetherNet: plaintext, DESIGN.md:102) → ClientToServerHandshake → PlayStatus(LoginSuccess). The client then sends ClientCacheStatus (gophertunnel `conn.go:900-931`). |
| S2 | **No resource-pack serving.** acacia only has the client side (`session/packs.rs`, `pack_store.rs`). | — | **S** for the negotiation policy (ResourcePacksInfo → ClientResponse → DataInfo / ChunkRequest / ChunkData → Stack → Completed). Possibly **A** for a sans-IO pack chunker, since it is generic. |
| S3 | **`Login` is not verified at all.** `client_public_key` decodes the client-data JWT *without verifying its signature* and trusts its x5u. | `acacia-auth/src/login/handshake.rs:34-39` (`jwt::decode`, not `verify`) | **A**: see §3. |
| S4 | **One pending switch slot.** Calling `start_compression` then `start_encryption` before an `encode` silently drops the first. | `server.rs:19,47-53` | **A**: use a queue, or assert. |
| S5 | **No "encode once, send to many".** Every player's broadcast is re-compressed: `encode` compresses then encrypts in one call. | `batch.rs:44-69` | **A**: split it into `compress_payload(&[packets]) -> Compressed` plus `seal(Compressed)` per connection. Encryption must stay per-connection; compression can be shared. This is the biggest CPU win for hundreds of players. |
| S6 | **No settings-level "no compression".** `NetworkSettings.compression_algorithm = 0xffff` (none) maps to an error. | `compression.rs:29-35` | **A**: an `Algorithm::None` variant. |
| S7 | **Every batch is copied.** `decode` does `body.to_vec()` on each batch. | `batch.rs:84` | Minor; decrypt in place from `BytesMut`. |
| S8 | The client `Session` (Android timing, deferred replies, blob reporting) ships in the same crate. | `session/mod.rs`, `session/deferred.rs` | Harmless (it compiles but is never called). It could move behind a `client` feature later. |

---

## 3. Login verification

### Today
- acacia only *builds* Logins: the token-only envelope, the legacy chain and the self-signed offline form (`acacia-auth/src/login/request.rs:75-147`).
- On the server side, the only parsing is mitm's *unverified* identity extraction. It reads `xname/xid/leguuid` from the token or `displayName/XUID/identity` from the chain's extraData (`acacia-mitm/src/login.rs:36-51`).
- `jwt::Unverified::verify` supports **ES384 only** (`acacia-auth/src/jwt.rs:65-74`). The multiplayer token is **RS256**.
- `ClientData` derives `Deserialize`, but every field is mandatory, with no `#[serde(default)]` (`acacia-auth/src/login/client_data.rs:8-50`). It models the Android 1.26.52 key set, so logins from other platforms or versions may fail to deserialize. Check `login/skin.rs` for the same issue.

### Required algorithm (sources: gophertunnel `protocol/login/request.go`, `service/{token,jwks,discovery}.go`)
1. **Envelope.** Use the `LEInt32 len ‖ auth JSON ‖ LEInt32 len ‖ client-data JWT` framing, and treat trailing bytes as an error.
   - The auth JSON is `{"AuthenticationType":u8,"Certificate":"<JSON string {\"chain\":[..]}>","Token":"<JWT>"}`. `Certificate` may be `""` or absent; the vanilla client sends a token-only envelope.
   - Legacy top-level `{"chain":[..]}` is also accepted.
   - `AuthenticationType` values:
     - 0 = FULL;
     - 1 = GUEST (split-screen; gophertunnel rejects it);
     - 2 = SELF_SIGNED (offline).
2. **Token path (FULL, current vanilla).**
   - Discovery: `GET https://client.discovery.minecraft-services.net/api/v1.0/discovery/MinecraftPE/builds/<ver>` → `result.serviceEnvironments.auth.prod.{serviceUri,issuer}` (today `https://authorization.franchise.minecraft-services.net`).
   - OpenID config: `{issuer}/.well-known/openid-configuration` → `issuer = "https://authorization.franchise.minecraft-services.net/"` (**trailing slash**), `jwks_uri = …/.well-known/keys`, algs `["RS256"]`.
   - Verify RS256 with the JWKS key whose `kid` matches the JWT header. Quirk: the JWKS `x5t` is 40-character hex, not base64url (gophertunnel `jwks.go:204-259`). Ignore it or convert it.
   - Checks:
     - `iss` equals the issuer exactly;
     - `aud` contains `api://auth-minecraft-services/multiplayer` (already `MULTIPLAYER_AUDIENCE`, `request.go:14`);
     - `exp` and `nbf` hold, with about 60 s of leeway.
   - Claims: `xid` (XUID), `xname` (gamertag), `cpk` (base64 DER P-384, **the client's identity key**), `mid`, `tid`, `ipt`.
   - Authenticated means the token is valid and `xid` is non-empty.
   - The identity UUID is a v3-style MD5 of `"pocket-auth-1-xuid:" + xid` (gophertunnel `request.go:475-487`; Cloudburst and PocketMine agree).
   - JWKS caching: re-fetch on an unknown `kid`, at most once per 30 min, single-flight (gophertunnel `jwks.go:84-104`). Discovery and the OpenID config are cached for the process lifetime.
3. **Self-signed path (type 2, offline).** The ES384 token is signed by `cpk` itself, with `aud` as above, `xid` empty, `xname`, and `leguuid` (must be a valid UUID). It is not authenticated.
4. **Legacy chain (fallback / titleId only).**
   - Mojang root key, P-384 SPKI b64, rotated Aug 2023: `MHYwEAYHKoZIzj0CAQYFK4EEACIDYgAECRXueJeTDqNRRgJi/vlRufByu/2G0i2Ebt6YMar5QX/R0DIIyrJMcUpruK4QveTfJSTp3Shlq4Gk34cD/4GUWwkv0DVuzeuB+tXija7HBxii03NHDbPAD0AKnLr2wdAp`.
   - Walk the chain:
     - `key = chain[0].x5u`;
     - verify each JWT with `key`, then set `key = identityPublicKey`.
   - A chain of length 3 is authenticated if `chain[0].identityPublicKey == Mojang`, and `chain[1]` and `chain[2]` have `iss == "Mojang"`.
   - `extraData = {XUID, identity, displayName, titleId}`. Require `authenticated == (XUID != "")`.
   - A chain of length 1 is offline.
   - When the token and chain agree on XUID, gophertunnel copies `titleId` from the chain.
5. **Client data.**
   - Verify the client-data JWT (ES384, raw r‖s) against the **final key**: `cpk` on the token path, or the last `identityPublicKey` on the chain path. PocketMine also requires the header x5u to equal that key.
   - That key is also the ECDH peer for `start_encryption`.
   - Validate the fields the way gophertunnel `ClientData.Validate` does (`data.go:342-371`):
     - DeviceOS is 1..15;
     - GameVersion contains only digits and dots;
     - PlatformOfflineId and SelfSignedId are UUIDs or empty;
     - ServerAddress is valid.
6. **Server policy (S).**
   - Disconnect unauthenticated clients when online-mode is on.
   - Handle duplicate XUIDs (kick old or refuse new).
   - Enforce a login deadline.
   - NetherNet: the key proven in the SDP `a=identity` must equal the Login key (gophertunnel `conn.go:851-861`).

### Recommendation
- **A: `acacia-auth::login::verify`**, network-free.
  - `verify_login(request, &Verifier, now) -> VerifiedLogin { identity: {xuid, name, uuid, title_id?}, authenticated, client_key, client_data: serde_json::Value + lenient typed view }`.
  - `Verifier` holds the issuer, audience and a JWKS snapshot. Add RS256 support (the `rsa` crate, pure Rust) next to ES384 in `jwt.rs`.
  - Move mitm's identity extraction (`acacia-mitm/src/login.rs:43-51`) onto it, so mitm keeps working.
- **A, behind `online`:** a small `JwksCache` fetcher (reqwest exists under that feature). Alternatively **S** fetches the JWKS itself and passes in the keys, which keeps acacia-auth network-free for sapling. Prefer the latter: sapling can then pin, refresh and inject.
- **A:** make `ClientData`/`Skin` deserialization lenient with `#[serde(default)]` on everything not universally present, or add a separate server-side `ClientDataView`.

---

## 4. NetherNet server side (`acacia-nethernet`)

### What exists
- `Connection::answer(offer, local, key, now)` produces a BDS-style answer with one host candidate, the DTLS client role, and an `a=identity` signed under domain `self` (`conn/answer.rs:17-34`; `server_identity.rs:89-103`).
- The direct-connect HTTP pieces for the server side exist (`http.rs`):
  - `request_complete`, `parse_request`, `response`;
  - the TLS-first refusal `TLS_REFUSAL` and `is_tls`, since vanilla tries HTTPS first (`:39-82`).
- LAN discovery codec both ways: `LanPacket::encode/decode` and `ServerData::encode/decode` (`lan/packet.rs:42,69`; `lan/server_data.rs:28,55`).
- The signaling-service session is sans-IO and symmetric (`send_signal` / `Signal` events), but written as a *client* (`signaling/mod.rs`).
- mitm already serves BDS-style signaling to the real game (DESIGN.md:59, `acacia-mitm/src/nethernet/`), which proves `answer` works against vanilla.

### Gaps
| # | Gap | Evidence | Fix |
|---|---|---|---|
| N1 | **The client's identity is never checked.** "The client's own assertion is not checked, so this host admits anyone." The offer's `a=identity` holds a MultiplayerToken (RS256, §3) plus a detached ES384 JWS over the DTLS fingerprints by `cpk`. `verify_answer` is `pub(crate)`, expects a *self-signed* ES384 `cpk` token, and so cannot verify a client offer. | `conn/answer.rs:15-16`, `server_identity.rs:20-42` | **A**: `verify_offer(sdp, &Verifier) -> client_key`, reusing §3's token verifier and the fingerprint JWS check. Then bind `client_key` to the Login key (§3.6). |
| N2 | **Many connections on one UDP socket.** Each `answer` uses `local` as the host candidate. A server with one socket must route datagrams to the right `Connection`. Nothing exposes the local ICE ufrag or an `accepts(datagram)` helper. str0m has `Rtc::accepts`, and `ufrag` is a private field. | `conn/mod.rs:52,187-192` | **A**: `Connection::accepts(source, data)` or `local_ufrag()`. **S**: a demux map from remote addr to conn, filled from the STUN USERNAME on the first binding request. |
| N3 | **No trickle answer** for signaling-service or LAN hosting. `answer` embeds one host candidate; it cannot answer a `CONNECTREQUEST` with `CONNECTRESPONSE` + `CANDIDATEADD`, or use a TURN relay as the answerer. | `conn/answer.rs`, `conn/candidates.rs:23-54` (offer side) | **A** if sapling wants Realms-like or friends-list hosting or LAN worlds: `Connection::answer_trickle(offer, identity, now)` plus `add_local_candidate`. Low priority for a dedicated server. |
| N4 | **No HTTP listener or I/O.** The pieces exist, but the TCP accept loop, keep-alive and 1 MiB cap (`http.rs:8`) are the caller's job. | — | **S** (tokio). It's about 100 lines; copy the shape from `acacia-mitm/src/nethernet/`. |
| N5 | **No LAN advertiser loop.** Codecs exist; the responder loop on UDP 7551 and the signal relay over LAN `Message` packets don't. | `lan/mod.rs:10` | **S**. It needs N3 for the actual join. |
| N6 | **Frames are capped at 128 KiB** because of str0m buffering. Large batches (chunks, packs) fragment, which is already handled. | DESIGN.md:105, `frame.rs` | Nothing to do; just be aware. |
| N7 | **A DTLS certificate per connection** (a P-256 keygen plus rcgen) is fine for bots and costs a few ms per join on a server. | `cert.rs` | **A** (optional): allow reusing a server certificate. |

**What vanilla clients need:**
- (a) For "Add Server" by IP: a RakNet ping, *or* NetherNet direct connect, meaning TCP `GET /v1/join` + `POST /v1/join/{id}` on the game port (DESIGN.md:16). N1 (if online-mode), N2 and N4 are the blockers; N3 and N5 only matter for LAN, Realms or friends hosting.
- (b) BDS requires a real MultiplayerToken in the offer even offline (DESIGN.md:100-101). A sapling server should *accept* a missing or self-signed one when offline.

---

## 5. Protocol (`acacia-proto`)

### What exists
- All 246 packets implement `Packet` with both `encode` and `decode` (the trait requires both; `lib.rs:28-35`).
- 246 `.hex` fixtures round-trip byte-exactly, and `decode(encode(x)) == x` holds (`tests/fixtures.rs:58-81,107-128`).
- `CLIENT_PACKETS` asserts only a client subset by name (`:10-45`), but the all-fixtures test covers every file. **Serverbound decode and clientbound encode are therefore covered.**
- Caveat: fixtures come from the same minecraft-data schema, so schema errors pass (`docs/proto.md:18-19`). Two overrides already exist (`proto.md:12-16`).

### Gaps (helpers a server needs)
| # | Gap | Evidence | Fix |
|---|---|---|---|
| P1 | **No `Default` on structs.** `StartGame` alone has about 80 fields. | `generated/packets/part_00.rs:1267-1346`, `proto.md:83` | **A**: codegen emits `Default` (numbers 0, empty strings and vecs, open enums' first or `Unknown(0)`), or **S** keeps a `StartGameBuilder` with vanilla-correct defaults. Prefer codegen `Default`, with **S** owning the semantic defaults. |
| P2 | **AvailableCommands must be built by hand.** `values_len` is a separate count field that controls the enum index width; enum and suffix dedup and the chained-subcommand tables are manual. | `part_03.rs:9-19,175` | **S** (command system), or **A** a small `CommandsBuilder` since it is pure encoding. |
| P3 | **The block palette exists for vanilla only.** Runtime id = index in the FNV-1-64-by-name sorted palette; `network_hash` is precomputed for vanilla states (`acacia-world/src/registry/mod.rs:1-5,54-61,154-159`; `state.rs:68-70`). States carry properties as a `"k=v,.."` string with no NBT types, and custom blocks get `network_hash: 0` (`mod.rs:126-140`). There is no `block_pallette_checksum`, and no way to build `StartGame.block_properties` NBT. | — | **A**: keep typed NBT states (or a `(name, Nbt)` side table) in `blocks.bin`, and a `network_hash(name, &Nbt)` function (FNV-1a-32 of LE NBT). **S**: custom-block definitions. |
| P4 | **No item registry, creative content, biome definitions, crafting data or entity identifiers.** No data assets exist in acacia at all; the only data is `acacia-world/data/blocks.bin`. | Glob of `crates/**` | **S**: vendor generated data (e.g. from BDS dumps or a license-clean dataset; check the license, since DESIGN.md:27 bans porting GPL/LGPL code) and encode it via the generated packet structs. |
| P5 | **The shield item id is process-global.** Item encoding and decoding reads `shield_item_id()`. | `acacia-proto/src/manual/mod.rs:47-60`, DESIGN.md:112-113, `proto.md:67,81` | A server with one item registry just calls `set_shield_item_id` once at boot. It only breaks with multiple registries or versions per process; see §7. |
| P6 | **No sub-client support in encoding.** `encode_packet` writes sender and target sub-client as 0. | `lib.rs:77-81` | **A** if split-screen is ever wanted. Low priority. |

---

## 6. Chunk encoding

### What exists
- Decode only: `Chunk::decode`/`decode_mapped`, `set_sub_chunk`, `set_biomes` (`acacia-world/src/chunk/mod.rs:71-124`).
- Paletted `Storage` is kept **packed in wire layout** (`chunk/storage.rs:1-17,32-60`), so an encoder is trivial.
- `xxh64` blob ids exist (`acacia-session/src/blob_store.rs:65`).
- Grepping for any chunk `encode` or `heightmap` in `acacia-world` and `acacia-session` finds nothing. The testserver answers SubChunkRequest with `SuccessAllAir` only (`acacia-testserver/src/peer.rs:99-103,175-188`).
- Storage keeps layers 0 and 1 only, and skips border blocks and block entities (`chunk/mod.rs:1-3,209-221`).

### Missing (all **A**, network-free, natural home `acacia-world::chunk::encode`)
1. `Storage::encode`:
   - header `(bits<<1)|1`;
   - LE u32 words;
   - zigzag varint palette length, then zigzag varint ids;
   - bits 0 means a single value.
   `Storage::set` only ever grows the palette (`storage.rs:94-115`), so add **palette compaction** before encoding.
2. Section encoding in version 9: `9, layer count, y index`, then storages.
3. Biome encoding: per-section storages, with `0xff` = copy the section below (`chunk/biomes.rs:10`).
4. `LevelChunk` payload: sections, biomes, border-blocks byte (0), then concatenated block-entity NBT.
   - Plus the request-mode variants:
     - `sub_chunk_count = u32::MAX` (limitless);
     - `u32::MAX-1` plus `highest_subchunk_count` (limited);
     - in both modes the payload is biomes only.
   - Struct: `generated/packets/part_02.rs:1292-1301`.
5. `SubChunk` response entries: result codes, payload, `heightmap_type` / `heightmap` (16×16 i8 relative to the sub-chunk, plus TooHigh / TooLow), and `render_heightmap`. The field set is visible in `testserver/src/peer.rs:175-188`. A heightmap computation is needed.
6. **Server-side blob cache:**
   - Honour `ClientCacheStatus`.
   - Emit `blobs: Vec<u64>` (xxh64 per section plus the biome blob) in LevelChunk/SubChunk.
   - Track what each client has.
   - Answer `ClientCacheBlobStatus` with `ClientCacheMissResponse`.
   - Client-side reference behaviour: `session/blob_cache.rs`, docs `research/blob-cache.md`. The per-client tracking is **S**; hashing and building the blob list are **A**.
7. `NetworkChunkPublisherUpdate` and chunk-radius logic: **S**.

`acacia-world::World` (`world.rs`) is a bot-shared chunk map with per-view owners. It is not a server world store. Sapling's LevelDB/CoW world (memory: sapling-decisions) should own storage and only borrow `Chunk`/`Storage` and the encoders.

---

## 7. Crate-boundary issues (client assumptions in shared crates)

| Issue | Evidence | Impact on sapling | Fix |
|---|---|---|---|
| The shield item id is a process-wide `AtomicI32`. | `acacia-proto/src/manual/mod.rs:51` | Fine for one registry per process. It breaks with a proxy or multi-version setup, or if acacia's client session runs in the same process and calls `set_shield_item_id` from a remote server's registry (DESIGN.md:112). | **A** (later): a per-codec context, e.g. `decode_with(&ItemCtx)`. Until then, never run acacia-bot and sapling in one process. |
| `ClientData` mirrors the Android key set with every field required. | `acacia-auth/src/login/client_data.rs:8-50,62-96` | Other platforms or versions may fail to deserialize. | **A**: lenient deserialize (§3). |
| `jwt` handles ES384 only. | `acacia-auth/src/jwt.rs:65-74` | Cannot verify multiplayer tokens. | **A**: RS256 (§3). |
| `client_public_key` trusts an unverified x5u. | `acacia-auth/src/login/handshake.rs:34-39` | It is security-relevant if anyone mistakes it for verification. | **A**: rename it or document it as `unverified_client_key`, and add `verify_login`. |
| The raknet `Server` and `Conn` are tuned for a bot: 10 ms ACK tick, no congestion control, no cookie. | §1 | See R1–R7. | **A**. |
| Session client logic (Android timing, deferred replies, blob reporting) is compiled into `acacia-session`. | `session/deferred.rs`, `session/blob_cache.rs` | Dead weight only. | Optional `client` feature. |
| `acacia-auth` defaults to the `online` feature (reqwest + tokio). | `acacia-auth/Cargo.toml` `[features] default=["online"]` | The workspace pins `default-features = false` (root `Cargo.toml:14`), but sapling's own manifest must also set it unless it wants reqwest. | **S**: depend with `default-features = false`. |
| `BlockRegistry` custom names are interned and leaked per process. | `acacia-world/src/registry/mod.rs:142-152` | Bounded by the number of distinct names, so fine. | — |
| `acacia-raknet` dev-depends on tokio and `acacia-nethernet` pulls str0m, rcgen and aws-lc-rs. | `acacia-raknet/Cargo.toml:15-16`; `acacia-nethernet/Cargo.toml` | aws-lc-rs needs a C toolchain or prebuilt binaries on some targets. Note it for CI. | — |

The Android fingerprint assumptions (device ids, skins, input modes, delays) all live in `acacia-auth/src/login/{device,skin,client_data}.rs` and `acacia-session/src/session/*`. Sapling never calls the builders, so they don't leak into server behaviour beyond the strict `ClientData` deserialization.

---

## 8. Recommendation summary

### Add to acacia (generic, network-free; roughly in priority order)
1. RakNet hardening:
   - **R1** MTU clamp;
   - **R3** reassembly budget;
   - **R2** cookie plus ALREADY_CONNECTED;
   - **R7** max peers and handshake timeout;
   - **R8** rtt, guid and stats.
2. `acacia-auth::login::verify` (§3):
   - token (RS256 + JWKS snapshot), self-signed, and legacy-chain paths;
   - client-data verification against the final key;
   - lenient `ClientData`.
   Then move mitm's identity extraction onto it.
3. `acacia-session::server::Handshake`, a sans-IO login state machine through LoginSuccess (S1). Also:
   - the switch queue (S4);
   - shared compression for broadcasts (S5);
   - `Algorithm::None` (S6).
4. RakNet congestion control and pacing (R4), RTO backoff (R5), and O(1) ready/timer structures (R6).
5. `acacia-world` chunk encoders (§6.1–6.5): palette compaction, typed NBT block states, and a hash function for custom blocks (P3).
6. NetherNet server pieces:
   - `verify_offer` (N1);
   - `Connection::accepts` / `local_ufrag` (N2);
   - later, `answer_trickle` (N3).
7. Codegen `Default` derive (P1).
8. Later: a per-connection shield-id / item context (§7).

### Sapling owns
- All tokio I/O:
  - UDP sharding across cores (R6);
  - the NetherNet HTTP listener (N4);
  - the LAN advertiser (N5);
  - JWKS fetch and refresh, injected into the verifier.
- Policy:
  - online-mode, bans, per-IP limits;
  - duplicate-XUID handling, login deadline;
  - MOTD content.
- Resource-pack negotiation and serving (S2).
- Game data: item registry, creative content, biomes, crafting, entity identifiers, custom blocks (P4), plus the semantic StartGame defaults and the AvailableCommands construction (P2).
- The world store (LevelDB/CoW), chunk sending and radius, per-client blob-cache tracking (§6.6-6.7), and heightmap source data.
