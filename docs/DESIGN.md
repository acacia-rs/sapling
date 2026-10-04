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
  void). A storage-backed `World` owns two I/O threads: `request(pos)` queues a load, the thread reads,
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
- **Block entities.** A column keeps its record as stored and saves it back. `set_block` drops the
  compounds at a position when the block there becomes another kind (a different name).
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
- **Mobs** (`mobs.rs`). The stored actors of a loaded column become shard entities, shown with AddEntity
  to players whose view covers the column and removed with it. They are statues: nothing moves them and
  no interaction reaches them, and the column saves them back unchanged. Sent: kind, position, rotation,
  `Variant`, `MarkVariant`, `Color`, the baby flag, the name tag. Not sent, though BDS does: attributes,
  some forty more metadata items, and the kind's hitbox, which is in no file (every mob gets 0.9 x 0.9).
  Item entities are skipped.
- **Sending.** A `View` keeps a nearest-first queue; each tick a player gets at most
  `ShardConfig::columns_per_tick` (8) columns that are loaded, and only the nearest `2 x` that many are
  asked of storage. `NetworkChunkPublisherUpdate` goes out when the player's chunk changes.
  PlayStatus(PlayerSpawn) follows once the columns within 3 chunks are sent (BDS: 26 columns).
  Sub-chunk requests for a column not loaded wait (`SectionBacklog`, 64 requests a player) and are
  answered once it is: a vanilla client disconnects itself on a `ChunkNotFound` entry.
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
  A new bar is ShowBar, `remove_boss_bar` HideBar; a bar that is shown changes by SetBarTitle,
  SetBarProgress and Texture (colour), one for each part that differs. A vanilla client ignores ShowBar
  for a bar it already shows, though BDS repeats ShowBar once a second for its wither (there the client
  reads the entity). BossEvent is flat in 1.26.5x: every type has id, type, title, filtered title,
  progress, colour, overlay, so each event carries the whole bar and HideBar carries an empty one. BDS's
  ShowBar bytes prove that layout (acacia `docs/proto.md`); BDS sends no other type, not even when the
  player leaves the wither's range, so the others rest on gophertunnel.
- **Titles.** SetDurations, SetSubtitle, SetTitle in that order, unset times -1, as `/title` sends them;
  `send_actionbar`; `send_toast` (ToastRequest).
- **Teleport.** `ctx.teleport` sends MovePlayer mode Teleport, cause Command, as `/tp`; the player's inputs
  are ignored until one carries `HandledTeleport`, and the teleport is repeated after 20 ticks without one.
- **Game mode.** `ctx.set_game_mode` sends UpdatePlayerGameType to everyone in the world, the player
  included: all BDS's `/gamemode` sends besides chat lines (no SetPlayerGameType, no UpdateAbilities).

## Step 5 as built (Worlds)

No BDS capture exists for this step: BDS has one world per dimension. The client side of a world
change follows dragonfly @ 1793369 (`session.handleWorldSwitch`, `Loader.ChangeWorld`), whose
same-dimension case sends no ChangeDimension: it hides the old world's entities, teleports and
sends every column of the view again. Every sapling world is the overworld, so ChangeDimension is
never sent yet, and BDS's order around it (a portal, `/execute in nether`) was not recorded.

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
  `handler(h)` is for servers of one shard (`StartError::HandlerPerShard` otherwise). Commands
  and templates go to every shard.
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
  A cancelled start is not a handler's cancel: every handler still sees the event (`Cancellable::is_stopped`)
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
- **Trust.** There are no inventories: the block placed is the one the client's item names, which is why
  only creative places by default and a handler that allows a survival place vouches for the item. A
  break counts when the client says it is finished: no mining-time check in any mode. Positions and the
  eye are the server's own; the packet's `player_pos`, `click_pos` and clicked block id are ignored.
- **Limits.** 16 block actions and 4 clicks a player a tick; an input's action list is cut to 16. What
  exceeds them is dropped without an answer. Absurd positions (the edges of `i32`, outside the height)
  are refused before anything is computed from them.
- **API.** `ctx.block(pos) -> Option<Block>` (`None` where not loaded or outside the height) and
  `ctx.set_block(pos, &block) -> bool`, which shows the block to its viewers and raises no event. The hub's
  whole protection is `if !may_build(ctx, player) { event.cancel() }` in `on_break` and `on_place`, with
  `may_build` = operator and in creative mode.

## Open items

- **Step 6, only a vanilla client can confirm:** which actions a creative client sends for an instant
  break (sapling breaks on PredictBreak and on an item use of type BreakBlock, never on StartBreak
  alone; `RUST_LOG=debug` logs every "input with an action"); that an input carrying StopBreak or
  AbortBreak decodes (acacia-proto reads a position for every block action, which BDS accepted from the
  probe); that a block picked from the creative inventory stays in the hand though no ItemStackRequest is
  answered; that the held item's `block_runtime_id` is the block's hash; the resync after a refused break
  and place (no ghost block, no hole), also with the blob cache on; a door opened on screen closing
  again; `ParticleDestroy` and the `place` sound seen and heard by a second client; a block placed while
  another client is still loading the column.
- **Blocks, left out of step 6:** inventories and ItemStackRequest; item drops; block entities' contents
  (a broken chest's record is dropped, nothing is created); mining time and tool speed, so also the crack
  animation others see (`BlockStartBreak` carries the break time); block updates and physics (sand, water,
  torches losing their support, fences and stairs reconnecting); orientation: the item's own state is
  placed, so stairs and logs ignore where the player looks; replaceable plants (tall grass) are built
  against, not replaced; `USED` is a list of name endings, not block data; what a click does (doors,
  levers, containers) and its sounds; a request over the tick limit gets no resync; `on_move`,
  which needs its own resync on cancel.
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
- Commands: console sender, `@a`/`@p`/`@r` selectors, position and item parameters, soft enums
  (UpdateSoftEnum) for dynamic completions, several overloads for a struct, permission nodes.
- Forms: images are untested, `ServerSettings`, ClientboundCloseForm, a timeout for unanswered forms.
- HUD: list and below-name objectives, several boss bars (needs an entity each), `/title` clear and reset.
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
- Saving: a vanilla client has not opened a sapling-written world (BDS has), and the BDS check does not
  cover biomes, block entities or actors; block entities are only ever removed, never created or edited.
- Mobs: a vanilla client has not seen them (bots decode AddEntity); per-kind hitboxes and attributes
  would have to be recorded from BDS kind by kind; item entities; movement, AI, damage, riding. The mob
  diff is O(players x mobs) a tick.
- Clock and level: a vanilla client has not confirmed `SyncWorldClocks` as sapling sends it; SetDifficulty,
  GameRulesChanged and SetSpawnPosition, which BDS sends at the join, are not sent; `set_spawn` tells no
  client; game rules other than the daylight cycle are not read from `level.dat`.
- Only the overworld is opened; nether/end keys parse but no world is made for them.
- LevelDB block compression id 2 is zlib in worlds; never enable zstd (Mojang master reuses id 2).
- Game data source and licence (item/creative/biome/recipes) for sapling-data.
- acacia-proto's shield item id is process-global: never run acacia-bot and sapling in one process.
