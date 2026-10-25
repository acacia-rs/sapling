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
| Plugin API | `Plugin::build` per shard registers **typed listeners** (`on::<BreakEvent>(priority, ..)`) and tasks, scoped to the shard, a world or a player; mutable events with `cancel()`; the `Handler` trait (`on_join`, `on_chat`, …) is filed as listeners; all get a borrowed `ShardCtx` facade, never bevy types | dragonfly allows one handler and refuses plugins (#400, #314); Chorus/Valence leak bevy into the public API and got stuck on old bevy |
| Plugins | Rust crates compiled in. Facade types stay plain data so a WASM host can mirror them later | Speed and types now, sandboxing later |
| Inbound packets | Network threads decode and push into per-shard queues; the shard drains them at tick start | dragonfly blocks one world round-trip per packet |
| Outbound | Never blocks the shard. Broadcasts encoded and compressed once per layer, encrypted per player; movement batched per tick | dragonfly re-encodes per viewer and blocks the world on a full queue |
| Storage | `WorldStorage` trait. Backends: vanilla LevelDB (rusty-leveldb, bedrock-crustaceans fork, pinned) and in-memory. All disk I/O on an I/O pool, one write batch per chunk | Build maps in vanilla, host them, take SMP worlds back to vanilla |
| Arenas | Immutable `TemplateWorld` (chunk → `Arc` sections) + `ArenaWorld` holding only changed chunks. Clone O(1), first write copies one section, reset = drop the diff | Nobody does real CoW (Minestom/Valence/Chorus deep-copy) |
| Chunk sending | Sub-chunk request mode + blob cache. Encoded section payloads cached per template section, shared by all arenas until changed | One encode per map, not per player per arena; client blob cache hits across arenas |
| Lighting | Reckoned on the server for its rules only ("Light (as built)"); never sent or saved | The Bedrock client computes its own; BDS saves none |
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
 │                      driver, signing-key fetch, bans/rate limits/duplicate-UUID reservation, the
 │                      directory (connection → shard)
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
5. NetherNet: verify the client offer identity; per-connection routing on a shared socket; a trickled
   answer for LAN (done: acacia `nethernet/host`).

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
- **Admission and access lists.** As a transport comes up, before its login costs a key pair, an
  address over `NetConfig::admission` (8 connections at once, 10 new ones per 10 s; loopback exempt)
  is closed (`admission.rs`; acacia caps peers and handshakes for the socket, R7). Opted into with
  `ServerBuilder::access`, `sapling_net::AccessLists` (`access/`) holds `allowlist.json` and
  `permissions.json` in BDS's format and `banned-players.json`/`banned-ips.json` in sapling's own
  (BDS has no bans; times in Unix seconds, optional expiry). Clones share one set, saved on each
  change. A finished login is checked before `Joined`: a ban by XUID, name or IP gets its screen
  (Java's wording, `DisconnectFailReason::Kicked`), an unlisted player with the allowlist on
  `disconnectionScreen.notAllowed`, and `ignoresPlayerLimit` passes the server-full check. A name-only
  allowlist entry takes the XUID of its first online join, as BDS's does. A new ban drops the players
  it names at the driver's next turn (`Sessions::enforce_bans`).
- **Game to network.** `Outbox::send` queues one batch per player per tick and never blocks. Backpressure
  is per player and reads the transport's unsent bytes (RakNet's queue past its window, NetherNet's queues
  past str0m's buffer). Over `NetConfig::congested_bytes` (1 MiB) the connection is congested
  (`Outbox::is_congested`) until it drains below `drained_bytes` (256 KiB), and the game holds back what
  can wait; past `max_queued_bytes` (16 MiB) it is kicked. Nobody paces on RTT: BDS bursts announcements and
  the client paces the section bytes (research: `acacia/research/sapling-chunk-pacing.md`).
- **Shutdown.** The shard owns the channels. Stopping it drops them, and the listener then tells every
  player the server closed and exits.
- **Join sequence** (`sapling-shard/src/join.rs`, stages in `components.rs`), BDS's order and gates as
  recorded in acacia-testserver (`cargo run -p extract-data -- --list`):

  | Server sends | then waits for |
  |---|---|
  | ResourcePacksInfo (the configured packs, "Packs (as built)") | ResourcePackClientResponse(HaveAllPacks) |
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
- **Listeners** run after the tick's systems, outside the schedule, with a `ShardCtx` over a `Backend`
  trait object, so `sapling-api` depends on nothing but `uuid`. The registry (`sapling-api/src/listen.rs`)
  is plain Rust too; the shard only feeds it events.
- **Game data** is BDS's own packets, patched (`crates/sapling-data/README.md`).

## Step 2 as built (Map)

- **Disk.** `sapling-leveldb` opens a world folder (`level.dat` + `db`) through the pinned rusty-leveldb
  fork with compression ids 0, 2 (zlib) and 4 (raw deflate, used for writes). `LevelDbWorld::load_column`
  decodes sub-chunk records (v1/8/9) into `acacia_world::Chunk` sections and re-encodes Data3D biomes into
  the network form the chunk already reads; acacia-world is unchanged. `open` works in place, as LevelDB
  does (log replay rewrites files); `open_in_memory` copies the folder into memory and never touches it
  (tests, read-only maps). `LevelDbWorld::store_column` writes a column as one batch: version 42,
  FinalizedState 2, Data3D (heightmap recomputed, biome headers with bit 0 set as BDS has them) and v9
  sub-chunks, one layer unless a block is waterlogged; sections of air are deleted. The block-entity
  record is written as the world holds it; pending ticks are left as stored.
  `LevelDbWorld::flush` writes the memtable out and replaces `level.dat` with the one it is given,
  keeping the old file as `level.dat_old`; a world opened in memory never touches its folder.
- **Actors** (`actors.rs`). A column's mobs and items are read through `digp` + `actorprefix`, or the
  legacy `Entity` tag, into one record of compounds, and written back the current way (stale
  `actorprefix` keys deleted, the legacy tag removed); a record equal to what is stored writes nothing.
  Reads the 73 actors of the hub world; `cargo run -p sapling-leveldb --example actor_dump -- <world>`
  prints one of each kind.
- **Checked against BDS 1.26.52** (2026-10-04): the fixture rewritten column by column with
  `cargo run -p sapling-leveldb --example rewrite -- <world> <new world>` (markers in every column: a
  byte, an int and a string state, and a waterlogged block), BDS started on a copy, and
  `cargo run -p sapling --example bds_world_check -- <server> <new world>` comparing what a bot is sent
  with the folder: 25 columns, 0 blocks differ, as for the untouched fixture. The bot needs physics on,
  or BDS sends only the 3x3 columns at the spawn. BDS recomputes a stair's `minecraft:corner`.
- **Block states.** Palette entries resolve by name + states against acacia-world's registry, cached by
  their raw bytes. Unknown states of a known block become its first state, unknown blocks become
  `minecraft:info_update`; each is logged once. Writing needs typed values (byte or int) and the
  registry keeps text: the typing whose `{name, states}` NBT reproduces the state's network hash is used
  (`states.rs`), and a test writes and reads back every vanilla state.
- **Storage.** `WorldStorage::load_column` (blocking) with `LevelDbStorage` and `MemoryStorage` (empty =
  void). A storage-backed `World` owns `WorldStorage::load_threads` I/O threads (two unless the storage
  says otherwise): `request(pos)` queues a load, the thread reads,
  builds the column and encodes all of it (`Column::warm`), `poll()` takes finished columns once a tick.
  The shard only does `try_recv`. Columns storage lacks are void. A sweep every 100 ticks hands the
  columns written since the last one to the same threads (`WorldStorage::store_column`, a copy of the
  chunk) and drops the saved columns outside every view. One save per column is in flight at a time and
  a column is not unloaded before its save is back, so saves cannot land out of order or be overtaken by
  a load; a failed save leaves the column unsaved for the next sweep. A stopping shard saves and waits
  (`World::save_and_wait`), then flushes storage with the world's `Level` (below). The only other calls that wait on disk are `World::open_folder`
  and `open_folder_in_memory`. A storage call that panics on an I/O thread is caught and counts as a
  failed load or save. A column storage could not read is served as void and refuses writes, so it is
  never saved over.
- **Generation.** `Generated<S>` wraps any storage: an overworld column the inner storage lacks is made
  by sapling-worldgen's `OverworldGenerator` (one per world, shared by the I/O threads) and converted to
  a `Chunk` by name + states (`generated/convert.rs`; water and lava in layer 0, as BDS saves them).
  Other dimensions stay void. A generated column is saved only once written, as BDS saves only changed
  chunks. `World::with_storage(Arc::new(Generated::new(MemoryStorage::default(), seed)))` is a pure
  generated world; `World::open_folder(path)?.generate_missing()` fills a folder's holes from its
  `level.dat` seed. `World::load_spawn` puts a world without a spawn on the surface above (0, 0).
  `Generated::with_areas(inner, seed, n)` generates on n I/O threads with caches for n areas (`new`: half
  the hardware threads, at most 4). The I/O threads take saves and player jobs first, then loads by area
  (`load_queue.rs`): a worker keeps to the area of its last load (waiting two ticks for its next columns
  before leaving it), then claims an area no other worker holds, else helps in one. So no more areas
  are generated at once than the caches hold, and a player's loads stay nearest first; players beyond n
  wait for a worker, except for their spawn columns: a shard asks for those with
  `World::request_urgent`, and urgent loads go first, area by area
  (docs/research/worldgen-server-load.md). The generator's caches, structure starts included, are
  bounded per area, so memory does not grow with the ground explored.
  Each column goes through every ported stage, features and structures included
  (`OverworldGenerator::chunk_blocks`); the Nether and the End are not generated yet (`docs/NEXT.md`).
- **Block entities.** A column keeps its record as stored and saves it back. `set_block` drops the
  compounds at a position when the block there becomes another kind (a different name). Clients get
  each compound whole, as network NBT, with the section it is in: after the section's bytes, or as the
  entry's payload for a blob-cache client (the blob is the section alone). A resync, which would leave
  the client's block without its data, is followed by `BlockEntityData` for the positions that have any.
- **Level.** A world keeps a `Level`: spawn block, time of day counter, whether the daylight cycle runs.
  A folder's comes from `level.dat`, and closing the world writes it back with `LastPlayed`, and with
  `lastOpenedWithVersion` when the folder was last written by another game version. So a folder opened
  in place gets a new `level.dat` every run; `open_folder_in_memory` leaves it alone.
  `ServerBuilder::folder` also takes the default game mode and the difficulty from it. Handlers have
  `ShardCtx::time`, `set_time` and `set_spawn`.
- **Clock** (`clock.rs`, ground truth `bds_entity_probe`). BDS 1.26.52 sends no SetTime: a joining
  player gets `SyncWorldClocks` InitializeRegistry (one clock, `minecraft:overworld`, six time markers,
  ids as BDS sent them), then SyncState about every 256 ticks and when the time is set; `paused` is the
  daylight cycle off, and StartGame's `dodaylightcycle` rule says the same.
- **Mobs** (`mobs.rs`, `actors.rs`). The stored actors of a loaded column become shard entities,
  shown to players whose view covers the column they are in and removed with it. Mobs of a kind
  with a recorded box are creatures and stored items lying items, which move ("Entities and their
  API" below); paintings and other kinds are statues. Before a save every column's actors are written
  anew: the compounds of what the shard does not move as they were, each moved one's stored compound
  with its `Pos` and `Motion` now, under the column it is in now, nothing of one that is gone. A mob
  is announced with the
  AddEntity BDS sent for a summoned one of its kind (hitbox, attributes, flags, all metadata; recorded
  for 110 kinds, sapling-data README "Mobs"), with the stored position, rotation, `Variant`,
  `MarkVariant`, `Color`, name tag and baby flag, and the flags of what it stores as saddled, tamed,
  sheared, chested or sitting. A stored `minecraft:villager` or `minecraft:zombie_villager` is sent as
  its v2 kind, as BDS does. Stored health sets the `minecraft:health` attribute. What it wears and holds
  (`Armor`, `Mainhand`, `Offhand`) follows as MobArmorEquipment and MobEquipment. A painting is an
  AddPainting of its `Motif` and `Direction` at its stored position. A dropped item is an AddItemEntity of its stored stack
  (name to network id through the bundled ItemRegistry; a block item's state by its network hash); one
  whose name the registry lacks is not shown.
- **Join, world state.** After the player's own state a joining player gets, as from BDS:
  SetSpawnPosition (no spawn of its own), the clock, SetDifficulty and GameRulesChanged (StartGame's
  rules). `set_spawn` sends the players SetSpawnPosition in the shape BDS uses for `/setworldspawn`.
- **Game rules** (`sapling_data::GameRules`, `Level::rules`, shard `rules.rs`). Every rule BDS has,
  its names, order and defaults those of the recorded StartGame; a world's are read from and saved to
  `level.dat` as BDS keeps them (lower-case root fields, a byte for a switch). `set_game_rule` tells
  the world's players of the one rule that changed, nothing when it did not, as BDS. Sapling acts on
  `doDayLightCycle` only; the rest are kept for plugins.
- **Actors checked against BDS 1.26.52** (2026-10-04): the hub world rewritten with the `rewrite`
  example, which renames every stored actor and adds one sheep per column holding only the fields
  `Actor` reads. BDS announced both to a bot (`bds_entity_probe`) with the names and wool written. The
  example also adds an armour stand (helmet, boots, sword, shield) and a painting per column: BDS's
  AddPainting, MobArmorEquipment and MobEquipment for them are the shapes sapling sends, except that
  BDS adds `Damage` to the tag of an item that wears.
- **Sending.** A `View` keeps a nearest-first queue; each tick a player gets at most
  `ShardConfig::columns_per_tick` (8) columns that are loaded, and only the nearest `2 x` that many are
  asked of storage. `NetworkChunkPublisherUpdate` goes out when the player's chunk changes.
  PlayStatus(PlayerSpawn) follows once the columns within 3 chunks are sent (BDS: 26 columns).
  A congested connection or a full blob cache gets no announcements until it drains; sub-chunk answers
  are never held. Sub-chunk requests for a column not loaded wait (`SectionBacklog`) and are answered
  once it is: a vanilla client disconnects itself on a `ChunkNotFound` entry, and never asks twice, so
  requests are dropped only past 32 sections per column of the view's square. Every 10 ticks, loads
  still queued for columns no view covers are dropped (`World::cancel_loads`), so an elytra flight or
  teleport does not leave the I/O threads generating what nobody will see.
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

## Step 3 as built (Players)

Ground truth: `cargo run -p sapling --example bds_probe -- <server>` prints what a server tells one bot
about another that joins, walks, sneaks, sprints, swings, chats and leaves. Run against BDS 1.26.52 it
gave the shapes below; run against sapling it shows the same packets.

- **Tick order.** Systems (join, input, chunks), then handlers (arrivals, chat, departures), then a second
  schedule: visibility, announcements, fan-out, flush. What a handler or an arrival causes leaves in the
  same tick.
- **Player list** (`roster.rs`). Shard-wide, independent of views. On entering the world a player gets one
  `PlayerList` per player in the world, itself included, and everyone else gets its entry; on quit, a
  remove. Each packet holds one record and is encoded once at login. Record prefix as BDS: add
  `type 1, legacy_type 0`, remove `type 0, legacy_type 1`.
- **Skins** (`skin.rs`). The login's client-data claims converted to the wire `Skin`; a test compares the
  result with what BDS sent back for the same claims (colours are `#aarrggbb` byte-swapped, piece types
  lose `persona_`, geometry is re-serialised compact). Claims are untrusted and go to every client: sizes,
  base64, JSON and counts are checked, and a skin that fails becomes a plain grey one.
- **Visibility** (`visibility.rs`). A player is an entity for the players of its layer whose chunk `View`
  covers its chunk. `Sees` is diffed once a tick: `AddPlayer` (feet position, `Fallback` game mode, the
  recorded metadata and Base ability layer, device id and OS) or `RemoveEntity`. AddPlayer is encoded once
  per tick per appearing player.
- **Movement** (`movement.rs`). `MovePlayer` mode Normal with the eye position, head yaw, on-ground and the
  mover's input tick: what BDS sends for other players (it uses MoveEntityDelta for other entities).
  Sent only when position or rotation changed, encoded once per mover, shared by its viewers as `Bytes`.
  On-ground is `VerticalCollision && delta.y <= 0` from the input.
- **Trust.** Positions are the client's word. Non-finite values are dropped; a step over 10 blocks is
  answered with `MovePlayer` mode Teleport back to the server's position (shape of BDS's `/tp`), repeated
  at most every 20 ticks.
- **Pose and swings.** StartSneaking/Sprinting/Swimming and their stops set entity flags; viewers get
  `SetEntityData` with the Flags item (recorded flags + BREATHING + pose) and BoundingboxHeight (1.49
  sneaking, 1.8 otherwise). `Animate(SwingArm)` is relayed under the sender's own id, once a tick.
- **Chat** (`chat.rs`). Only `Text` of type Chat from players in the world. Control characters and `§`
  codes are stripped, 512 characters kept, 3 lines a tick per player. `Handler::on_chat` returns
  `ChatOutcome` (Allow / Cancel / Replace); what passes goes to everyone in the world as BDS relays it:
  type Chat, category Authored, `source_name` = verified name + `§r`, verified XUID.
- **API.** `ShardCtx::players()`, `position(id)`, `broadcast(msg)`, `send_message(id, msg)` (Raw text).
  Join/quit announcements are the server binary's job (the hub example broadcasts them).
- **Hub world.** `worlds/hub` (gitignored) is BDS seed 20261003, chunks x -19..=20, z -8..=31 all saved.
  `cargo run -p sapling-leveldb --example inspect -- <world>` draws which columns a folder holds.
  Regenerating: touch every chunk (100-chunk ticking areas, bedrock floor to stone and back) with a bot
  online, let the bot leave so its chunks unload, then `stop`. Never `save hold` before `stop`: BDS hangs.

## Step 4 as built (API)

Ground truth: `cargo run -p sapling --example bds_ui_probe -- <server>` prints BDS's command list and what
it sends for `/title`, `/scoreboard`, `/gamemode`, `/tp`, a wither's bar and commands that fail.

- **Plugins and listeners** (`sapling-api/src/{listen,plugin}.rs`, `sapling-shard/src/plugins.rs`).
  `Server::builder().plugin(p)`: `Plugin::build` runs once per shard and adds commands, listeners
  (`p.on::<E>(priority, |ctx, ev| ..)`) and tasks (`every`, `later`, in ticks). Every event carries its
  player (`ev.player()`). `Highest` runs first, `Lowest` last, `Monitor` after all; a cancel stops the
  listeners that have not run, except monitors (a copy of the final event, changes discarded) and those added
  `Priority::X.receiving_cancelled()`. A running listener adds more with `ctx.in_world(name)`,
  `ctx.for_player(id)` or `ctx.on_shard()`; they join from the next event, and a world's or player's go
  when it is removed or they leave the shard (after their `QuitEvent`). A world's tasks run with it in
  focus. The registry is a world resource (`plugins::Registry`), so any of the game's code raises an event
  with `plugins::offer(world, ev)`; it is taken out while it runs, so **what a running listener or task
  causes is not offered again** (a damage listener that hurts someone cannot set itself off; a plugin's
  `ctx.hurt_player` goes unheard). Changes made meanwhile wait in `plugins::Deferred`; both are locked
  because a resource must be `Sync`. Commands sit apart in an `Arc` (`CommandRegistry`), so a player's
  command runs with the listeners in place and what it causes is heard. `Handler` is a plugin as one value: each method is a listener at the handler's priority
  (`handler_at`). Tasks run once a tick after the events, then `TickEvent` (`on_tick`).
- **Keeping the API stable** (`sapling-api`). Enums that will grow (`DamageCause`, `Effect`, `Attacker`,
  `DeathMessage`, `Target`, `Sender`, `Scope`, form `Element`, `FormError`, `ParseError`, a command's `Selector`,
  `Pick` and `Filter`, which took its arguments in 0.2.0) and the snapshots
  plugins only read (`Player`, `Position`, `EntityInfo`, built with `new`) are `#[non_exhaustive]`; events
  keep private fields. Not marked: closed enums (`Face`, `GameMode`, `Difficulty`, `Slot`, `Priority`, ..),
  `ParamKind` (only the shard and the derive match it), and values plugins write as struct literals
  (`Block`, `Item`, forms, HUD), which would lose struct-literal and `..Default::default()` construction.
  `tools/semver-check.sh [rev]` runs cargo-semver-checks on testbox against `rev` (default origin/master).
- **Vanilla rules** (`sapling-shard/src/rules.rs`, `sapling-data/src/game_rules.rs`). A world's game rules
  are its vanilla switches: plugins set them with `ctx.set_game_rule`, operators with `/gamerule`, and they
  are saved in level.dat. Read: `doDayLightCycle`, `doWeatherCycle`, `doFireTick`, `randomTickSpeed`,
  `mobGriefing`, `pvp`, `doMobLoot`, `keepInventory`, `showDeathMessages` (the `DeathEvent`'s starting
  message), `fallDamage` / `fireDamage` / `drowningDamage` / `freezeDamage` (players only; a spared tick
  hit keeps its timers), `naturalRegeneration`, `doTileDrops` (every block drop), `doEntityDrops` (a
  falling block's item), `tntExplodes` (the fuse burns, nothing bursts), `tntExplosionDropDecay` (a block
  drops by 1 in the power), `respawnBlocksExplode` (anchors), `playersSleepingPercentage`. What no BDS rule
  switches is a `sapling:` rule in the same store, never sent to clients, each on by default:
  `sapling:hunger` (no exhaustion), `sapling:liquidFlow` (due flows are dropped: liquids stand),
  `sapling:redstone` (circuits pause; what was written waits), `sapling:mobAi` (mobs fall and are hurt but
  decide nothing). `sendCommandFeedback` is under Commands. `doMobSpawning` stops natural spawning
  (`spawning.rs`), not eggs or commands. Not read yet: `spawnRadius`, `doImmediateRespawn` (the client's),
  and rules for what sapling lacks (`doInsomnia`, command blocks).
- **World events** (`sapling-api/src/world_event.rs`). What the world does on its own is offered with its
  world in focus (`plugins::offer_in`), so `Scope::World` listeners hear their world's; each is built only
  where a listener hears it (`plugins::stopped_in`), since flows and fire raise them often.
  `ExplosionEvent` (`creatures/blast.rs`): source kind, `power` and the `blocks` it breaks, editable, worked
  out before it is offered; cancelled, nothing is hurt or broken. `EntitySpawnEvent` (`creatures::spawn`):
  kind, feet, `SpawnReason` (`Command`, `Summoned` vexes, `Effect` slimes and silverfish, `Natural`,
  `SpawnEgg`); mobs from
  storage are not offered. `BlockSpreadEvent`: fire into a cell or burning it away (`blocks/fire.rs`), a
  liquid flowing in (`blocks/liquid/flow.rs`), asked before any fizz or popped loot. `BlockGrowEvent`: a
  random tick's growth (`blocks/growth.rs`, all of one plant's changes at once) and a sapling's tree; not
  farmland drying or leaves decaying.
- **Player actions** (`sapling-shard/src/actions.rs`). `MoveEvent` (one a tick per player that moved or
  turned, from the tick's start; `changed_block()` for block-level checks): `movement::player_input` queues
  it in `movement::Moves`, since systems hold no plugins, and dispatch offers it before anything reads
  positions; a cancel teleports the client back, as Paper does. `ItemUseEvent` is a use on nothing
  (eating, drinking; a cancel stops the client's use), `EntityAttackEvent` / `EntityInteractEvent` a click
  on a player or an entity (`Target`); a cancelled attack does no harm, an interaction has no vanilla effect
  yet. Clicks naming nothing raise no event.
- **Harm, death, rising** (`sapling-shard/src/damage.rs`, `vitals/{stand,struck,death}.rs`). `DamageEvent`
  (a player or a mob, its `DamageCause`, the `Attacker` a hit's `Slayer` names, a mutable `amount`) is
  asked before a hit lands and before its throw, so a cancel leaves no harm and no knockback. What
  surroundings do is asked once it is taken (`damage::weigh`): a player's tick as one loss (`stand`), each
  of a mob's hits on its own (`creatures/harm.rs` `tick`). A cancel gives back the health and the hit's
  guard but keeps the timers (`Health::reweigh`), so a spent landing or a fire's countdown does not come
  back; a changed amount is the loss (`Health::lose`). `/kill` asks with all the health left. `DeathEvent` (players) sets `keep_inventory` and `message`
  (`Vanilla`, `Hidden`, `Text`); the death screen keeps the game's words. `RespawnEvent::feet` starts as
  the bed or the world's spawn.
- **State.** `ctx.state::<T>()` is a per-shard `T::default()` keyed by type: what a plugin's handler,
  commands and form callbacks share (commands are types, not values, so they hold nothing).
- **Commands** (`sapling-api/src/command`, `sapling-shard/src/commands.rs`). `#[derive(Command)]` on a struct
  (fields = parameters) or an enum (variants = sub-commands) writes `CommandSyntax`; the plugin writes
  `Command::run(self, ctx, sender, out)`. Parameters: integers, floats, `bool`, `String` (a word or a quoted
  run), `Text` (rest of the line), `Target` (an online player or `@s`), `GameMode`, any `#[derive(CommandEnum)]`,
  and `Option<T>` for trailing optional ones. `Sender` is `Player | Console`; only players send yet.
  - `AvailableCommands` is built once per permission level and sent when a player enters the world and when
    `ctx.set_permission` changes its level (`Member` / `Operator`). Hidden commands are left out entirely and
    answer as unknown; every command sent carries permission `"any"` and flags 0.
  - Wire shape as BDS 1.26.52: enum values and enums deduplicated; an enum parameter carries its enum index
    with flag 0x30; a sub-command word is a one-value enum; aliases are an enum `<name>CommandAliases` led by
    the name. Type ids from the capture: int 1, float 3, target 8, string 56, message 68 (acacia-proto names
    44 and 55 for the last two: an older protocol). A test compares them with the recorded BDS list.
  - Answers are `CommandOutput` with the request's origin echoed, `output_type` "alloutput", and BDS's keys
    for failures: `commands.generic.unknown [name]`, `commands.generic.syntax [before, token, after]`,
    `commands.generic.noTargetMatch`. `out.print` / `out.error` send text as the message id, as dragonfly does.
  - An operator's success lines, whatever the command, are echoed to the other operators on the shard as BDS
    echoes them (it echoed an operator's `/tell`): a Json `Text`, `§7§o[` + the sender's name + `: ` + the
    line + `]`. `ctx.send_translation` sends one player a Json line under a key, as BDS tells a `/tp` victim
    (`commands.tp.successVictim`). `out.done_to(n)` sets `success_count` (BDS's `/give @a` and `/tell @a`).
  - Where the sender's world has `sendCommandFeedback` off, a command that succeeds goes untold: no answer,
    no echo, and vanilla's lines to its targets held back; failures are answered, and so is a command that
    calls `out.always_answer()` (`/tell`). The rule is read after the command, so turning it off goes
    unanswered (research/captures/bds-feedback.txt, `bds_command_probe feedback`).
  - Three lines a tick per player, 512 characters, control characters dropped.
- **Forms** (`sapling-api/src/form`, `sapling-shard/src/forms.rs`). `Menu` (answer: button index), `Modal`
  (bool), `Custom` (one `Value` per element), and `#[derive(Form)]`: a struct whose value holds the defaults
  and whose answer is the struct filled in. `ctx.send_form(player, form, |ctx, player, response| ..)`; the
  response is `Submitted(T)` or `Closed`. Ids count up per player; an answer to an id that is not open is
  ignored; 16 forms may be pending; all are dropped when the player leaves. Answers are checked against the
  form: array length, null for labels, types, slider bounds, option indices, input length (256 characters
  unless `max_len`), 32 KiB in total. Anything that fails is `Closed`. JSON as in acacia's
  `docs/research/forms.md` (menus use the legacy `buttons` array).
- **Sidebar** (`hud.rs`). One objective `sapling` in slot `sidebar`, ascending, one fake player per line
  (id = -(line + 1), below the world board's ids; score = line). `ctx.set_sidebar` diffs against what the
  player was shown: changed lines are removed and set again (the client keeps the text of an entry it
  knows), a new title rebuilds the board, 15 lines at most. It hides the world board's sidebar, which is
  shown again when it goes.
- **Scoreboard** (`sapling-shard` `scoreboard.rs`, `scoreboard/`; API `sapling-api` `scoreboard.rs`). One
  board per world (BDS: per level, so a realm's dimensions share it), a `Board` component on the
  overworld layer, made from the level's `scoreboard` record when first needed and handed back to it on
  save (`keep_all`); NBT layout in `sapling-leveldb` `scoreboard.rs`. Holders are a player's or entity's
  `UniqueID` (players: `Marks`, saved in their record, made from the UUID until one is; entities:
  `UniqueId`, from `entities::keep`) or a fake name; scoreboard ids count from 1. A change is a `Change`;
  after each command (before its answer, as on BDS) and after the handlers `scoreboard::send` tells
  every player of that world (SetDisplayObjective, SetScore, RemoveObjective, as BDS: a score only
  while its objective is shown, a replaced slot cleared first, a removed objective to everyone) and shows a player entering a world its slots after removing those of
  the world it left (`Hud::board_objectives`, carried). No SetScoreboardIdentity: BDS sent none in the
  drills. Tags (`tags.rs`): a player's in `Marks` (carried, saved as `Tags`), an entity's in its stored
  compound's `Tags`. Answers and packets: `docs/research/scoreboard.md`.
- **Boss bar.** One per player, on the player's own entity id (dragonfly's way: no extra entity).
  A new bar is ShowBar, `remove_boss_bar` HideBar; a bar that is shown changes by SetBarTitle,
  SetBarProgress and Texture (colour), one for each part that differs. A vanilla client ignores ShowBar
  for a bar it already shows, though BDS repeats ShowBar once a second for its wither (there the client
  reads the entity). BossEvent is flat in 1.26.5x: every type has id, type, title, filtered title,
  progress, colour, overlay, so each event carries the whole bar and HideBar carries an empty one. BDS's
  ShowBar bytes prove that layout (acacia `docs/proto.md`); BDS sends no other type, not even when the
  player leaves the wither's range, so the others rest on gophertunnel.
- **Titles.** SetDurations, SetSubtitle, SetTitle in that order, unset times -1, as `/title` sends them;
  `send_actionbar`; `send_toast` (ToastRequest). `ctx.title_action` sends one step of `/title` alone
  (title, subtitle, action bar, their rawtext `*Json` types, durations passed on as given, -1 too,
  Clear, Reset).
- **Sounds and particles** (`sapling-api/src/sound.rs`, `sapling-shard/src/sounds.rs`). `ctx.play_sound`
  (PlaySound, position in eighths of a block cut toward zero, as BDS's `-147` for y -18.375),
  `ctx.stop_sound` (StopSound; no name is `stop_all`), `ctx.spawn_particle` (SpawnParticleEffect, entity -1,
  the world's dimension); the `_in_world` forms go to whoever sees the place.
- **Rawtext** (`sapling-api/src/command/rawtext.rs`). The `RawText` parameter (wire type 74, json) reads a
  JSON object to its closing bracket and checks it with BDS's `commands.tellraw.error.*` keys
  (`ParseError::Invalid`); text that is no JSON is a syntax error at its first word. Parts with no known
  key are dropped. `resolve` replaces selectors and scores; `to_json` writes it as BDS sends it (`with`
  always a rawtext, no parts `null`, a trailing newline). `ctx.tell_raw` sends a JsonWhisper, `ctx.emote`
  `/me`'s sender-less Chat `* name message`.
- **Teleport.** `ctx.teleport` sends MovePlayer mode Teleport, cause Command, as `/tp`; the player's inputs
  are ignored until one carries `HandledTeleport`, and the teleport is repeated after 20 ticks without one.
- **Game mode.** `ctx.set_game_mode` sends UpdatePlayerGameType to everyone in the world, the player
  included: all BDS's `/gamemode` sends besides chat lines (no SetPlayerGameType, no UpdateAbilities).

## Step 5 as built (Worlds)

No BDS capture exists for this step: BDS has one world per dimension. The client side of a world
change follows dragonfly @ 1793369 (`session.handleWorldSwitch`, `Loader.ChangeWorld`), whose
same-dimension case sends no ChangeDimension: it hides the old world's entities, teleports and
sends every column of the view again. A move into a world of another dimension is a dimension change
instead ("Dimensions and portals (as built)").

- **Worlds in a shard** (`worlds.rs`). A world is a layer entity with a name. `Atlas` maps names to
  layers, and to a shard index for worlds elsewhere, which a shard learns by message (`WorldAdded`,
  `WorldRemoved`). Chunk views, the section backlog, mobs, block changes and their viewers,
  visibility, the clock, the saving sweep and the spawn already went by the player's layer; chat
  and `set_time` now do.
- **Scope.** The `ShardCtx` calls that name no world (`block`, `set_block`, `spawn`, `set_spawn`,
  `time`, `set_time`) act on the world of the event: the player's for join, quit, chat, command,
  block and form events and `on_world_change`, the shard's first world in `on_tick`.
  `ctx.within(world, |ctx| ..)` runs them on a named world of the shard (`block_in`,
  `set_block_in` are shorthands); `ctx.world()` names the world in scope.
- **Who hears what.** A player's chat goes to its world. `broadcast` is the shard, as it was;
  `broadcast_in(world, ..)` is one world on whichever shard; `broadcast_all` is the server.
  `players()` and `player_count()` are the shard's, `players_in(world)` a world's.
- **Player list.** The server's: a join or quit is told to the other shards (`Listed`,
  `Unlisted`), which keep the entries (`roster::Elsewhere`) for their own newcomers. A transfer
  sends no list packet. Entity ids are unique across shards (a shard counts up from
  `index << 24`: MovePlayer carries 32 bits) and a player keeps its id, so its list entry, its
  boss bar and what others were told of it stay valid.
- **A move inside a shard** (`worlds::enter`): the player's layer is written, its view forgets every
  column, its blob references and waiting section requests are dropped, and it is sent MovePlayer
  mode Teleport to the new world's spawn and that world's clock (SyncState). The visibility diff of
  the same tick removes the old world's players and mobs (RemoveEntity) and shows the player to the
  new world; from the next tick the new world's columns go out (NetworkChunkPublisherUpdate,
  LevelChunk), then its entities. The stage stays `InWorld`: no loading screen, no PlayStatus.
- **Leftover columns.** A vanilla 1.26.5x client kept a hub column on screen in the arena: it
  does not drop every old column when the publisher area moves, and a column the new world never
  sends stays. So every column the view holds that a view around the new spawn does not cover is
  overwritten first, ahead of the teleport and while the client's publisher area is still the
  old one, with the one empty column (`sapling_world::empty_column`: no sections, so nothing is
  asked for; for a blob-cache client it names the biome blob like any column). The columns
  inside the new view are overwritten by the new world's own. Across shards the view travels
  with the player, so the new shard knows what the client holds. Nothing queued for the old
  world goes out afterwards: the view's queue is dropped with its columns. acacia-bot answers
  each empty column with a request for no sections; such a request is ignored, or a view's
  worth of them would fill the section backlog and the new world's first requests be dropped.
- **What the client still sends from the old world.** Inputs are ignored until one carries
  `HandledTeleport`, block actions and clicks with them (this holds for any teleport). A section
  request is answered only once its column was announced in the current world: until then it
  waits in the backlog, so nothing is answered `ChunkNotFound` or from the old world; outside the
  view it is dropped, and the backlog is emptied by the next move. A blob status naming released
  blobs gets the empty reply every status gets.
- **Shards.** `Link::mesh(n)` gives each shard a mailbox and the others' senders (std mpsc);
  nothing else is shared between shards, no lock and no condition variable.
  `sapling_net::Router` is the directory, connection to shard, on the network task.
  `Outbox::route` moves a connection, and the old shard is told `Rerouted` through its inbox,
  behind the last event it was sent.
- **A move across shards** (`handoff.rs`):

  | Step | Old shard | New shard |
  |---|---|---|
  | `Ask` | the player plays on; another transfer of it is refused | answers whether it has the world |
  | `Answer` | refused, or none in 100 ticks: `on_transfer_failed` | |
  | `Commit` | takes the player's components off the entity and sends them, its unsent packets and the RemoveEntity of all it saw among them; removes it for its viewers; tells the directory; despawns | spawns the player in the world (its first world if that one is gone), settles it as above, `on_world_change` |
  | until `Rerouted` | buffers what the network still delivers, a disconnect included | holds what the network delivers directly |
  | `Forward` | sends the buffer | takes the forwarded events, then the held ones |

  Every packet is handled once, in order, by the shard that owns the player then, and no shard
  waits on another. A commit to a shard that is gone comes back and the player is put together
  again. A connection that ended before the reroute is reported as left to the new shard by the
  directory. A stopping shard takes in what was committed to it, sends the forwards it owes,
  withdraws its worlds and disconnects its players (`Shard::close`); the directory closes a
  connection whose shard is gone. A held player whose forward does not come in 100 ticks is let
  in without it.
- **What goes with the player** (`player.rs`): identity, entity id, permission and the command
  list it was told, game mode, rotation and pose, view radius, blob-cache setting, list entry and
  skin, HUD state (the client keeps its sidebar and boss bar; nothing is sent again, a later
  change is a diff), open forms with their callbacks (they run on the shard the answer reaches)
  and unsent packets. Not `ctx.state`: that is per shard.
- **Events.** A transfer raises no `on_quit` and no `on_join`. `on_world_change(ctx, player, from,
  to)` runs on the destination shard, before what the player then sends; `on_quit` runs on the
  shard the player is on when it leaves.
- **Builder.** `world` / `folder` are the default world (`"default"`, or `default_world_name`),
  on shard 0; `add_world(name, ..)`, `add_world_on(shard, name, ..)`, `add_platform_on(shard,
  name)`, `join_world(name)`, `template(name, template, spawn)`. One thread per shard index
  used; a shard's first world is its fallback. `handler_each(|| h)` makes a handler per shard;
  `handler(h)` is for servers of one shard (`StartError::HandlerPerShard` otherwise). Plugins,
  commands and templates go to every shard.
- **Arenas** (`sapling-world/src/template.rs`, `sapling-shard/src/arenas.rs`).
  `Template::of(world)` freezes a world in memory (`World::load_all` first for a folder) with
  every encoding made; `World::arena(&template)` copies nothing and reads the template, void
  beyond it. The first write to a column copies that column's blocks into the arena
  (`Column::fork`) and keeps the template's encoded sections, as the same bytes under the same
  blob hashes, for all but the section written. So arenas of one map share one encoding in
  memory and one set of blobs in a client's cache. `ctx.create_world(name, template)`,
  `reset_world(name)` (the changed columns are dropped and announced again to the players
  inside, who stay) and `remove_world(name)` (players go to the shard's first world, mobs go
  with the arena) act on the handler's shard; an arena is never saved.
- **Hub.** `hub` is the default world and `arena` a stone platform on a second shard; `/world
  [name]` and a menu button move between them; each world hears who came and went; anyone may
  build in the arena.

## Step 6 as built (Protection)

Ground truth: `cargo run -p sapling --example bds_block_probe -- <server>`: an actor bot breaks, places and
clicks blocks in creative, survival and adventure, also what no client should get away with, while a
watcher bot stands by; both print the block, sound and inventory packets they get. BDS 1.26.52.3 gave:

| The actor | got itself | the watcher got |
|---|---|---|
| StartBreak | LevelEvent `ParticlePunchBlock<face>` (block centre, data = block hash); in survival and adventure also `BlockStartBreak` (block corner, data = 65535 / break ticks) and `BlockBreakSpeed` | the same |
| PredictBreak, taken | LevelEvent `ParticleDestroy` (centre, hash), UpdateBlock air (flags 3: Neighbors, Network; layer 0); in survival the dropped item | the same |
| PredictBreak, refused (adventure; survival, finished at once) | UpdateBlock of the block, the one above it (twice) and the next above, layer 0 | nothing more |
| ClickBlock with a block in hand, taken | UpdateBlock of the clicked block and the cells around it, its inventory, then UpdateBlock of the new block (flags 3, layer 0) and LevelSoundEvent `place` (centre, extra data = hash, no entity) | the last two |
| ClickBlock, refused (adventure; too far) | UpdateBlock of the clicked block, the cell on the clicked face and the cells before and beyond them, layer 0, and its inventory | nothing |
| ClickBlock on a trapdoor | UpdateBlock of the new state, LevelSoundEvent `trapdoor.open` | the same |

BDS took a click whose block centre was 6.4 blocks from the eye in survival and 12.2 in creative and
refused 7.3 and 13.2. In creative StartBreak alone breaks nothing: PredictBreak does. A survival break
finished at once is refused and BDS then breaks the block itself when its time is up (it mines along
from StartBreak). It closes the connection of a client that sends `CreativePlayerDestroyBlock` as a
block action, and takes StopBreak and AbortBreak with a position.

- **Requests** (`sapling-shard/src/blocks.rs`, packets in `blocks/send.rs`). `movement::player_input` hands
  an input's block actions and item use to `BlockInputs`; `InventoryTransaction` ItemUse packets join them;
  `blocks::dispatch` runs with the other handlers. PredictBreak, or an item use of type BreakBlock, is a
  break (the same position twice in a tick is one break). StartBreak sends the punch particles to
  everyone who sees the block. ClickBlock is a place when the item names a block and the clicked block is
  not one a click uses (`USED`: doors, buttons, chests and the like by the end of their name; sneaking
  builds on them too), otherwise an interaction.
- **Checks before any event.** The position is inside the world's height, its column is loaded, and its
  centre is within 7 blocks of the player's eye (13 in creative). A break needs a block that is not air or
  liquid. A place needs a face 0..=5, a clicked block that is not air, a free cell (air or liquid, inside
  the height, loaded) and, for a block with collision, no player standing in it.
- **Events.** `Handler::on_break(ctx, player, &mut BreakEvent)`, `on_place(.., &mut PlaceEvent)`,
  `on_interact(.., &mut InteractEvent)`: plain data (`BlockPos = [i32; 3]`, `Face`, `Block { name,
  properties }` with the registry's `key=value,..` text), `cancel()`, `allow()`, `is_cancelled()`.
  `PlaceEvent::block` is public: a handler may place something else. Priorities and monitors as for chat.
- **Defaults by game mode.** A break starts allowed in survival and creative and cancelled in adventure
  and spectator; a place starts allowed in creative only; an interaction starts cancelled for spectators.
  A cancelled start is not a handler's cancel: every handler still sees the event (`Event::is_stopped`)
  and any may `allow()` it, so a plugin can open one region to adventure players. A handler's `cancel()`
  stops the handlers after it, as for chat.
- **Taken.** A break writes air, or the water the block stood in (layer 1 is emptied), and everyone whose
  view covers the column, the breaker included and players still loading too, gets UpdateBlock (flags 3)
  and `ParticleDestroy`. A place writes the event's block and sends UpdateBlock and the `place` sound.
  `World::set_block` marks the column for saving and drops its cached section and heightmap encodings, so
  a later joiner, cached or not, gets the new section under a new blob hash (tested with a shard and a bot).
- **Refused: the resync** (dragonfly #749). The client predicted, so it is shown the truth: after a break
  the block and the one above it; after a place the clicked block **and** the cell on the clicked face,
  where the client put its block; after an interaction those two and the blocks above and below (a door
  is two blocks). Each position goes out on both storage layers (layer 1: the water or air), flags 3, to
  that player only. BDS's captures show layer 0 only; none had water in them.
- **Interactions** change nothing yet, allowed or cancelled: the event is raised and the click is resynced,
  so a door a client opened on its own screen closes again.
- **Trust.** Positions and the eye are the server's own; the packet's `player_pos`, `click_pos` and
  clicked block id are ignored.
- **Placing.** In survival the block is the one in the server's copy of the hand, whatever the client
  names; a place that happens takes one from the stack. In creative, where any block is to be had, it is
  the block the client names and nothing is taken. In adventure and spectator, and with a hand that
  holds no block, the event starts cancelled and a handler that allows it vouches for the client's
  block. Outside creative the placer is sent its four windows after every attempt, as from BDS: its
  client counted one off the stack whether the place happened or not.
- **Breaking time** (`blocks/mining.rs`). In survival StartBreak starts a clock: the break needs
  hardness x 30 ticks over the tool's speed when the tool in the server's hand harvests the block, x 100
  at speed 1 when it does not; Efficiency adds level² + 1. The formula and its sources are acacia-bot's
  (`interact/break_time.rs`). PredictBreak on the same block counts when that many ticks have passed,
  less 3 for inputs that run ahead (with a sword or shears in hand at once: their speeds here are
  Java's, an upper bound on Bedrock's); otherwise the BreakEvent starts cancelled (a handler may allow it)
  and the client is shown the block. AbortBreak ends the clock. Unbreakable blocks and blocks of unknown
  hardness never pass in survival. Viewers get `BlockStartBreak` (block corner, 65535 / ticks) at the
  start and `BlockStopBreak` on an abort. Creative breaks at once, as before. Not taken into account,
  each of which would only lengthen a break: haste and mining fatigue (there are no effects), water and
  being off the ground (the server's on-ground flag is a guess). BDS also sends `BlockBreakSpeed` and
  breaks the block itself when a refused break's time is up; sapling does neither.
- **Turning and replacing** (`blocks/facing.rs`, `blocks/place.rs`). A placed block's state comes from
  the clicked face, how high on it the click was, and the placer's yaw and pitch. Every rule and its
  evidence is a row of `docs/research/block-rules.md`, section 1 (collision boxes per state, the Java
  game's placement code mapped to Bedrock's states, acacia-bot's runs on BDS): logs' axis, stairs,
  slabs and trapdoors' upper half, furnace-like blocks facing the placer, doors and anvils across the
  look, fence gates along it, torches, ladders, buttons, wall signs, levers, standing signs, hoppers,
  rods, dispensers, droppers, barrels, observers, pistons (other servers' code only: unverified).
  A door hinges on the right where the same door stands to its left, or only its right has a solid
  block (the Java game's `DoorBlock.getHinge` less its tie-break by where the click fell). A bed's foot
  goes into the clicked cell and its head one cell along the look (`blocks/doors.rs`); both need
  something under them, each gets a `Bed` block entity with the item's colour, and breaking either
  takes both. Items that name no block find theirs by name (`blocks/items.rs`): doors, beds,
  hoppers, cake, flower pots, cauldrons, brewing stands, campfires, seeds and the like (wheat seeds
  build `wheat`, sugar cane `reeds`, redstone `redstone_wire`, string `trip_wire`); a sign or
  banner stands on a top and goes on the wall on a side (dark oak's are `darkoak_`), a hanging
  sign hangs under a ceiling (from one chain, `attached_bit`, under a block without a whole bottom
  face or when its placer sneaks) or sticks out of a wall. Skulls are block items and turn to the
  clicked face; a campfire faces the way its placer looks. A block is placed into the cell of a block
  the Java game marks replaceable (grass, ferns, vines, fire, a single snow layer, nether roots and
  the like) instead of against it. A block that needs something to stand or hang on (next bullet) is
  not placed where that is not solid.
- **Block entities from items** (`blocks/entities.rs`, `docs/research/block-entities.md`). A placed
  sign, hanging sign, skull, banner, flower pot, cauldron or campfire gets the entity BDS writes on
  placement (`Sign` with empty `FrontText`/`BackText`, `Skull` with the floor rotation, `Banner`
  with `Base` = the item's damage, ...); a brewing stand is a container of five (window type
  BrewingStand; ingredient, three bottles, fuel) whose block shows a bottle in each place that
  holds something (`containers/brewing.rs`); it brews nothing. Breaking gives
  the item back what the entity kept: a bed's and a banner's colour, a pot's plant, a campfire's
  food. Signs of every wood leave their wood's sign, banners `banner`. Not done: cauldrons hold
  nothing, a banner's patterns are lost (`Item` has no tag).
- **Pots and campfires by hand** (`blocks/pots.rs`, `campfires.rs`). A click on an empty flower pot
  with a plant the Java game pots (saplings, flowers, mushrooms, fern, dead bush, cactus, bamboo,
  azaleas, fungi, roots) puts it in as `PlantBlock` (the block's `{name, states, version}`, as BDS
  keeps it) and takes one in survival; an empty hand takes it back. A click on a lit campfire with
  food lays one on its first free place (`Item1` to `Item4`); 600 ticks later it drops off cooked,
  by the `campfire` recipes of the recorded `CraftingData` (`Station::Campfire`). A campfire with
  food ticks while its column is seen, as furnaces do (`ticking.rs`).
- **Writing on signs** (`blocks/signs.rs`). The placer of a sign is sent `OpenSign` (front), and a
  click on a sign that is not waxed opens the side the clicker stands on (Java
  `SignBlock.isFacingFrontText`: a standing sign faces its 16th of a turn, a wall sign its face).
  That player may answer once with `BlockActorData`: of it only each side's `Text` is kept (512
  bytes at most, a cap of sapling's), with `TextOwner` = the writer's XUID, and shown to everyone.
  Anyone else's `BlockActorData` is ignored. Before the editor, a click goes as BDS's
  (`blocks/sign_ink.rs`, `docs/research/sign-dyes.md`): nothing in adventure mode; a waxed sign
  only sounds (`block.sign.waxed_interact_fail`); honeycomb waxes it (`IsWaxed`, LevelEvent
  `ParticleWaxOn`); on a side with words, glow ink lights it and an ink sac puts it out
  (`IgnoreLighting`, `InkSackUsed`), and one of the 16 dyes sets its `SignTextColor` from BDS's
  table (`DyeUsed`), each taking one outside creative mode. One that changes nothing opens the
  editor instead.
- **Ground** (`blocks/ground.rs`, `docs/research/standing-and-shapes.md` part A). BDS's own rule for
  where crops and stems (farmland), nether wart (soul sand), sweet berries, sugar cane (cane, or
  dirt or sand with water or frosted ice beside that ground block), cocoa (a jungle log on the
  side it names), cave vines, redstone wire, repeaters, comparators and rails may stand; string
  stands anywhere. It replaces the "something solid below" rule for these blocks, is asked before
  a place, and after any change for the six cells around it, which pop where it fails. "Support"
  is read off collision boxes: a top under the middle of the face. Cane one up and beside a change
  is asked too, so cane whose water goes breaks at once (BDS waits for a random tick).
- **Shapes** (`blocks/shapes.rs`, part B). After any change the block and the four beside it (and
  a wall under it) take the state their neighbours decide: fences, panes and bars
  `minecraft:connection_*` (to their own kind, a gate across their line, a full block other than
  leaves, pumpkins, melons, shulker boxes); walls BDS's `tryFixWallStates` (an arm is tall where
  the block above reaches into it, the post rule for straight and crossing lines; a changed wall
  makes the wall under it look again, since a post raises one under it); stairs
  `minecraft:corner` by the Java game's `getStairsShape`; a fence gate's `in_wall_bit`. Who joins
  whom is the Java game's lists; outlines are collision boxes.
- **Rails** (`blocks/rails.rs`). A placed rail starts along the look, then takes the shape the
  Java game's `RailState.place` gives it among the rails around it (curves for plain rails,
  rising to a rail one up), and those rails turn to it. Taking one away leaves the others. No
  power: `rail_data_bit` stays 0, junctions never switch.
- **Opening, and doors' two cells** (`blocks/doors.rs`). An allowed click on a door, trapdoor, fence gate
  or lever (not an iron one) flips its `open_bit` for everyone, on both cells of a door; a button goes in
  for 20 ticks if of stone, 30 otherwise (BDS `ButtonBlock::buttonPressed`). Each is heard:
  `LevelSoundEvent` `button.click_on` / `button.click_off` (levers too, as BDS's `LeverBlock::toggle`),
  `trapdoor.open` / `.close`, `fence_gate.open` / `.close`, `door.open` / `.close`, with the block's
  network id as extra data. The names are strings in the server binary except the door's, which follow
  the pattern. A block with an `upper_block_bit` needs the cell above free, is placed with its upper
  cell, and loses its other cell when one is broken (one drop). No redstone; beds are one cell; a
  gate does not swing away from its user.
- **Sleeping** (`sleep.rs`, `docs/research/beds-and-containers.md` sections 1 to 4). An allowed
  click on a bed checks as BDS's `BedBlock::use_sleep` does: occupied by another sleeper
  (`tile.bed.occupied`; a stale `occupied_bit` is cleared), eyes more than 3, 4, 3 blocks from
  the head's corner (`tooFar`), then the bed becomes the player's spawn (`SetSpawnPosition`, and
  `respawnSet` when it is a new one), then only between ticks 12541 and 23459 of the day
  (`noSleep`). Messages are BDS's grey JSON `Text`. Lying down: the head gets `occupied_bit`, the
  player's `PlayerFlags` sleep bit and `PlayerBedPosition` go to it and its viewers, and it lies
  at the head facing along the bed. It gets up on `PlayerAction StopSleeping`, when its bed is
  gone or day comes: `Animate WakeUp`, the bit and flag cleared, put back where it lay down from.
  When every non-spectator of a world has slept 100 ticks, all wake and the time jumps to the next
  day's tick 0. Whenever someone lies down or gets up, the world's players are told how many sleep
  of how many: `LevelEventGeneric` 9801 with the Ints `ableToSleep`, `overworldPlayerCount`,
  `sleepingPlayerCount` (BDS `updateSleepingPlayerList`). Not done: monsters near the bed (mobs
  are statues), weather, BDS's stand-up search, and the spawn is never used.
- **Ground truth for what follows**: `cargo build -p sapling --example bds_mechanism_probe`, then
  `tools/bds-drill.sh <bds dir> <port> bds_mechanism_probe <out>` runs drills (falls, plates, buckets,
  lava, waterlogging, the odd cases too) on a BDS with a fresh world, and `tools/sapling-drill.sh
  <port> bds_mechanism_probe <out>` the same on sapling's `drill_server` example (offline, flat
  ground, everyone an operator, vanilla commands, nothing protected). The runs and what they showed:
  `research/captures/`.
- **Vanilla commands** (`sapling::vanilla`, opted into with `ServerBuilder::vanilla_commands`; on the
  plugin API alone). For operators: `/setblock <pos> <block> [states]` and `/fill <from> <to>
  <block> [states] [replace [block] [states]|destroy|keep|hollow|outline]` (32768 blocks at most,
  counting the blocks it changed) write with `ctx.update_block`, which settles neighbours
  as a player's place does (`ctx.set_block` still settles nothing); `/clone <begin> <end> <dest>
  [replace|masked] [normal|force|move]` or `filtered <mode> <block> [states]` copies blocks with
  their block entity (`ctx.block_entity`, opaque to plugins) and `/testforblocks [all|masked]`
  compares (`vanilla/regions.rs`, `copying.rs`); `/tp [victim] <pos>` moves
  players and entities; `/give <player> <item> [amount] [data] [components]` (old coloured names
  take their colour from data; `keep_on_death` and `item_lock` components), `/clear [player] [item]
  [data] [maxCount]`, `/enchant <player> <id|name> [level]` (the table in `sapling_data::enchant`;
  `Item` carries enchantments, `keep_on_death` and `item_lock`), `/gamemode <mode>
  [player]`, `/summon <kind> [pos]` (`ctx.spawn_entity`) and `/kill [target]` (`ctx.kill_entity`),
  which leaves players alone (no damage). `/setworldspawn`, `/spawnpoint` and `/clearspawnpoint`
  (`ctx.set_spawn_point`, kept as a bed spawn is), `/testfor`, `/spreadplayers` (`vanilla/spawns.rs`,
  `selecting.rs`), `/difficulty` with letters and 0-3, and `/setmaxplayers` (a `PlayerLimit` the
  listener reads, up to the limit it started with), `/transfer <name> <server> [port]`
  (`ctx.send_to_server`, the Transfer packet; `ctx.transfer` moves between this server's worlds),
  `/defaultgamemode` (this shard's newcomers), `/seed` and `/version` (`vanilla/session.rs`) answer
  as BDS did (`bds_command_probe worldcmds|itemcmds`, research/captures/bds-worldcmds.txt,
  bds-itemcmds.txt, bds-console-cmds.txt; BDS has no `/defaultgamemode`, `/seed` or `/version`,
  and keeps `/setmaxplayers` and `/transfer` for its console; unverified details:
  docs/research/world-commands.md). Positions take `~` and `~<offset>` from the sender;
  whole numbers are the middle of their block across, as Bedrock reads them. Targets: a name or
  any Bedrock selector (`@s @p @a @r @e @n @initiator`) with its filters, see "Selectors" below. Block states come as typed, `["liquid_depth"=3]`, and a
  block named with some of its states takes the rest from its first state. The client is told the
  parameters as BDS types them (position 65, block position 64, block states 84). `/gamerule`
  lists, shows and sets the world's rules with BDS's answers, bounds and errors
  (`research/captures/bds-gamerule.txt`); its rule names are not completed. `/say <message>`
  (operators) announces `[name] message` to the shard (`ctx.announce`); `/tell`, `/w` and `/msg <target>
  <message>` (anyone) whisper to each target (`ctx.whisper`), as BDS sent them
  (`research/captures/bds-feedback.txt`). `/me` (anyone), `/tellraw`, `/title`, `/titleraw`,
  `/playsound`, `/stopsound` and `/particle` answer and send as BDS did (`vanilla/messaging.rs`,
  `titles.rs`, `sounds.rs`; `bds_command_probe messaging`, research/captures/bds-messaging.txt):
  `/me` and `/tellraw` go unanswered when they work; a rawtext selector (any selector, through
  `ShardCtx::select`) names who it finds from the sender, `, `-joined, or is dropped; a score part
  the scores of those it names that have one, `, `-joined, or is dropped; `*` is the reader, kept raw
  for a reader without one (`/titleraw`: always kept raw, unverified). `/playsound`
  with no place plays at each target's eyes; out of range (16, or 16 per unit of volume over 1) it
  is moved 2 blocks from the hearer at `minimumVolume`, or not played (Java's rule; BDS's answers
  and positions fit it). Sound and particle names are not checked, as on BDS.
  `/scoreboard` (BDS's 11 overloads, every subcommand) and `/tag` (`vanilla/scoreboard.rs`,
  `scoreboard/`, `tag.rs`; `bds_command_probe scoreboard|scoreboard2`) answer under BDS's keys
  (docs/research/scoreboard.md); a name is the online player of that name or a fake player, `*`
  everyone with a score.
  `/kick <target> [reason]`, `/op` and `/deop` (operators,
  `vanilla/operators.rs`) answer under BDS's keys; with access lists `/op` and `/deop` save the role
  of a player with an XUID, and a joining player gets the role `permissions.json` has for them.
  `ServerBuilder::access` adds `/allowlist` (`/whitelist`: add, remove, list, on, off, reload; on and
  off not saved, as on BDS) and Java's and PocketMine's `/ban <name> [reason]`, `/ban-ip <ip|player>
  [reason]`, `/pardon`, `/pardon-ip` and `/banlist [players|ips]` (BDS has none; operator-only,
  answered under the client's old `commands.ban*` keys, `vanilla/bans.rs`). Not done: the rest of
  vanilla's commands; `/give`'s `can_destroy` and `can_place_on` (stacks do not carry them); `/permission list|reload`; a ban's expiry from a command (the API takes one);
  `/kick` and `/op` reach players of the sender's shard only; `whitelist.json` is not read in place
  of `allowlist.json` as BDS does.
- **Selectors** (`sapling-api/src/command/selector/`). Parsed into a `Selector` (`Named` or
  `Variable(Base, Filters)`); `ShardCtx::select(sender, &selector)` is the one resolver, used by every
  vanilla command and open to plugins (`select_from` counts from another place: `/execute at`). Bases: `@p` and `@n` the nearest (players / everything),
  `@a` players and `@e` everything in join then creation order, `@r` a random player (any entity
  once `type=` is given), `@initiator` nobody (no NPC dialogues). Filters: `x y z` (with `~`),
  `dx dy dz` (whole blocks from the origin), `r rm`, `c` (nearest first, negative farthest; `@r`
  random), `name type family tag m` (each with `!`, each one that is given must pass), `l lm`,
  `rx rxm ry rym` (yaw round past 180), `scores`, `hasitem` (item, data, quantity range with `!`,
  location and slot), `haspermission`. `r`, `l`, `rx`, `ry` take a range too (`r=..5`). Brackets may
  hold spaces. An unknown key, a repeated one-only key, a `!` where none goes or a bad value is a
  `commands.generic.syntax` pointing at the key or value. Families come from the behaviour packs
  (`sapling-data` `mob_families`); tags from `ShardCtx::add_tag` (saved, `tags.rs`); `scores` from the
  world's board, a holder without the score failing even with `!`; entity rotation is what viewers were
  last told. Not done: no `/inputpermission`, so every input is enabled; entities have no name tags (`name=` sees an empty
  name) and no inventory for `hasitem`; selectors do not look into other dimensions apart; `Target`
  parameters still take only `@s` and names. Not checked on BDS (no capture has selectors beyond
  `type`): the error tokens, `@r` taking entities with `type=!x`, `m` numbers 5 and 6, whole-block
  volumes, unnamed entities' `name`, `hasitem` with no location counting armour and offhand,
  ranges on `r`/`l`/`rx`/`ry`, `c=0` refused, `name=` and `type=` matched in any case.
- **Neighbours** (`blocks/updates.rs`). After a player's break or place: falling blocks around are
  queued to fall (next bullet); what stood on a block that is gone (the Java game's `canSurvive`
  blocks: plants, rails, carpets, doors, standing torches and signs, candles, floor buttons and levers)
  and what hung on it (wall torches, ladders, buttons, levers, wall signs and banners) breaks and
  leaves its loot; each of these is followed up, 512 cells at most for one change. Support is "the
  cell is solid", where the game asks the face. `ctx.set_block` settles nothing.
- **Falling blocks** (`blocks/falling.rs`, `falling/flight.rs`). Sand, gravel, concrete powder, anvils,
  the dragon egg and snow layers over a cell that is free (gives way, a snow layer, coral) fall: a
  placed one in its tick, one whose block below changed a tick later, others two (BDS `FallingBlock`,
  and the capture). The lowest of a stack goes first. The cell becomes air and a
  `minecraft:falling_block` entity (`Variant` = the block's network id, box 0.98) drops by BDS's
  gravity 0.04 and drag 0.98 as a body (`body.rs`, kept up by `sync.rs`), shown a tick into its
  fall at the box bottom + 0.49. Where it comes to rest
  on a collision box it is placed a tick later and removed three after that; a cell that does not
  give way (a torch, a slab) or a brushable block breaks it at once: particles at the entity and the
  block's own item (a snowball for snow), nothing for brushables. 600 ticks in the air also break it;
  below the world it vanishes. Anvils thud (LevelEvent 1022 at the cell's corner). Not done:
  UpdateBlockSynced at start and landing (BDS's; acacia-proto reads its entity id as zigzag), the
  `MOVING` flag, `Health` 1, the splash into water, concrete hardening, fall damage.
- **Water and lava** (`blocks/liquid.rs`, `liquid/flow.rs`, `liquid/harden.rs`). The rule is the
  Bedrock server's own, `LiquidBlock::_tick` as decompiled with bdsre (`docs/research/water.md`,
  `buckets-lava.md`). A liquid next to a change is queued and moves after its delay, 512 cells a tick
  at most: water 5 ticks, lava 30 (10 in the nether). A flowing cell takes the least depth beside it
  plus the drop-off (water 1, lava 2, 1 in the nether; `liquid_depth` 0 source, up to 7 by
  distance; falling liquid beside counts as 0), or 8 plus the depth above it (8 to 15: falling liquid
  keeps the depth of its stream), or dries up when nothing feeds it; for water, two sources beside,
  or one beside and one above, over a floor or a source make it a source. Lava thinning out waits 4
  times as long three times in four. Then it falls into room below and does nothing else; over a
  liquid only a source goes on; on a floor it runs towards the nearest drop, looked for 4 steps out
  for water (less for depths 5 to 7) and 2 for lava, every equally near way, or every open way. Water
  stops at a block whose BDS liquid reaction is Blocking, replaces Popped (with its loot) and Broken
  ones, and takes Noreaction ones in beside them (layer 1); lava stops at blocks with collision and
  burns the rest with a hiss. The water a waterlogged block keeps in layer 1 is the cell's water to
  every rule. Water passes between two cells only through two open faces (`liquid/faces.rs`, BDS
  `liquidCanFlowIntoFromDirection`, `docs/research/liquid-faces.md`): a stair closes its back and
  its floor (its top when upside down) and a side where an inner corner fills it, a slab its floor
  or its top, sculk sensors and heavy cores their floor, a shelf its back; every other block is
  open all round. Lava ignores faces. Lava touching water through its sides or top hardens (obsidian from a source, cobblestone
  otherwise, basalt by blue ice over soul soil); lava falling onto water makes stone; each hisses
  (LevelEvent 1004 at the centre, pitch 2.6 ± 0.8 x 1000). Neither overwrites lava or its own kind.
  Not done: lava setting fire, BDS's per-block `isLavaBlocking` (collision stands in).
- **Waterlogged blocks** (`blocks/waterlog.rs`, `sapling_data::liquid_detection`). Each vanilla block's
  liquid reaction and whether it can contain liquid were read off BDS's block registration
  (`docs/research/waterlogging.md`, `sapling-data/data/liquid-reactions.tsv`). Every layer-0 write
  (`send::set`) applies BDS's `LevelChunk::setBlock` rule: a block that can contain liquid written
  over a water source keeps it in layer 1; any other block clears layer 1; air lets layer-1 water take
  the cell. Viewers are sent layer 0 only, as BDS does: their clients apply the same rule.
- **Buckets** (`blocks/buckets.rs`, BDS `BucketItem::_useOn`). A click with a bucket, unless the clicked
  block is one a click uses and the player is not sneaking. An empty bucket takes the source clicked
  (from layer 1 of a waterlogged block too; never flowing liquid; water walled in by sources and solid
  blocks on all six sides stays). A full one pours a source into the clicked block when it is water
  and the block can contain it (layer 1), else into the cell on the clicked face, or the snow layer
  under it, which is cleared; a solid block, a portal or something else in layer 1 refuses it; a
  Popped block there leaves its loot; water poured in the nether only hisses. Taking is offered to
  handlers as a `BreakEvent` and pouring as a `PlaceEvent` (both start allowed in survival and
  creative), so build protection covers buckets. Sounds `bucket.fill.*` / `bucket.empty.*` at the
  player's feet. Outside creative the hand's bucket swaps (a stack of empties: one fills into a
  free slot, or is thrown). Not done: powder snow, fish, milk, cauldrons, evaporation particles.
- **Pressure plates** (`blocks/plates.rs`, BDS `BasePressurePlateBlock`). A plate goes down the tick
  a body is in its sensitive box (1/16 in from each side, a quarter high): players for every plate
  (not spectators on plain ones), lying items for wooden and weighted ones. While down it is checked
  again every 20 ticks (weighted: 10) and comes up on the first check that finds it empty; a weighted
  plate's signal is `ceil(min(n, max) / max x 15)`, max 15 or 150. A plain plate stores
  `redstone_signal` 1 when down. `pressure_plate.click_on` / `click_off` at y + 0.1 between up and
  down, extra data the new state. Unverified: the drill pressed no plate on BDS at all, not even
  under a walking chicken (research/captures/bds-mechanisms-findings.md); no redstone behind them.
- **Tool wear** (`inventory/wear.rs`). A survival break of a block with hardness above 0 counts one use
  (a sword: two) in the `Damage` tag of the tool in the hand, where the game keeps it; the tool is gone
  when its damage would pass its durability, so a wooden tool lasts 60 uses (BDS
  `ItemStackBase::hurtAndBreak`: breaks on `max < new`). One throw in a hundred decides whether a use
  counts: always, or 100 / (level + 1) with Unbreaking (armour 60 + 40 / (level + 1), the wiki's),
  never with an `Unbreakable` tag. No break sound; `Item::damage` does not show it.
- **Enchantments** (`sapling_data::enchant`, all the rules; `enchants.rs` reads what a player wears
  and holds; `docs/research/enchantments.md`, decompiled BDS). Levels come from the stack's `ench`
  list. Sharpness (1.25 a level), Smite and Bane of Arthropods (2.5 on undead and arthropods), summed
  and floored, add to a hit before strength (`creatures/attacks.rs`, `duels.rs`); Bane slows an
  arthropod (amplifier 0, 20 + up to 10 x level ticks), Fire Aspect burns 80 ticks a level, Knockback
  throws half a block a level further (the Java game's: BDS's throw was not traced), Looting goes
  into the mob's loot table (`looting_enchant`, `random_chance_with_looting`; a player's kill also
  meets `killed_by_player`). Protection's points (Protection 1 a level on all but void, starving and a
  sonic boom; Fire, Blast and Projectile 2, Feather Falling 3 on their own), summed over the pieces,
  each take 4 hundredths off past the armour, 20 at most (`Health::warded`, every path a player is
  hurt by). Fire Protection shortens a burn by 15 hundredths a level, summed. Thorns: each piece
  rolls 0.15 a level, hurts a mob or player that hit (melee) for 1 to 4, and wears 3 when it hurt
  back, 1 when not. Respiration keeps a breath with level / (level + 1). Depth Strider goes into
  the server's movement step. Frost Walker freezes still water in a square of 2 + level (6 counts at
  most) around a player that moved, as frosted ice that ages and melts by the Java game's rule
  (`blocks/frost.rs`). Mending: an orb mends one damaged Mending stack worn or held, two of wear a
  point, before the rest is experience. Curse of Vanishing is gone at death; Curse of Binding keeps
  a piece on outside creative (`inventory/enchanted.rs`). Power, Punch, Flame, Infinity, Multishot,
  Piercing, Quick Charge, Loyalty, Riptide and Impaling: see "Players' shots". Not done: Channeling,
  fishing rods and the mace and their enchantments; Aqua Affinity (the server does not slow a break
  under water); Soul Speed, Swift Sneak; Thorns against arrows; Blast Protection's throw (BDS reads
  none); Looting on armour.
- **What a block leaves** (`blocks/loot.rs`, `sapling_data::block_loot`). `BreakEvent::drops` starts as
  what the game's loot table for the block gives, in survival when the tool in the server's hand
  harvests the block; empty otherwise. The tables are the Java game's (`data/block-loot.tsv`, made from
  its jar; sapling-data's README says how): Bedrock keeps most block loot in code, and the 99 tables
  its server does ship matched Java's where sampled. A row drops when its tool condition (Silk Touch,
  shears, neither), its block states and its chance hold; counts and Fortune follow the game's three
  formulas. Rows name Java block states, read off the Bedrock block where that is known: a crop's `age`
  from `growth`, a double slab from its block name, counts that start at one (candles, snow layers,
  sea pickles) from Bedrock's that start at zero; a row about any other state is skipped. Handlers may
  change the list. Each stack is then tossed up out of the emptied cell as an item (next section), to
  be picked up after 10 ticks. Stems leave nothing, a bed drops per cell broken, and the dice are a
  fixed-seed xorshift.
- **Limits.** 16 block actions and 4 clicks a player a tick; an input's action list is cut to 16. What
  exceeds them is dropped without an answer. Absurd positions (the edges of `i32`, outside the height)
  are refused before anything is computed from them.
- **API.** `ctx.block(pos) -> Option<Block>` (`None` where not loaded or outside the height) and
  `ctx.set_block(pos, &block) -> bool`, which shows the block to its viewers and raises no event. The hub's
  whole protection is `if !may_build(ctx, player) { event.cancel() }` in `on_break` and `on_place`, with
  `may_build` = operator and in creative mode.

## Dimensions and portals (as built)

Details and BDS evidence: docs/research/nether-dimension.md, docs/research/end-dimension.md.

- **A dimension is a layer.** A world's Nether is a `sapling_world::World` of dimension 1 (its own storage
  keys, I/O threads, load queue, generator, columns, mobs and saves) added to the overworld's shard
  (`Shard::add_dimension`, `ServerBuilder::nether` / `dimension_of`) as `<world>:nether`. The layers of one
  overworld are a `Realm` (sapling-shard `dimensions.rs`); the others copy the overworld's `Level` each tick,
  so time, rules, weather and portal records are the overworld's, and only it writes `level.dat`.
- **Dimension change** (`dimensions::change`): RemoveEntity of all seen, ChangeDimension with a counted
  loading screen, the sleepers' count to all; inputs held until the client's DimensionChangeAck or
  loading-screen end; BDS's own PlayerAction DimensionChangeAck once the spawn-radius columns are sent.
  `ctx.travel(player, dimension, feet)` and `/execute in <dimension> run tp` use it; a world move into another
  dimension's world (`transfer`, handoff) does too. The player record keeps `DimensionId` and `Pos`, and
  StartGame waits for it so a player joins in the dimension it left from (`dimensions::start_in`).
- **Portals**: blocks in `blocks/portals.rs` (frame shape, lighting by flint and steel, fire charge or any
  fire, breaking, records), travel in `portals.rs` (BDS's timer and cooldown, 8:1 scaling, record search
  within 128, placement and portal making in `portals/arrive.rs`, MT19937 direction draws from the seed).
- **The End** (docs/research/end-dimension.md): a layer of dimension 2 (`Generated::end`, `WorldFolder::end`,
  `ServerBuilder::end`), y 0..255. An eye in the last of twelve frames facing a 3x3 fills it with end portal
  (`blocks/end_portal.rs`); a body in one goes to the End at once, onto (100, 50, 0), with BDS's obsidian
  platform built under it the tick after (`portals/end.rs`); a generated gateway sends a body over its exit
  (`portals/gateway.rs`); the dead of the End and the Nether rise in the overworld (`dimensions::respawn_home`).
  A thrown eye of ender flies to the stronghold its world's storage locates (`WorldStorage::locate_stronghold`,
  only `Generated` overworlds; `eyes.rs`).
- **Players' shots** (sapling-shard `shots.rs` and `shots/`, `sapling_data::shots`,
  `sapling_data::enchant` archery, docs/research/projectiles.md). A click in the air throws a snowball,
  egg, ender pearl, splash or lingering potion, bottle o' enchanting or wind charge, shoots a loaded
  crossbow, or starts drawing a bow, loading a crossbow or readying a trident (`Drawing`); `ItemRelease`
  looses it. Every shot, a mob's or a player's, flies by `creatures/flight.rs` (one `Shot` table per
  kind), hurts by `creatures/hits.rs` and lands by `creatures/landing.rs`. A player's carries `Loosed`
  (Power, crit, Punch, tip, Piercing, Impaling, Loyalty, who may pick it up). A stuck player's arrow or
  trident is kept with its column (sapling-leveldb `actors/stuck.rs`) and picked up as an item is
  (`shots/pickup.rs`). A loaded crossbow may hold a firework rocket (`fireworks.rs`); a Channeling
  trident calls lightning down on what it hits while it thunders (`lightning.rs`). Not done: drowned tridents.
- **Lightning, fireworks, the wither** (sapling-shard `lightning.rs` and `lightning/`, `fireworks.rs` and
  `fireworks/`, `creatures/wither.rs` and `creatures/wither/`, `blocks/summoning.rs`;
  docs/research/wither-lightning-fireworks.md). A bolt is an entity that strikes once on its first tick
  (`creatures/struck.rs` for mobs) and lives through its flashes; storms pick columns near players,
  drawn to rods by `lightning/rods.rs`; what the strike does to a rod and copper is
  `blocks::lightning_struck`'s. Rockets fly by their own tick and burst by `fireworks/burst.rs`;
  dispensers launch them by `fireworks::dispense`. The wither is a creature whose `Wither` part steers it ahead of hunting; `damage::ask` asks
  `creatures::spares` first, so its charge, armour and own blasts hold for every kind of harm.
- **The dragon fight** (sapling-shard `dragon.rs` and `dragon/`, docs/research/end-dimension.md): a `Fight`
  component on the End layer, its record in the realm's `Level::dragon_fight` (LevelDB `TheEnd`, sapling-leveldb
  `dragon_fight.rs`). Run from `handlers` after the creatures' tick: dragon lookup and making, podium, crystals,
  the death, gateways and their exits (`exits.rs`, BDS's spawn searches in `exit_spawn.rs`), the summoning again
  (`respawn.rs`), boss bar, the strafe's fireball (`fireball.rs`) and the breath clouds (`cloud.rs`). The dragon is an entity
  with a `Dragon` component flown by BDS's goals (`flight/`, `fly.rs`, `parts.rs`, `wire.rs`), not by
  `creatures::tick`. The exit portal's way home with the credits is `portals/exit.rs`. Plugins read
  `ShardCtx::dragon_fight` and get `DragonKilledEvent` and `DragonRespawnEvent`.

## Inventories (as built)

- **The server's copy rules.** Every player has an `Inventory` (36 slots, hotbar first; 4 armour; offhand;
  cursor). Each stack has an id the client names it by. A whole stack keeps its id when it moves; a part
  split off gets a new one; a merge keeps the id of the stack merged onto.
- **Join.** After the world state packets: `InventoryContent` for windows Inventory (36), Armor (5, the
  last is the body slot), Ui (54, the first is the cursor) and Offhand (1), then `PlayerHotbar`
  (`select_slot` true). Shapes are BDS's recorded ones (`extract-data --show InventoryContent,PlayerHotbar`).
  They go out once storage has answered for the player, which in a world without storage is the StartGame tick.
- **Requests.** One `ItemStackRequest` is done whole or not at all, on a copy first. Take and Place move
  a count into an empty slot or onto a stack of the same kind (up to 64: an item's own limit is the
  client's to keep); Swap; Drop; in creative mode Destroy and CraftCreative, which puts 64 of the entry
  (`CreativeContent.bin`) into the created-output slot (50) until the request ends. A slot named with a
  positive stack id must hold that stack. Done: `ItemStackResponse` Ok with every slot the request named.
  Anything else (crafting, a wrong id, more than is there): Error, then all four windows as they are.
  32 requests a player a tick.
- **Others see** the hand in `AddPlayer`, then `MobArmorEquipment` and an offhand `MobEquipment` if there
  is something to show; later changes go to viewers as `MobEquipment` / `MobArmorEquipment`, once a tick.
  A client's own `MobEquipment` only moves the selected slot (0 to 8).
- **Screen.** `Interact OpenInventory` is answered with `ContainerOpen` (type Inventory, the player's
  block, entity -1 as BDS's `ServerPlayer::openInventory` passes, window ids counting up from 2), and
  every `ContainerClose` is echoed: clients wait for both.
- **Thrown items** (`drops.rs`, `docs/research/items.md`). A Drop raises `on_drop` (cancellable;
  cancelled, the request is refused). The stack is thrown from half a block under the eye at 0.3
  along the look, give or take 0.02 (BDS `Player::drop`); a broken block's items appear at a random
  point 0.15 to 0.85 into its cell, tossed up at 0.2 and up to 0.1 aside (`Spawner::spawnItem`). An
  item is a body (next bullet) 0.25 across that slides on the ground and does not bounce, shown
  through the mob grid at the middle of its box, gone after 6000 ticks. After 40 ticks (10 for a
  block's) a player whose box, grown by 1 across and 0.5 up and down, touches it and who has room
  takes it, or as much as fits: `TakeItemEntity` to those who see the column, the entity is removed,
  the taker's windows are resent. Not done: merging, floating in water, burning in lava, being saved;
  an item stays in the mob grid's cell for the column it was let go in.
- **Bodies the server moves** (`body.rs`, `sync.rs`). Falling blocks, items and orbs share one body:
  gravity (0.04), then the move, cut short by the collision boxes of the blocks in its way (up and
  down first, then across, as the game does), then drag (0.98 in the air; on the ground 0.588 across
  and the kind's bounce). Viewers are kept up as BDS's generic actor sync does: a `MoveEntityDelta`
  of the axes moved more than 0.01 (tick fields 0), a `SetEntityMotion` when the motion changed by
  more than 0.2 since the last one or stopped; a falling block on BDS was sent exactly that. Not
  done: water and lava, being pushed out of a block.
- **Entities and their API** (`entities.rs`, `creatures.rs`; `sapling_api::EntityId`, `EntityInfo`).
  Whatever moves has a `Kind`, a `Body` and what its viewers were `Told`, beside its own part
  (`Dropped`, `Orb`, `FallingBlock`, `Creature`); the generic code finds, lists, moves and removes
  any of them by those alone, and files each under the column it is in every tick, so it is shown
  and unloaded with that column. `ctx.spawn_entity(kind, feet)` makes a mob of a kind BDS was
  recorded announcing (`Mobs.bin`; its box from there), in a loaded column; it falls and slides with
  the Java game's living-entity physics (gravity 0.08, drag 0.98 down and 0.91 across, 0.546 on the
  ground) and does nothing else. `ctx.entity(id)`, `entities()` (the world in focus, oldest first),
  `move_entity`, `set_velocity` and `remove_entity` reach every kind: items, falling blocks and orbs
  too. `kill_entity` kills a mob as BDS's `/kill` does: DeathAnimation, its loot (`sapling_data::mob_loot`,
  BDS's tables, under `doMobLoot`) at its feet, gone 21 ticks later; anything else, and an armor
  stand, goes at once. A spawned mob is heard (`spawn` at its head, docs/research/spawn-sound.md).
  Mobs press every pressure plate, items and falling blocks wooden and weighted ones, orbs none.
  A spawned mob is saved with its column as a new actor (`UniqueID`, `identifier`, `definitions`,
  `Pos`, `Rotation`, `Motion`: what BDS reads of one is untried). Mobs also appear by themselves
  around players by the vanilla packs' spawn rules, capped by category, and go again by their kind's
  despawn distances; spawn eggs hatch them (`spawning.rs`, docs/research/mob-spawning.md). Not done: AI, damage; saving the
  items players throw, orbs and falling blocks; a kept entity that wanders into a column that is not
  loaded is lost with it.
- **Husbandry** (`creatures/husbandry`, `sapling_data::husbandry`, docs/research/husbandry.md). Every
  creature has a `Raised`: what the world folder keeps of its raising (`sapling_world::Raising`, BDS's
  actor fields, written back with its compound) and what lives only while it is loaded (its lead's
  holder, its partner, its foe, its next egg). A player's click on an entity (`actions::interacted`,
  after plugins) goes to the first that takes it: a vehicle's own use, raising, a trader's screen,
  then tack or a seat (riding). Raising does what the mob's packs say of the held item (`clicks.rs`):
  name tags, leads, shears (a dispenser's too, `shear_in`), buckets, bowls, flowers, dyes, brushes,
  healing, growing a baby, breeding, taming, temper, sitting.
  Before a mob's move, its raising may steer it (sit, be pulled on its lead, fight for its owner,
  court, follow its owner, follow food) ahead of strolling. Its flags, scale, wool, name, owner and
  lead holder are told as they change and shown to new viewers (`creatures/shown.rs`); a kept
  `definitions` list gains and loses the groups each event swaps. Riding keeps a mob's saddle in
  `Raised::saddled` and reads its taming; getting on a wild mount calls `owners::tame_by_riding`.
- **Kept between visits** (`sapling-leveldb/src/player`, docs/research/player-records.md). The first
  world's storage keeps a player where BDS does: `player_<uuid>` maps it to a `player_server_` compound
  with BDS's fields: items (`Inventory` with `Slot`, `Armor`, `Offhand`, `SelectedInventorySlot`,
  `EnderChestInventory`), `PlayerLevel`/`PlayerLevelProgress`, health and hunger as `Attributes`,
  `ActiveEffects`, the bed (`SpawnBlockPosition*`, `Spawn*`, `SpawnDimension`), `DimensionId`, `Pos` (at
  the eyes, as BDS), `Rotation` and `PlayerGameMode`. Fields sapling does not know stay as stored. A record
  under sapling's old `sapling_player_<uuid>` is read while no vanilla one exists and dropped at the next
  save. A player joins where it left, facing the same way, in its own game mode; a place in the void or
  past the world's edge, or in another world than the entry shard's first, starts it at the spawn. Loads
  and saves run on the world's I/O threads; a save happens when the player leaves, every 100 ticks while
  something of it changed (its place too), and when the shard stops. One save per player is in flight at a time, and a player back before it lands
  is given what is being saved. A record that cannot be read is never written over.
- **Chests and barrels** (`containers.rs`). 27 slots, kept where the game keeps them: the `Items` of the
  block's entity, so a world's own chests open with what they hold. A placed chest gets an empty entity
  (`BlockEntityData` to viewers); clients are never sent `Items`, in chunks or otherwise. An allowed
  click opens it: `ContainerOpen` (type Container, the block, entity -1) and `InventoryContent` for that
  window, and the first viewer raises the lid (`BlockEvent` ChangeState 1, 0 when the last one closes).
  While open, the slots live in the shard under stack ids from 2^24 up; a request reaches them as
  container `Container` or `Barrel`, borrowed into the player's inventory for the length of the request,
  and an accepted one is written to the block entity and sent to the other viewers as one
  `InventorySlot` per changed slot (BDS `ServerPlayer::slotChanged`). The open lid is shown again every
  80 ticks (BDS `ChestBlockActor::tick`). A barrel's block takes `open_bit` while open and sounds
  `block.barrel.open` / `block.barrel.close`. Breaking the block spills its items and closes its screens
  (`ContainerClose`, server true). The evidence for all of it, mostly decompiled from an older BDS
  build with its symbols, is `docs/research/containers-crafting.md`; no chest traffic of a real BDS
  was ever recorded. A block a handler sets gets its empty entity too. A chest, trapped chest or
  ender chest does not open while a block above either half collides with the space its lid needs
  (BDS `ChestBlockActor::_canOpenThis`: x and z 1/16 to 15/16, half a block up; slabs and glass hold
  it down, torches do not), and a screen closes (`ContainerClose`, server true) on a player whose
  eyes are further from the block's middle than a click reaches (`containers/access.rs`; BDS's own
  distance was not found). Not done: a chest's own sounds (their names in this protocol were not
  found), sitting cats on chests.
- **Shulker boxes and ender chests** (`containers/shulker.rs`; `docs/research/shulker-ender.md`). A
  shulker box is a chest of 27 (request container `Shulker`, entity `ShulkerBox` with a `facing` byte:
  the face clicked, from memory) that refuses shulker boxes. Broken, it spills nothing: it drops
  itself with its items in the item's tag as `Items` (in creative only when it holds something), and
  the loot table drops nothing more; placed, the tag's `Items` become the new entity's. An empty box
  is tagged `Items: []` (a list of no type), as recorded from BDS, so it does not stack with new
  ones. An ender
  chest (entity `EnderChest`, request container `Container`) opens its player's own 27 slots, kept
  with the player's items as `EnderChestInventory`; the lid is the block's, shared by its viewers.
  Both sound (`shulkerbox.open`/`.closed`, `enderchest.open`/`.closed`, names from BDS's binary).
- **Ticking block entities** (`ticking.rs`). Furnaces, hoppers and campfires tick while a player's
  view covers their column, as BDS ticks block actors near players. On the column sweep (every 100
  ticks, before saving) those in seen columns with work start (a furnace whose entity says it was
  burning or cooking, every hopper, a campfire with food on it) and those in columns nobody sees stop, their state written to their entity.
  A placed hopper and an opened furnace start at once. Monster spawners, trial spawners and vaults
  (`spawners.rs`, docs/research/spawners.md) tick in seen columns too, every one, from the sweep or at
  once when a plugin or a spawn egg sets them: a spawner spawns its kind near players by BDS's rules, a
  trial spawner runs its waves and ejects rewards, a vault takes a key once per player and ejects its
  loot (BDS's tables, `sapling_data::table_loot`). A ticking container's slots stay held
  (`containers::pin`); hoppers reach other containers through `containers::with_slots`.
- **Hoppers** (`hoppers.rs`; `docs/research/hoppers.md`). A container of five. Each tick its cooldown
  counts down; when it is over and the hopper is not powered (`toggle_bit`), it pushes one item from
  its first slot that can give one into the container it faces (`facing_direction`), then pulls one
  from the container above or, when the block above is no container, takes the first item lying in
  the box 0.625 to 2 blocks over it, as much as fits in two slots. A move sets the cooldown to 8
  ticks (recorded from BDS: an item every 400 ms into a chest; its code says 4 of its hopper
  ticks, so those are two game ticks each); an empty hopper that is given an item waits a whole cooldown. A furnace takes from above into its
  input, from the sides only fuel, and gives its result (and an empty bucket in its fuel slot)
  downwards; ender chests are not reached. Not done: BDS ticks the hoppers below first, so a chain
  moves an item in one tick; composters; minecarts.
- **Riding** (`riding.rs`, `docs/research/vehicles.md`). A vehicle's `Seats` list its riders, the
  first drives; a rider has `Riding`. Seating tells viewers BDS's `mount` sound, the seat's
  `SetEntityData` and a `SetEntityLink`; later viewers get them after the vehicle's announcement
  (`mobs::show_after`). A player gets on by a click no raising took (`riding/mounts.rs`: saddles and
  horse armour go on first; a wild mount's temper may tame it or it throws the rider off), leaves on `Interact LeaveVehicle` (`inventory::screens` takes it in); a player rider's
  input goes into `Reins` instead of moving it (movement.rs), and riders are carried to their seats
  each tick (`riding/carry.rs`). A driver's client moves a boat or a tamed, saddled horse
  (`riding/drive.rs`, trusted); a pig or strider walks the way its rider looks while the rider
  holds its stick (`creatures/legs.rs`). A vehicle's `LinksTag` is written and read
  (`adopt`). Plugins and `/ride` go through `ShardCtx::ride` and its kin (`riding/backend.rs`).
- **Boats and minecarts** (`vehicles.rs`, `vehicles/*`). Placed from their items (`blocks/request.rs`),
  hit until their integrity goes and dropped as their item, broken by blasts. Empty boats float with
  acacia-physics' `boat_tick`; carts roll by BDS's rail rules (`vehicles/rails.rs`), fall off them,
  are pushed, press detector rails and are acted on by activator rails (`vehicles/carts.rs`).
  Chest boats and chest and hopper carts carry slots on a screen naming the vehicle
  (`containers::Screen::Carried`, `vehicles/cargo.rs`); boats, minecarts and camels pull in mobs.
- **Furnaces, blast furnaces and smokers** (`furnaces.rs`, `sapling_data::smelted`;
  `docs/research/furnaces.md`). Containers of three slots (input 0, fuel 1, result 2; window types
  Furnace, BlastFurnace, Smoker), which requests name by slot type (`FurnaceIngredient` and its two
  kin, `FurnaceFuel`, `FurnaceOutput`). A ticking one (above) ticks in BDS's order
  (`FurnaceBlockActor::tick`): while lit it crackles every 20 to 99 ticks (`block.furnace.lit`,
  `block.blastfurnace.fire_crackle`, `block.smoker.smoke`) and the fire burns down; an unlit one with
  something it can smelt lights its next piece of fuel (a lava bucket leaves its bucket); a lit one
  cooks, and at 200 ticks (100 in the other two) the result grows, shown to the screens before the
  input shrinks; an unlit one loses two ticks of cooking a tick. Recipes are the `furnace`,
  `blast_furnace` and `smoker` ones of the recorded `CraftingData`. Screens are told the three times
  as `ContainerSetData` (0 cooked, 1 fire left, 2 what the fuel gave) when they change. Lighting and
  going out swaps the block for its `lit_` one, facing kept, and gives it the old entity
  (`BurnTime`, `CookTime`, `BurnDuration`, `StoredXPInt` beside `Items`). Burn times are BDS's
  `getBurnDuration` rules (`furnaces/fuel.rs`; wood is told by name where BDS reads a material),
  truncated. A player may not put what does not burn into the fuel (BDS answers status 55; sapling
  Error); into the result it may put anything. Both as recorded with stone.
  Taking results pays `count x` the result's share (`furnaces/xp.rs`) plus what hoppers left owed
  (`StoredXPInt`, hundredths), each rounded at random, as orbs at the player's eyes; breaking pays
  the result's at the block.
- **Experience** (`experience.rs`, `orbs.rs`; `docs/research/experience.md`). Orbs split as BDS's
  `spawnOrbs` (2477 down to 1; 13 equal ones above 12000), thrown a little up and aside, shown as
  `minecraft:xp_orb` with `ExperienceValue` and moved by the server as a body that bounces (kept up
  by the actor sync: a `MoveEntityDelta` of the axes that changed every tick it moves, as recorded;
  the client does not move them). One falls, slides to a stop, and from 21 to 120 of its ticks looks for the nearest
  player within 8 blocks every 20 and flies to it; a player whose box, grown by a block across and
  half a block up and down, overlaps its box takes it, with no age gate, also before it looks
  (BDS's `Player::aiStep` pickup area, `drops::within_pickup`, docs/research/assumptions-checked.md 3)
  (`LevelEvent` SoundOrb). A player's level and progress follow BDS's
  `addExperience` (the Java formula), go to its client as the `player.experience` attribute, with
  `player.level` when the level changed (as recorded), and are kept
  with its items (`PlayerLevel`, `PlayerLevelProgress`); a fifth level sounds `levelup`. Orbs join
  up as BDS's (`orbs/merge.rs`) and are kept with their column at what they are worth. An orb
  mends a Mending stack first (Enchantments).
- **Double chests** (`containers/pairs.rs`). Two chests of one kind, side by side and facing the same
  way, name each other in their block entities (`pairx`, `pairz`, `pairlead`), which clients are sent
  and draw one large chest from. A chest placed beside a single one pairs with it, the old one leading
  (BDS `ChestBlockActor::onPlace`); the pair opens as one window of 54 at the lead half, slots 0 to 26
  the lead's, each half keeping its own 27. Breaking a half spills that half and leaves the other a
  single chest.
- **Crafting** (`inventory/crafting.rs`, `sapling_data::recipe`). Clients are sent BDS's recorded
  `CraftingData` and `UnlockedRecipes` with the registries. A request that starts with `CraftRecipe`
  (by hand) or `CraftRecipeAuto` (the recipe book) names a recipe by its network id. `Consume` actions
  use items up, and the request is done only if what they used is exactly what the recipe's cells take
  (an item by name, or any item carrying a tag; no packet carries the tags, so their members come from
  `data/item-tags.tsv`, the Java game's tags checked against a dump of Bedrock's). The result lies in
  the created-output slot (50), one stack at a time: what a shift-click makes beyond a stack follows as
  the request empties the slot (BDS keeps the surplus pending the same way). Of a recipe with several
  results (cake, honey block, sugar from honey bottles) each is laid there by a `Create` action, which
  needs the slot empty. Screens' own slots are the `Ui` window's: the grids' cells 28 to 40
  (`CraftingInput`), the stonecutter's input 3; what is left in them goes back among the items when a
  screen closes. A recipe that does not fit 2x2 needs a crafting table's screen (`ContainerOpen`, type
  Workbench), a stonecutter's recipe its own (type Stonecutter); a click on the block opens it. A
  `CraftRecipe` in a grid is held to the grid as BDS holds it (`_handleCraftAction`): the recipe must be
  laid out in the cells (`Recipe::is_laid_out`: a shaped one anywhere, mirrored or not, the mirroring
  from memory of the game and not decompiled; a shapeless one in any cells; nothing beside it), and the
  request must consume the crafts' count from each filled cell and from nowhere else. A
  `CraftRecipeAuto` is held only to what it used. Of the special recipes (`CraftingData`'s multi
  recipes, `sapling_data::is_special_recipe`) only fireworks are made: the grid's stacks are read by
  `fireworks::crafted` into a recipe of one craft. Not done: maps, banners, dyed armour and the rest.
- **Workstations** (`workstations.rs`, `inventory/forge.rs`, `sapling_data::enchant::{table, anvil,
  grind}`, `sapling_data::smithed`; docs/research/workstations.md). The enchanting table, anvil,
  grindstone and smithing table open their screens on a click, and the server works out each result
  from the screen's slots as BDS does: the request must consume exactly what that result takes. The
  table's three offers are drawn from the player's `EnchantmentSeed` (kept in its record, new after
  each enchant) and the bookshelves, and sent as `PlayerEnchantOptions` with the answer to the
  request that laid the item in. The anvil charges BDS's levels (what enchantments gain, not Java's
  sums), wears 12% of crafts out of creative and says so (level events 1021, 1020); the grindstone
  lets out experience at the player. Not done: the anvil's map recipes, `AnvilDamage` (ignored).
- **API.** `ctx.item`, `set_item`, `give` (returns what did not fit), `clear_inventory`, `held_slot`,
  `held_item`, with `Item { name, count, damage }` and `Slot::{Main, Armor, Offhand}`; every change
  resends the four windows, as BDS does after `/give`. A stack's tag stays with it but does not show.

## Who is near (as built)

`grid.rs` buckets things by cells of 8 chunks. Players are bucketed every tick and a viewer looks only at
the cells its view reaches into; packets about a player go to the viewers found by inverting the `Sees`
sets once a tick. Mobs sit in a grid that changes only when they come or go, and a player's mob set is
remade only when its view moved or the grid changed.

## Light (as built)

Sky light and block light for the server's rules (`sapling-world` `light/`); the client lights what
it draws itself, and nothing is saved (BDS saves no light), so a column is lit anew each load.

- **Data.** What a state emits and filters is acacia-world's `light_emission` / `light_filter`
  (minecraft-data corrected by the wiki's Bedrock table, not read from BDS); a waterlogged cell takes
  the stronger of both layers. Light is a nibble per cell per channel, per section, and a section at
  one level throughout holds no array: an open column holds none, a generated one about three.
- **Rules.** A step loses 1 + the filter of the cell entered; full sky light going down loses only the
  filter. The nether and the end have no sky light. `World::brightness_at` is BDS's
  `getRawBrightness` (docs/research/saplings.md): sky less `skyDarken`, or block light if brighter, 0
  inside a block of filter 15; the shard's `light::sky_darken` is the daylight detector's reckoning.
- **Work off the tick.** A loaded column is lit alone on the I/O thread that loads it (`own.rs`, a
  few ms a generated column). On the shard, `World::relight` joins each arrived column to its loaded
  neighbours (only the section faces whose levels differ), then mends the light around every block
  written since (a write is noted only when it changes emission or filter) in one flood per channel.
  Light stops at a column that is not loaded and flows in when it arrives. An unloaded column's light
  goes with it. An arena reads its template's light (lit when the template was made) until it changes
  a column's; a world without storage has open sky in every column nobody lit.
- **When.** The shard relights at the start of every tick (`chunks::poll_and_relight_worlds`) and
  again before random ticks, so a block written in a tick shows in light from the next of those.
  Plugins read `ShardCtx::light_at` and `brightness_at`.
- **Consumers.** Saplings (raw brightness 9 over them, propagules none); crops and stems (9 in
  their cell, `CropBlock::randomTick`) and bamboo (9 over it); a crop with under 8 in its cell or
  a mushroom off mycelium with 13 or more, night not counted, breaks when a neighbour changes;
  grass and mycelium spread from blocks with 9 over them and die under a block; ice and snow melt
  by block light of 12; water freezes and snow lies where the biome is cold and dim
  (`blocks/growth/`); daylight detectors read the sky light in their cell; undead burn where their
  head has the sky's full 15 by day (`creatures/harm.rs`); natural spawning's brightness filters
  (`spawning/spots.rs`). docs/research/growth.md "Light" has the rules and what is unverified.
- **Not done.** Unverified against BDS: the filter table, the sky's loss going down through a filter, and whether
  BDS lags light updates as sapling does.

## NetherNet (as built)

Off unless `ServerBuilder::nethernet` is called: a vanilla client probes `GET /v1/join` first and
only falls back to RakNet when that fails, so turning it on moves every vanilla player over.

- **One port.** RakNet and WebRTC share the game's UDP socket; HTTP signaling listens on the same
  port number over TCP. `acacia_nethernet::host::Host` claims a datagram if it is an ICE binding
  request carrying one of its connections' credentials, or comes from an address such a request
  bound; everything else goes to RakNet. An address can only be re-bound by a request that passes
  the STUN integrity check, so a spoofed datagram cannot steal a player's route.
- **Links.** `sapling-net/src/links.rs` puts both transports behind `Link`; the login sessions,
  backlog limit, kicks and `Left` reports are the same code for either. A NetherNet session starts
  when both data channels are open and stays plaintext after the handshake (DTLS encrypts).
- **Direct connect** (`http.rs`): one request per TCP connection, as BDS serves it. A TLS hello gets
  the alert that makes the client retry over HTTP. `GET /v1/join` answers the status JSON (the
  NetherNet pong); `POST /v1/join/{id}` answers the SDP with one host candidate: the address the
  request arrived on, or `NetherNet::public_ip` behind a forwarded port.
- **LAN** (`lan.rs`, `NetherNet::lan`): answers discovery on UDP 7551 with the same name and counts,
  and takes `CONNECTREQUEST` / `CANDIDATEADD` in discovery Messages, answering with a trickled
  candidate on the interface that routes to the client. The port is bound shared, because a game
  on the same machine holds it too.
- **Identity.** An offer's `a=identity` must be signed by the key its token names
  (`acacia_nethernet::verify_offer`). Online mode refuses an offer without the auth service's token
  with error 37, as BDS does; offline takes a self-signed one or none. The Login that follows must
  be signed by the offer's key, and it is the Login that verifies the token itself.
- **Host key.** Every answer is signed by `NetherNet::host_key`; clients pin it per address on
  first use, so `sapling::host_key(path)` keeps it in a file.
- Tests: `sapling-net/tests/nethernet.rs` (both signaling paths, beside RakNet) and the NetherNet
  bot in `sapling/tests/integration/join.rs`.

## Server-side movement (Observe by default, Correct opt-in)

`sapling-shard/src/simulation.rs`; research: `acacia/research/sapling-server-movement.md`.

- `ShardConfig::movement_check` (`ServerBuilder::movement_check`): a default `MovementPolicy`, `threshold`
  (0.5, BDS's `player-position-acceptance-threshold`) and `correction_delay` (5,
  `player-rewind-min-correction-delay-ticks`). `Trust`: the sanity checks in movement.rs only. `Observe`
  (the default): stepped and logged, below. `Correct`: past the threshold the server's place stands, sent
  as `CorrectPlayerMovePrediction` stamped with the input's tick, and the next `correction_delay` inputs
  (predicted before the client heard) take the server's place untold; inside the threshold the client's
  place stands but the server keeps its own step, so a stray does not add up unseen. The player's
  `Position`, fall and `MoveEvent` are then the server's.
- A plugin sets one player's policy (`ctx.set_movement_policy(player, Some(policy))`, `None` for the
  shard's), which restarts the step from the client (`simulation/policy.rs`). No physics overrides.
- Correct is opt-in until real clients' Observe counts are clean: acacia's BDS-exact bots walk under it
  with no correction (`tests/integration/map.rs`), while the shard harness's hand-made inputs, which jump
  without physics, are corrected (12 harness tests fail with Correct as the default).
- Each accepted `PlayerAuthInput` of a survival or adventure player is stepped with acacia-physics'
  `tick` over the player's world (`RegistryWorld` of the feature `acacia-world`, over a `Blocks` newtype),
  the held controls read from the input flags as acacia-bot's trace replay reads them, gliding and swimming
  latched from their edges, eating from `Vitals`. The server's feet are compared with the client's; past the
  threshold the input is logged (debug) and counted, and the simulation goes on from the client's place
  but its own velocity and on-ground, so a client claiming a speed is not handed it.
- Adopted, not stepped: the first input on a shard, an input carrying `HandledTeleport`, creative and
  spectator players. Fed to the simulation: knockback (`vitals/struck.rs` `throw`), the speed attribute
  (`vitals/effects.rs`), piston pushes. A leaving player's counts are logged (info): the divergence data
  that decides when correcting is turned on, per platform.

## Operations (as built)

All set through `ServerBuilder` (`sapling/src/server/ops.rs`).

- **`server.properties`** (`server/properties.rs`): `config_file(path)` or `ServerProperties::load` +
  `properties(&props)`. Read: `server-name`, `level-name` (the list's level and StartGame's name; the
  folder is the binary's to open), `gamemode`, `difficulty` (names or numbers), `max-players`,
  `online-mode`, `allow-list`/`white-list` (sets the `access` lists' allowlist; on without lists is
  `StartError::NoAllowlist`), `server-ip`, `server-port`, `view-distance` (≥ 5), `player-idle-timeout`,
  `level-seed` (numbers only, for the binary), `compression-threshold`, `compression-algorithm`,
  `player-position-acceptance-threshold`, `player-rewind-min-correction-delay-ticks`. Every other key
  (`server-portv6`, `tick-distance`, `force-gamemode`, `allow-cheats`, `default-player-permission-level`,
  `transport`, `enable-lan-visibility`, ...) is logged as ignored. Bad values fail with their line.
  Settings made after it win. `examples/generated.rs --properties=` uses it.
- **Compression**: `NetConfig::compression` (zlib = raw deflate, or snappy) and `compression_threshold`
  (default 1) go into acacia's `LoginConfig`, i.e. NetworkSettings.
- **Packet limits** (`sapling-net/src/limits.rs`, `NetConfig::packet_limits`): per session and one-second
  window, 400 game packets of all types and 10 Text, 10 CommandRequest, 40 Animate, 5 Emote, 10 BookEdit;
  the excess is dropped and the second is a strike, five strikes in a row close the session with
  `disconnect.spam`. Loopback sessions are not metered (drills and bot tests run the clock faster).
  `PacketLimits::off()` for load tests.
- **Chat spam** (`chat.rs`): Java's rule, BDS's being unknown: a line adds 20, a tick takes 1, over 200
  the player is kicked with `disconnect.spam` (a line a second goes on for ever, ten at once is the most);
  the 3-lines-a-tick cap stays. Commands do not count and operators are not exempt (Java exempts them).
- **Idle kick** (`idle.rs`, `ShardConfig::idle_timeout`, `idle_timeout(d)` or `player-idle-timeout`):
  the first system of the tick, before packets are taken. Activity is a changed position, yaw or pitch,
  or any of Text, CommandRequest, InventoryTransaction, ItemStackRequest, Interact, Animate,
  PlayerAction, ModalFormResponse, MobEquipment. Counted in ticks of 50 ms, so drills compress it.
  Kicked once with `KickedForIdle` and `disconnectionScreen.body.idleTooLong` (`Outbox::kick_for`).
- **Tick timing** (`sapling-shard/src/timing.rs`): `Shard::tick` publishes its phase (Messages, Systems,
  Handlers, Sending) and start time into `ShardTiming` atomics, and the mean, longest and rate of the
  last 100 ticks when it ends; no lock. `Server::tick_stats()`, or a `Metrics` handle given to
  `ServerBuilder::metrics` before start (for a plugin's `/tps`), with `Metrics::prometheus()` text.
- **Watchdog** (`watchdog.rs`, `ServerBuilder::watchdog`): one thread reads every `ShardTiming` every
  quarter of `slow` (10-250 ms) and logs a tick past `slow` (1 s) once, past `hung` (10 s, BDS's script
  watchdog) once with shard, tick and phase, no stack; past `abort` (off by default) it writes a crash
  report and aborts the process, since a stuck thread cannot be saved or stopped from outside.
- **Crash handling** (`running.rs`, `crash.rs`, `ServerBuilder::crash_reports(dir)`): each tick runs
  under `catch_unwind`. A panic stops that shard only: it closes as on shutdown (handoffs forwarded,
  players kicked with `disconnect.closed`, inventories, actors, scoreboard and columns saved), then writes
  `crash-<unix>-shard<N>-panic.txt` (time, versions, shard, tick, phase, message, location, worlds with
  dimension, loaded columns and players, whether saving worked, the backtrace a panic hook kept for
  shard threads); without a dir the report is logged. The other shards go on; players routed to the
  dead shard are closed by the router.

## Packs (as built)

Opt-in on `ServerBuilder` (`sapling/src/server/packs.rs`); packs are read once, when the server is
built: no runtime loading, as the README's non-goals say. Evidence: `research/captures/bds-packs.txt`,
`bds-functions.txt`; notes and what is unverified: `docs/research/packs-and-functions.md`.

- **Reading** (`sapling/src/packs/`): a pack is a folder with `manifest.json`, or a `.mcpack`/`.zip`
  of one (rooted at the shallowest manifest; stored or deflated entries, read by `archive.rs`, no zip
  crate). `world(world, folder)` takes the packs `world_resource_packs.json` /
  `world_behavior_packs.json` name, from the world's own pack folder or `folder`.
- **Resource packs** (`ResourcePacks` → `sapling_shard::ResourcePacks` in `ShardConfig`, served by
  `sapling-shard/src/packs.rs`): info lists each pack with its unpacked size, content key (given by
  config; an encrypted pack without one is warned of), uuid as content identity and CDN url;
  `required` is `texturepack-required` (must_accept, refusing disconnects with
  `disconnectionScreen.resourcePack`). SendPacks → a DataInfo per pack (102400-byte chunks, SHA-256
  of the zip, which is the pack's own zip or one deflated from the folder); chunk requests are
  queued per player and sent two a tick while the connection is not congested. Stack after
  HaveAllPacks, `game_version` `*`.
- **Functions** (`sapling/src/functions/`, a plugin): `functions/**/*.mcfunction` by path, case kept;
  comments and blank lines dropped, leading spaces trimmed. A function with an invalid line or a line
  starting with `/` is not found (checked on first use against the command registry). `/function`
  runs BDS's stack model: each line counts once, a nested `function` line counts and pushes its lines,
  the run stops at `functionCommandLimit` (10000 if unset); inner output goes nowhere, the sender is
  told `commands.function.success [count]`. `tick.json` functions run every tick as the server in
  every world. `/schedule delay add|clear`, `/schedule clear` keep due ticks per shard; a due function
  runs as the server in the world it was scheduled in and operators get `commands.schedule.functionRan`.
- **`/scriptevent`** (`vanilla/scriptevent.rs`): BDS's answers; the event goes to plugins as
  `ScriptEvent`, queued and offered before tasks each tick (raised from a task or listener, heard the
  next tick).
- Not done: behavior pack data beyond functions, `/schedule on_area_loaded`, the Script API, and
  `server.properties` `texturepack-required` (docs/NEXT.md).

## Building against a local acacia

`Cargo.toml` takes the acacia crates from GitHub at one commit. To build against a checkout beside
this one (to try an acacia change before it is pushed), put a patch for each crate in
`.cargo/config.toml`, which git ignores:

```toml
[patch."https://github.com/acacia-rs/acacia"]
acacia-proto = { path = "../acacia/acacia/crates/acacia-proto" }
acacia-world = { path = "../acacia/acacia/crates/acacia-world" }
# ... and the others `Cargo.toml` names
```

`Cargo.lock` then names the paths: do not commit it while the patch is in place.

## Open items

- **NetherNet, only a vanilla client can confirm:** a direct-connect join and a LAN join by the real
  game (bots only so far); whether it wants the status JSON or BDS's empty body; whether the LAN
  list shows a server that answers from a dedicated process.
- **NetherNet, left out:** the signaling service (friends list, Realms-style joins), TURN and
  server-reflexive candidates, HTTPS signaling, IPv6 LAN discovery, rate limits on signaling (the
  per-address admission counts a NetherNet connection once it opens, not its offers).
- **Access lists, not checked against BDS:** the kick screen without a reason (`disconnect.kicked`)
  and with one (the reason alone); `/kick` refusing the sender; the allowlist list's "seen" count
  (sapling gives the entry count twice); whether `ignoresPlayerLimit` counts with the allowlist off;
  BDS's `commands.allowlist.noOnlineAuth` refusal offline is not copied.

- **Step 6, only a vanilla client can confirm:** which actions a creative client sends for an instant
  break (sapling breaks on PredictBreak and on an item use of type BreakBlock, never on StartBreak
  alone; `RUST_LOG=debug` logs every "input with an action"); that an input carrying StopBreak or
  AbortBreak decodes (acacia-proto reads a position for every block action, which BDS accepted from the
  probe); that the held item's `block_runtime_id` is the block's hash; the resync after a refused break
  and place (no ghost block, no hole), also with the blob cache on; a door opened on screen closing
  again; `ParticleDestroy` and the `place` sound seen and heard by a second client; a block placed while
  another client is still loading the column.
- **Blocks, left out of step 6:** redstone; `USED` and the lists of blocks that fall,
  need support or give way are name endings, not block data; a request over the tick limit gets no
  resync; `on_move`, which needs its own resync on cancel.
- **Landing after a world change.** The teleport goes out before the new world's columns, so a
  client that keeps simulating falls until the floor arrives (acacia's bot falls a tick, 0.0784).
  A player teleported onto a world's spawn is therefore watched for 100 ticks
  (`movement::teleport_onto_ground`): an input below the spawn and within a block of it sideways is
  answered with the teleport again, until one stands there. It stands in for the correction BDS's
  own simulation would send. What a vanilla client does while the columns are missing is not known.
- **Items and several worlds:** a player's items go with it to another shard. They are kept by one
  shard only, the one players join on (`Peers::entry`), in its first world's storage: every other
  shard hands it what it would save (`ShardMsg::KeepPlayer`), and `Server::shutdown` stops it last
  so that it is there to take them. A shard that stops on its own while the entry shard is gone
  loses what its players changed since. Lying items and open containers are per world and do not travel.
- **Step 5 on a vanilla client (1.26.5x, 2026-10-04).** Working: `/world` and the menu button
  both ways (inside a shard and across), no kick, sidebar and boss bar untouched, the refusals
  for an unknown world. Not working: one hub column stayed visible far off in the arena; old
  columns outside the new view are now overwritten with empty ones ("Leftover columns" above),
  which no client has seen yet.
- **Step 5, only a vanilla client can confirm:** that the empty columns take the leftover hub
  column away, and that the client accepts them for positions it is about to leave (they are sent
  before the teleport and the publisher update); columns the view let go of while the player
  walked, before the change, are not emptied: the client is trusted to have dropped those, as
  on any walk; that LevelChunk for a column the client already holds replaces it (also the way an
  arena reset re-sends a column), and standing in old terrain
  for the few ticks until the new columns arrive does no harm; whether the client asks again for
  the sections of a re-announced column when a request for that column was still unanswered (the
  old request is answered from the new world once the column is announced, which covers a client
  that does not); that RemoveEntity for every old entity, sent before the teleport on a cross-shard
  move and after it inside a shard, leaves no ghost; that the sidebar and boss bar survive the
  swap untouched; that a form opened before a cross-shard move can still be answered; that
  `HandledTeleport` comes after a teleport into columns that are not there yet; the hub's `/world`
  both ways with the blob cache on and off.
- **Worlds, left out of step 5:** worlds of another dimension and with them ChangeDimension (BDS's
  order around it is not recorded; dragonfly sends ChangeDimension, StopSound, PlayStatus
  PlayerSpawn and PlayerAction DimensionChangeDone); a loading screen or an intermediate dimension
  to hide the swap; shards created or stopped at runtime through `Server` (a `Link` is fixed at
  start; `Shard::close` exists); an arena's first write copies its column, not one section: that
  needs `Arc` sections inside `acacia_world::Chunk`; `create_world` makes the arena on the
  handler's own shard only, and nothing balances arenas over shards; two shards that create a
  world of the same name in the same tick both keep theirs; with three or more shards a player in
  flight between two can miss a list entry a third announces meanwhile; a player on another
  shard cannot be addressed (`send_message`, `position`, `Target` in commands are the shard's);
  per-world game mode, difficulty and StartGame name; `on_quit` is not raised for a player whose
  shard stopped; a template is built from a world held in memory whole.
- A blob-cache client told a section hash before a block changed and asking for the blob after gets the
  old section, though it already applied the UpdateBlock: re-send the block updates of a tick with the
  blobs, or name blobs by content and time.
- **Only a vanilla client can confirm** (bots decode all of these): the plain fallback skin
  (`geometry.humanoid.custom`, 64x64); `trusted` = "false" for skins that claim to be untrusted; one
  PlayerList packet per record instead of one packet with many; the correction teleport's
  `cause: Unknown`; smoothness of MovePlayer sent on change instead of every tick as BDS does;
  BoundingboxHeight sent with every pose change (BDS: only when sneaking changes).
- **Step 4 on a vanilla client (1.26.5x, 2026-10-04).** Working: title, sidebar, the boss bar showing,
  chat, the `AvailableCommands` list and completion, `/hub` teleport, creative flight via `/gamemode`
  (no UpdateAbilities needed), the menu form and the custom form (answers arrive as
  `[null,true,1,100.0,""]`: a label is answered with null). Not working: a second ShowBar with another
  colour or progress changed nothing on screen; changes are now sent as SetBarTitle / SetBarProgress /
  Texture ("Boss bar" above), which no client has seen yet.
- **Step 4, only a vanilla client can confirm:** boss bar changes by SetBarTitle / SetBarProgress /
  Texture, and if they do nothing, HideBar then ShowBar as dragonfly sends every change; the hub's first
  bar (green, half full: ShowBar's colour and progress); HideBar (`/clear` in the hub); plain text as
  `CommandOutput` message ids (a `%` in it may be read as a format); the bytes of a closed form; a
  sidebar with two lines of the same text; removing and re-adding a line in one batch without flicker;
  RemoveObjective alone to hide the sidebar (BDS also clears the slot); ToastRequest;
  UpdatePlayerGameType for another player's id.
- Commands: console sender, selectors in `Target` parameters, position and item parameters, soft enums
  (UpdateSoftEnum) for dynamic completions, several overloads for a struct, permission nodes.
- Forms: images are untested, `ServerSettings`, ClientboundCloseForm, a timeout for unanswered forms.
- HUD: several boss bars (needs an entity each).
- Not sent yet, though BDS sends them for other players: UpdateAbilities, UpdateAdventureSettings,
  UpdateAttributes (sprint speed), PlayerLocation / LocatorBar, the MOVING flag and the FlagsExtended bit
  that changes with sneaking, held items and armour.
- Movement: corrections from the server's step (`CorrectPlayerMovePrediction`, BDS's 5-tick delay and
  40-tick rewind), on-ground and fall height from it, `SetMovementAuthority` (check whether BDS sends it),
  knockback stamped with the input tick, a per-player policy for plugins.
- Broadcasts are encoded once but compressed per player; `SharedBatch` needs a per-layer batch in
  sapling-net. Block changes and chat still look at every player of the shard per event.
- A newcomer gets every skin at once (about 290 KB each before compression); throttle before the
  per-player backlog limit (16 MB) matters. Skin changes in game (PlayerSkin) are ignored.
- Chat: private messages, profanity filter fields.
- **Operations, not checked against BDS:** the idle kick's activity rule and screen, BDS's chat or
  packet rate limits (Java's chat rule is used), `level-seed` text hashing, `tick-distance`, an IPv6
  listener beside the IPv4 one (`server-portv6`), config hot reload.
- **BDS only saves chunks that changed since generation.** A folder's holes are void unless it is
  opened with `generate_missing`, which fills them as a new world would be generated ("Generation").
- Block-state upgrader (CC0 BedrockBlockUpgradeSchema) for palettes older than the current version;
  until then see "Block states" above. Legacy sub-chunk versions (< 8, except 1) are skipped as air.
- Block entities: no vanilla client has seen them (bots decode them), and what BDS itself puts into a
  chunk for them was not recorded: sapling sends the stored compound without `Items`. Only chests and
  barrels are opened, edited or created; the container shapes (`InventoryContent` naming no container,
  the lid's `BlockEvent`) are what acacia-bot accepts, not what BDS was recorded sending.
- Inventories: no vanilla client has used them (the shard tests play the client). Unverified against a
  client: the entity id in the inventory screen's `ContainerOpen` (the player's id is sent; blocks use
  -1), an offhand `MobEquipment` for a player (the stored-mob shape is used), `item_stack_id` 0 for an
  emptied slot. Not built: wear of anything but tools, armour only in armour slots,
  `PlayerAuthInput`'s own stack request (MineBlock), a pickup event. Stack sizes for `give` are a list
  of name endings (`sapling_data::max_stack`). Items given in `on_join` before a slow storage answers are
  replaced by the stored ones. Lying items land once and stay, are not saved, and stored item entities
  cannot be picked up. Only the first world's storage keeps players.
- Saving: a vanilla client has not opened a sapling-written world (BDS has), and the BDS check does not
  cover biomes or block entities, which sapling now also writes (chests, barrels).
- Mobs: a vanilla client has not seen them or the dropped items (bots decode both). A mob looks like a
  freshly summoned one of its kind: what it stores beyond the fields above (a horse's armour slot,
  effects) is not shown; villagers and wandering traders are shown and kept as traders
  (docs/research/villagers.md). A stored block
  item saved by an older game version carries old states, whose hash a client may not know. Movement,
  AI, damage; a player's `RideID`.
- Clock and level: a vanilla client has not confirmed `SyncWorldClocks` or the other join packets as
  sapling sends them; game rules other than the daylight cycle are not read from `level.dat`. A
  difficulty set while the server runs (`ShardCtx::set_difficulty`) is not saved.
- Only the overworld is opened; nether/end keys parse but no world is made for them.
- LevelDB block compression id 2 is zlib in worlds; never enable zstd (Mojang master reuses id 2).
- Game data source and licence (item/creative/biome/recipes) for sapling-data.
- acacia-proto's shield item id is process-global: never run acacia-bot and sapling in one process.
