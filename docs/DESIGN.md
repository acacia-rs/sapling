# sapling — design

A Rust Bedrock server framework for hosting any kind of server: hubs and minigames first, SMP and
vanilla features as later layers. Built on acacia's protocol, transport, session and auth crates.
Evidence: `research/dragonfly.md`, `research/acacia-server-gaps.md`, `research/worlds-and-ecs.md`.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Purpose | Library framework; servers are binaries that add plugins | Hubs, minigames, SMP and networks are all "a server plus plugins" |
| Versions | Latest protocol only (acacia's) | Same rule as acacia; old clients via a proxy later |
| Threads | One **shard** = one OS thread with its own `bevy_ecs::World` + `Schedule` (no `bevy_app`). Many shards, created and destroyed at runtime | Cores scale with worlds; no locks in game code; no fixed world count (dragonfly has 3) |
| Worlds | A world is a **layer entity** inside a shard (Valence model). A shard holds many worlds (e.g. 50 arenas); a player points at its layer by component | Switching world inside a shard is a component write; arenas are cheap |
| Cross-shard | Message passing only. Player transfer = serialise, two-phase handoff over a channel, a directory forwards messages meanwhile | dragonfly's cond-var transfers are its main deadlock source (#966, #981, #990, #1448) |
| Plugin API | Handler traits (`on_join`, `on_chat`, `on_break`, …) with default no-ops, mutable event params, `ctx.cancel()`; **many handlers with priorities**; handlers get a borrowed `ShardCtx` facade, never bevy types. Advanced plugins may add systems/components | dragonfly allows one handler and refuses plugins (#400, #314); Chorus/Valence leak bevy into the public API and got stuck on old bevy |
| Plugins | Rust crates compiled in. Facade types stay plain data so a WASM host can mirror them later | Speed and types now, sandboxing later |
| Inbound packets | Network threads decode and push into per-shard queues; the shard drains them at tick start | dragonfly blocks one world round-trip per packet |
| Outbound | Never blocks the shard. Broadcasts encoded and compressed once per layer, encrypted per player; movement batched per tick | dragonfly re-encodes per viewer and blocks the world on a full queue |
| Storage | `WorldStorage` trait. Backends: vanilla LevelDB (rusty-leveldb, bedrock-crustaceans fork, pinned) and in-memory. All disk I/O on an I/O pool, one write batch per chunk | Build maps in vanilla, host them, take SMP worlds back to vanilla |
| Arenas | Immutable `TemplateWorld` (chunk → `Arc` sections) + `ArenaWorld` holding only changed chunks. Clone O(1), first write copies one section, reset = drop the diff | Nobody does real CoW (Minestom/Valence/Chorus deep-copy) |
| Chunk sending | Sub-chunk request mode + blob cache. Encoded section payloads cached per template section, shared by all arenas until changed | One encode per map, not per player per arena; client blob cache hits across arenas |
| Lighting | Not stored | The Bedrock client computes its own |
| Commands / forms | `#[derive(Command)]` / `#[derive(Form)]` macros (dragonfly's struct model without reflection) | Typed and checked at compile time |
| Tests | Synchronous shard mode (manual tick) + acacia-bot as the test client | Deterministic; acacia already drives a full client |
| Licence | MIT. Port dragonfly (MIT, keep notice); block upgrade data from pmmp schema repo (CC0). Never read GPL code (PocketMine, PowerNukkitX, Pumpkin, AdvancedSlimePaper) | Same rule as acacia |

## Crates (dependencies point downward)

```
sapling                 umbrella: Server builder, re-exports
 ├─ sapling-api         facade: handler traits, events, ShardCtx, Player/World handles (no bevy types)
 │   └─ sapling-macros  #[derive(Command)], #[derive(Form)]
 ├─ sapling-shard       shard thread: bevy World/Schedule, layers, players/entities, viewers, broadcast buffers,
 │                      inbox/outbox, directory + handoff
 ├─ sapling-net         tokio I/O: RakNet socket sharding, NetherNet HTTP listener, LAN advert, login state
 │                      driver, signing-key fetch, bans/rate limits/duplicate-UUID reservation
 ├─ sapling-world       sections/palettes, Template/Arena CoW, network encode cache, WorldStorage trait
 │   └─ sapling-leveldb vanilla keys, level.dat, sub-chunk codec, block-state upgrader
 └─ sapling-data        block/item/biome/creative/command data and StartGame defaults
acacia-{proto,raknet,nethernet,session,auth}  (git deps; acacia-auth with default-features = false)
```

## Upstream work in acacia (before or alongside milestone 1)

1. RakNet hardening: MTU floor (remote panic), reliability allocation cap, security cookie, max peers and
   handshake timeout, expose RTT/GUID. Then congestion control and per-peer send scheduling.
2. Network-free `acacia-auth::login::verify`: multiplayer token (RS256, issuer
   `https://authorization.franchise.minecraft-services.net/`, audience `api://auth-minecraft-services/multiplayer`,
   `cpk` = client key), client-data signature, legacy Mojang chain, offline self-signed. Lenient `ClientData`.
3. Server login state machine in acacia-session, plus compress-once/encrypt-per-player and a "none" compression.
4. Chunk encoders in acacia-world: palette compaction, v9 sections, biomes, LevelChunk/SubChunk, heightmaps.
5. NetherNet: verify the client offer identity; per-connection routing on a shared socket.

## Milestone 1: hub server

Each step ends with an acacia-bot test and a vanilla client check.

1. **Join**: listener, verified online login, StartGame from defaults, spawn in a void world.
2. **Map**: load a vanilla world from LevelDB, send chunks (request mode, blob cache, shared encode cache).
3. **Players**: players as shard entities, see each other move (per-tick batch), PlayerList, chat.
4. **API**: handler traits with priorities, commands (derive), forms, scoreboard/sidebar, bossbar, titles.
5. **Worlds**: several worlds per shard, several shards, `/transfer`-style world moves (handoff + directory).
6. **Protection**: cancel break/place/interact with client resync (dragonfly #749 is the trap).

## Step 1 as built (Join)

One listener task, one shard thread, one world. The pieces and why they are shaped this way:

- **Network to game.** `sapling-net` owns everything up to PlayStatus(LoginSuccess): RakNet, the login state
  machine with a deadline, the server-full check and the identity reservation. It then reports `Joined`,
  `Packet`s (header parsed, body undecoded) and exactly one `Left` per connection, whoever ended it. The
  game never guesses whether a player is still connected: it despawns on `Left` only.
- **Game to network.** `Outbox::send` queues one batch per player per tick and never blocks. Backpressure
  is per player: a connection whose RakNet backlog passes `NetConfig::max_queued_bytes` is kicked.
- **Shutdown.** The shard owns the channels. Stopping it drops them, and the listener then tells every
  player the server closed and exits.
- **Join sequence** (`sapling-shard/src/join.rs`, stages in `components.rs`), BDS's order and gates as
  recorded in acacia-testserver (`cargo run -p extract-data -- --list`):

  | Server sends | then waits for |
  |---|---|
  | ResourcePacksInfo (no packs) | ResourcePackClientResponse(HaveAllPacks) |
  | ResourcePackStack | ResourcePackClientResponse(Completed) |
  | StartGame, ItemRegistry, BiomeDefinitionList, AvailableEntityIdentifiers, CreativeContent, UpdateAbilities, SetEntityData, UpdateAttributes | RequestChunkRadius |
  | ChunkRadiusUpdate, NetworkChunkPublisherUpdate, a LevelChunk per column in view, PlayStatus(PlayerSpawn) | SetLocalPlayerAsInitialized |
  | — the player is in the world: `on_join` | |

  BDS starts sending chunks before the radius request arrives; sapling waits for it, so the first
  publisher update already has the client's radius.
- **Chunks.** Request mode without the blob cache: a LevelChunk carries biomes and the section count,
  `SubchunkRequest`s are answered from the world's per-section encode cache. Unwritten columns share one
  void column. Block ids on the wire are state hashes (`sapling_world::NETWORK_IDS_ARE_HASHES`), as BDS
  sends them and as the recorded StartGame announces.
- **Packets inside a tick.** The inbox is drained into queues by packet id; each system takes the types it
  handles. Order is kept within a type, not across types.
- **Handlers** run after the tick's systems, outside the schedule, with a `ShardCtx` over a `Backend`
  trait object, so `sapling-api` depends on nothing but `uuid`.
- **Game data** is BDS's own packets, patched (`crates/sapling-data/README.md`).

## Step 2 as built (Map)

- **Disk.** `sapling-leveldb` opens a world folder (`level.dat` + `db`) through the pinned rusty-leveldb
  fork with compression ids 0, 2 (zlib) and 4 (raw deflate, used for writes). `LevelDbWorld::load_column`
  decodes sub-chunk records (v1/8/9) into `acacia_world::Chunk` sections and re-encodes Data3D biomes into
  the network form the chunk already reads; acacia-world is unchanged. `open` works in place, as LevelDB
  does (log replay rewrites files); `open_in_memory` copies the folder into memory and never touches it
  (tests, read-only maps). `Db` writes (`put`, `Batch`, `flush`); no chunk encoder for disk yet.
- **Block states.** Palette entries resolve by name + states against acacia-world's registry, cached by
  their raw bytes. Unknown states of a known block become its first state, unknown blocks become
  `minecraft:info_update`; each is logged once.
- **Storage.** `WorldStorage::load_column` (blocking) with `LevelDbStorage` and `MemoryStorage` (empty =
  void). A storage-backed `World` owns two I/O threads: `request(pos)` queues a load, the thread reads,
  builds the column and encodes all of it (`Column::warm`), `poll()` takes finished columns once a tick.
  The shard only does `try_recv`. Columns storage lacks are void. A sweep every 100 ticks drops unmodified
  columns outside every view.
- **Sending.** A `View` keeps a nearest-first queue; each tick a player gets at most
  `ShardConfig::columns_per_tick` (8) columns that are loaded, and only the nearest `2 x` that many are
  asked of storage. `NetworkChunkPublisherUpdate` goes out when the player's chunk changes.
  PlayStatus(PlayerSpawn) follows once the columns within 3 chunks are sent (BDS: 26 columns).
  Sub-chunk requests for a column not loaded answer `ChunkNotFound`.
- **Blob cache.** `ClientCacheStatus` turns it on per player (and `ShardConfig::blob_cache`). Each
  section is encoded once per column with its xxh64 and shared by all players, cached or not: `LevelChunk`
  names the biome blob, `Subchunk` entries name section blobs with an empty payload. A player holds
  references to what it was told about until its `ClientCacheBlobStatus` answers; every status gets one
  `ClientCacheMissResponse`. `missing` is served before `have` is dropped: a hash shared by several
  columns arrives as missing once and under `have` for the other slots of the same status.
- **Hub.** `hub [port] [world folder] [--offline]`; spawn from `level.dat`, on the surface (highest block
  with collision or liquid) when SpawnY is 32767.
- **Verified against BDS 1.26.52.3 saves** (`crates/sapling-leveldb/tests/fixtures`): chunk version 42;
  v9 sub-chunk Y is the section index of the key; the Data3D
  heightmap is the top light-dimming block + 1 above the floor, the rule acacia-world's heightmap uses.

## Open items

- **BDS only saves chunks that changed since generation.** A world folder has holes wherever nothing was
  touched; vanilla regenerates them from the seed, sapling shows void. Maps must be fully saved (or
  sapling needs a generator).
- Block-state upgrader (CC0 BedrockBlockUpgradeSchema) for palettes older than the current version;
  until then see "Block states" above. Legacy sub-chunk versions (< 8, except 1) are skipped as air.
- Block entities are read from disk (`DiskColumn::block_entities`) but not sent: chests, signs and the
  like show without their data.
- Saving: `WorldStorage::store_column`, the disk sub-chunk encoder, digp/actorprefix. Written columns are
  never unloaded meanwhile.
- Only the overworld is opened; nether/end keys parse but no world is made for them.
- LevelDB block compression id 2 is zlib in worlds; never enable zstd (Mojang master reuses id 2).
- Game data source and licence (item/creative/biome/recipes) for sapling-data.
- acacia-proto's shield item id is process-global: never run acacia-bot and sapling in one process.
