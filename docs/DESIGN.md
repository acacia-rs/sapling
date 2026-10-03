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

## Open items

- Verify against a fresh BDS 26.5x world: chunk version byte, v9 sub-chunk Y semantics.
- LevelDB block compression id 2 is zlib in worlds; never enable zstd (Mojang master reuses id 2).
- Game data source and licence (item/creative/biome/recipes) for sapling-data.
- acacia-proto's shield item id is process-global: never run acacia-bot and sapling in one process.
