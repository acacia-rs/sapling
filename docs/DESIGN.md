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

- **Handlers.** `Server::builder().handler_at(Priority, h)`; `Highest` runs first, `Lowest` last, `Monitor`
  after all. `on_chat` and `on_command` get a mutable event (`ChatEvent::message`, `cancel()`); a cancel stops
  the handlers that have not run, except monitors, which get a copy of the final event and cannot change it.
  `on_tick` runs once a tick after the events. `on_move` was left out: it needs a resync on cancel.
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
  (id = line + 1, score = line). `ctx.set_sidebar` diffs against what the player was shown: changed lines are
  removed and set again (the client keeps the text of an entry it knows), a new title rebuilds the board,
  15 lines at most.
- **Boss bar.** One per player, on the player's own entity id (dragonfly's way: no extra entity).
  Every change sends ShowBar again, as BDS does for its wither; `remove_boss_bar` sends HideBar.
- **Titles.** SetDurations, SetSubtitle, SetTitle in that order, unset times -1, as `/title` sends them;
  `send_actionbar`; `send_toast` (ToastRequest).
- **Teleport.** `ctx.teleport` sends MovePlayer mode Teleport, cause Command, as `/tp`; the player's inputs
  are ignored until one carries `HandledTeleport`, and the teleport is repeated after 20 ticks without one.
- **Game mode.** `ctx.set_game_mode` sends UpdatePlayerGameType to everyone in the world, the player
  included: all BDS's `/gamemode` sends besides chat lines (no SetPlayerGameType, no UpdateAbilities).

## Open items

- **Only a vanilla client can confirm** (bots decode all of these): the plain fallback skin
  (`geometry.humanoid.custom`, 64x64); `trusted` = "false" for skins that claim to be untrusted; one
  PlayerList packet per record instead of one packet with many; the correction teleport's
  `cause: Unknown`; smoothness of MovePlayer sent on change instead of every tick as BDS does;
  BoundingboxHeight sent with every pose change (BDS: only when sneaking changes).
- **Step 4, only a vanilla client can confirm:** `AvailableCommands` as a whole (a wrong one crashes or
  kicks; the bot only decodes it) and whether completion shows with every command at permission "any" while
  StartGame is the recorded one; plain text as `CommandOutput` message ids (a `%` in it may be read as a
  format); the form JSON (legacy `buttons`, no `tooltip`/`submit` keys) and the bytes of a closed form; a
  sidebar with two lines of the same text; removing and re-adding a line in one batch without flicker;
  RemoveObjective alone to hide the sidebar (BDS also clears the slot); the boss bar on the player's own
  entity, its colours, and HideBar's layout (BDS never sent one in 20 s: its bar goes with the entity, and
  acacia-proto gives every BossEvent the ShowBar layout); ToastRequest; UpdatePlayerGameType for another
  player's id; whether creative flight works without an UpdateAbilities.
- Commands: console sender, `@a`/`@p`/`@r` selectors, position and item parameters, soft enums
  (UpdateSoftEnum) for dynamic completions, several overloads for a struct, permission nodes.
- Forms: images are untested, `ServerSettings`, ClientboundCloseForm, a timeout for unanswered forms.
- HUD: list and below-name objectives, several boss bars (needs an entity each), `/title` clear and reset.
- `on_move` and the other cancellable world events (break, place, interact) come with step 6.
- acacia-proto: `AvailableCommands` value type names are stale for 1.26.5x (string 56, block position 64,
  position 65, message 68, raw text 70, json 74, block states 84, command 90).
- Not sent yet, though BDS sends them for other players: UpdateAbilities, UpdateAdventureSettings,
  UpdateAttributes (sprint speed), PlayerLocation / LocatorBar, the MOVING flag and the FlagsExtended bit
  that changes with sneaking, held items and armour.
- Movement: physics validation (acacia-physics), on-ground from collision.
- Broadcasts are encoded once but compressed per player; `SharedBatch` needs a per-layer batch in
  sapling-net. Visibility and fan-out are O(players²) per layer: index players by chunk.
- A newcomer gets every skin at once (about 290 KB each before compression); throttle before the
  per-player backlog limit (16 MB) matters. Skin changes in game (PlayerSkin) are ignored.
- Chat: proper rate limiting, private messages, profanity filter fields.
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
