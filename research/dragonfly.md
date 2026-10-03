# Dragonfly deep-dive (reference for sapling)

Source: `github.com/df-mc/dragonfly` @ `1793369` (2026-09-20), shallow clone. Paths below are relative to
that repo. Go 1.26, gophertunnel v1.62, `df-mc/goleveldb` fork.

**Licence: MIT** (`LICENSE`, "Copyright (c) 2019 Dragonfly Tech"). Porting is allowed; keep the notice
(same rule as acacia's `LICENSE-bedsim`).

## 1. Server, listeners, sessions

- `server/server.go`: `Server` holds 3 fixed worlds (overworld/nether/end), `[]Listener`, a `p map[uuid]*onlinePlayer`
  under `pmu`, and an `incoming chan`. `Listen()` starts one goroutine per listener (`listen`), each spawning a
  goroutine per conn (`finaliseConn`): load player data from `PlayerProvider` → pick world → `StartGameContext` →
  reject duplicate UUID → send `ItemRegistry` → `createPlayer` → push to `incoming`.
- `Accept()` is an iterator the *user* drives: for each incoming it does `world.Call(w, tx.AddEntity(handle); s.Spawn)`
  and yields the `*player.Player` inside the world transaction. So the plugin's join hook runs on the world goroutine.
- `Listener` interface (`server/listener.go`): `Accept() (session.Conn, error)`, `Disconnect`, `Close`. RakNet and
  NetherNet listeners both implement it; `session.Conn` is an interface (`server/session/session.go:121`) so any
  transport works.
- Per session (`session/session.go`): **3 goroutines**: writer (drains `packets chan` of 256), `handlePackets`
  (reader), `background` (20 Hz: command/enum resend every 1 s/5 s, `sendChunks`).
  - Reader: every packet → `withControllable` → `world.CallRef` → **blocking round-trip onto the world goroutine**
    per packet; handler is looked up in `handlers map[uint32]packetHandler` (`registerHandlers`, one file per
    packet `handler_*.go`). Unknown packets are logged and dropped; `nil` entries are explicit ignores.
  - `writePacket` blocks when the 256-slot channel is full → a slow client can stall the world goroutine.
  - Disconnect: `c.Close()` (may respawn → world change) then a second Call for `Session.Close` (save via
    `HandleStop`, close loader, `RemoveEntity`, close handle).
- Session ↔ world: the session holds only an `*world.EntityHandle`; it never caches the world. World switch is
  detected lazily in `sendChunks` (`chunkLoader.World() != tx.World()` → `handleWorldSwitch`: clear blob cache,
  `ChangeDimension` if dimension differs, teleport, `Loader.ChangeWorld`).
- Chunk loading (`world/loader.go`): `Loader{r, pos, loadQueue, loaded, pending}`; `Move` evicts out-of-radius
  chunks and rebuilds a centre-outward queue; `Load(tx, n)` pulls ≤4 per tick (capped by ≤8 open blob
  transactions). Loads go through `world.loadChunkAsync` → `chunkWorkerPool` (`world/chunk_request.go`,
  default **1** worker, queue 4096) → result re-enters the owner via `w.Do(r.signal)`.
- Chunk send (`session/chunk.go`): sub-chunk request mode is on (`subChunkRequests = true`): `LevelChunk` carries
  only biomes + `SubChunkLimit`; client then sends `SubChunkRequest` and `ViewSubChunks` encodes each sub-chunk on
  demand. Blob cache: xxhash64 of each serialised sub-chunk, blobs kept per session (`s.blobs`, cap 4096).
  **No cross-session encoding cache**: every viewer re-encodes the same sub-chunk (`chunk.EncodeSubChunk`).
- Entity visibility = **chunk-based, not distance-based**: each `Column` has `viewers []Viewer` + `loaders`
  (`world/world.go:1532`). `addViewer` shows all entities in the chunk; `tickEntities` (`world/tick.go:212`) diffs
  old/new chunk viewers when an entity crosses a chunk border. `Viewer` (`world/viewer.go`) is a ~25-method
  interface the session implements (`session/world.go`). Runtime IDs are **per session**
  (`entityRuntimeIDs map[*EntityHandle]uint64`; self is always 1). `ViewLayer` (`world/view_layer.go`) gives
  per-viewer overrides (name tag, visibility).

## 2. World model, transactions, plugin access

- One goroutine per world runs `handleTransactions` over `queue chan transaction` (cap 128) (`world/world.go:196`);
  a separate `tickLoop` goroutine enqueues `tick` 20×/s and waits for it; `autoSave` enqueues save/unload.
  The world's state (`chunks`, `entities`, scheduled ticks) is accessed only from inside a transaction.
- `world.Tx` (`world/tx.go`): handle valid only during the callback (`closed` → panics after). All world ops
  hang off it (`SetBlock`, `Block`, `AddEntity`, `RemoveEntity`, `EntitiesWithin`, `Viewers`, ...).
  `tx.Defer(f)` queues FIFO follow-ups run after the callback.
- Current API (2026; `world/task.go`) replaced the old `ExecWorld`/`<-chan` style:
  - `w.Do(f) *Task` fire-and-forget; `w.DoAfter(d, f)`; `world.Call[T](ctx, w, f)` blocking typed result.
  - Entity-targeted: `EntityRef[T].Do`, `CallRef`, `CallEntity`, `player.Do(h, f)` (`world/entity_ref.go`,
    `player/ref.go`): resolve the entity's *current* world at run time.
  - `Task` has `Done/Err/Wait/Cancel/OnDone`; panics in tasks are recovered into `PanicError`.
  - Full queue → hands off to a helper goroutine instead of blocking (`scheduleTask`).
  - Documented deadlock rule: calling `Call`/`Wait` for the same owner from inside a callback deadlocks
    (`server.go:117-131`, `task.go:167`).
- Entities: `EntityHandle` (`world/entity.go`) is the persistent identity (UUID, type, `EntityData`); an
  `Entity` value is **opened per transaction** (`EntityType.Open(tx, handle, data)`, e.g. `player/type.go` allocates
  a fresh `*Player` wrapper each time) and is invalid after the tx.
- Cross-world transfer (`player/player.go:984` `respawn`): `h := tx.RemoveEntity(p)` in source owner →
  `dst.Do(tx.AddEntity(h))`; fallbacks re-add to source if dest closed, else close the session. Between the two,
  the handle is "worldless"; scheduled entity tasks block on `EntityHandle.cond` until it lands.
  The machinery for that (`execWorld`/`weakExec`, `world/entity.go:201-321`) is a sync.Cond + "weak transaction"
  dance to avoid deadlocking when a queued tx moves the entity — the most intricate code in the repo.
- Event handlers: one `Handler` interface per subject — `player.Handler` (~50 methods, `player/handler.go`),
  `world.Handler` (`world/handler.go`: liquid, fire, explosion, redstone, entity spawn/despawn, close),
  `inventory.Handler`. Embed `NopHandler` for defaults. **One handler per player/world** (`p.Handle(h)`
  replaces). Cancellation: `ctx.Cancel()` on `*player.Context` = `*world.Context{*Tx, cancel}` + player
  (`player/context.go`); mutable params are pointers (`*message`, `*damage`, `*drops`). Generic
  `event.Context[T]` (`event/context.go`) is used by the leaf packages.
- `Config.Synchronous` (`world/conf.go:79`): no goroutines, `Do` runs inline, `AdvanceTick()` drives ticks —
  used for deterministic unit tests (57 test sites).

## 3. Entities, players, inventory, registries, commands, forms, scoreboards

- Entities (`server/entity/`): one generic `entity.Ent` + pluggable `Behaviour` (projectile, item, falling block,
  TNT, ...) rather than one struct per mob. No mob AI at all (only passive/projectile physics).
  `EntityRegistryConfig` (`world/entity.go:493`) is a table of constructor funcs so `block`/`item` can spawn
  entities without import cycles.
- Player: `player/player.go` is 3552 lines; state lives in `playerData` inside `EntityData.Data`. Movement is
  client-authoritative: `PlayerAuthInputHandler` (`session/handler_player_auth_input.go`) just applies the
  delta → `Player.Move` → `HandleMove` → `ViewEntityMovement` to every chunk viewer **immediately per packet**
  (no per-tick batching). Only teleport-desync and NaN guards; no movement validation.
- Inventory (`item/inventory/inventory.go`): `RWMutex`-guarded slots + slot-change callback (session uses it to
  send updates) + `Handler` (take/place/drop). Server-authoritative inventory (`ServerAuthoritativeInventory:
  true`). Item stack requests: `session/handler_item_stack_request.go` — per request, iterate actions
  (take/place/swap/destroy/drop/craft/auto-craft/creative/beacon/workstations), verify slot ids against
  tracked `changes`, then `resolve` or `reject` (reject resends authoritative slots). Workstation crafts in
  `handler_{smithing,stonecutter,enchanting,loom,grindstone,anvil}.go`.
- Blocks/items are Go **value types** implementing `world.Block` (`EncodeBlock() (name, props)`,
  `Hash() (base, state)`, `Model()`). `BasicBlockRegistry` (`world/block_registry.go`): `BlockRuntimeID` =
  combined hash → map lookup (`/cmd/blockhash` generates `Hash()`); slow path hashes name+props.
  Per-rid flag table (`blockInfo`: random-tick, light, filter, NBT, liquid) for hot loops. State list loaded from
  embedded `block_states.nbt`; registry must be `Finalize()`d; custom/data-driven blocks supported. Items via
  `world.RegisterItem` + `vanilla_items.nbt`.
- Commands (`server/cmd`): a command is a struct implementing `Runnable.Run(src, *Output, tx)`; exported fields are
  parameters parsed by reflection in field order (`cmd/argument.go`), `cmd:"name,suffix"` tags,
  `Optional[T]`, `Varargs`, `[]Target`, `SubCommand`, custom `Parameter`/`Enum`. Multiple Runnables = overloads.
  `Allower.Allow(src)` gates visibility. Registry is a **global map** (`cmd/register.go`). The session polls for
  permission changes and resends `AvailableCommands` every 5 s / enums every 1 s (`session.go:450-466`).
- Forms (`player/form`): `form.New(structValue)` — exported fields are elements, built by reflection; `Submit`
  gets the filled struct. `Menu` (buttons) and `Modal` variants. Pending forms in a per-session
  `map[formID]Form` (`session/handler_modal_form_response.go`).
- Scoreboard (`player/scoreboard`): sidebar only, `io.Writer` of ≤15 lines; resend whole board via
  `Player.SendScoreboard`; session diffs `currentLines`. No objectives/teams/below-name.
- Also present: bossbar, title, hud elements, dialogue (NPC), debug shapes.

## 4. Storage, generation, saving, network chunk encoding

- `world.Provider` (`world/provider.go`): `Settings/SaveSettings`, `LoadColumn/StoreColumn(pos, dim)`, player
  spawn. Wrapped in `lockedProvider` (one mutex) because owner and chunk workers both call it (`world/conf.go:141`).
- `world/mcdb`: vanilla LevelDB format. Keys `x,z[,dim] + tag` (`mcdb/keys.go`: `,` version, `/`+y sub-chunk,
  `+` 3D biomes (first 512 B heightmap discarded), `1` block entities, `digp`/`actorprefix` entities, `3`
  scheduled ticks). `column()` does ~one `Get` per sub-chunk (≈24 for overworld). `storeColumn` writes a
  `leveldb.Batch`. Chunk version 42; older versions load with a debug log. `level.dat` in `mcdb/leveldat`.
  Needs the `df-mc/goleveldb` fork for Mojang's raw-zlib compression.
- `world/chunk`: paletted sub-chunks (`palette.go`, `paletted_storage.go`), `DiskEncoding`/`NetworkEncoding`
  (`encoding.go`), light (`light.go`, `light_area.go` — BFS over a 3×3 chunk area once all neighbours load),
  heightmap. `Compact()` before save shrinks palettes. Legacy-state upgrade in `world/state_upgrade.go`.
- Generators: only `generator/flat.go` (fills layers + biome); `NopGenerator` default. Vanilla terrain gen is
  out of scope.
- Saving: `autoSave` every `SaveInterval` (10 min) and unload every `ChunkUnloadInterval` (2 min) for chunks with
  no viewers — **both run as one owner transaction over all chunks**, encoding + LevelDB writes on the tick
  thread. `Tx.chunk()` (`world/world.go:1368`) also loads/generates *synchronously* on the owner when a tx
  touches an unloaded chunk.
- Ticking (`world/tick.go`): world stops ticking with zero viewers; random ticks only within `tickRange` of a
  loader; scheduled ticks are a flat slice scanned every tick; entity tick only if its chunk has viewers.

## 5. Concurrency pitfalls and performance notes (code)

Documented rules (wiki `dev/World-Transactions.md`, https://github.com/df-mc/wiki): one owner goroutine per
world; `Tx`/`Entity`/`*Player` valid only inside the callback (store `EntityHandle`/`player.Ref`); `Call`/`Wait`
from owner code deadlocks; `tx.Defer` for FIFO same-owner follow-ups, `handle.Do` to follow an entity across worlds.

From code:
- Every inbound packet is a blocking cross-goroutine `CallRef` (`session.go:414`); one packet in flight per
  player; the world goroutine is the bottleneck for all players in it.
- Outbound back-pressure: `writePacket` blocks the world goroutine once a client's 256-slot queue is full.
- Owner-thread I/O: `Tx.chunk()` sync load/generate, full-world save/unload transactions, `lockedProvider` mutex
  shared with chunk workers.
- O(viewers × movers) per-packet movement fan-out, each packet built and encoded per session; per-tx
  `EntityType.Open` allocates wrapper objects; chunk sub-chunks re-encoded per viewer.
- Scheduled ticks: flat slice scanned every tick (`tick.go:309`); random ticks walk every loaded chunk × every loader.

From issues/PRs:
- Self-deadlocks were the main bug class until the API was redesigned: #966 (`ExecWorld` inside `Exec` while
  entity removed — `e.cond.L` re-lock), #981 (`<-World().Exec` from inside a tx), #990 (player removed and not
  re-added hangs server), #1448 + open PRs #1419/#1381 (`Loader.viewChunk` holds non-reentrant `l.mu` while viewer
  code triggers an inline chunk load that re-enters it — owner frozen; still open in v0.11.2).
- PR #1305 (v0.11, 2026-07) removed `World.Exec`/`ExecWorld` because "which goroutine runs this" was unclear;
  replaced by `Do`/`Call`/`EntityRef`/typed errors/`PanicError`/per-event `world.Context`.
- PR #1302: `Synchronous` worlds were added because block/world code had no tests (20 TPS async = nondeterministic).
- PR #1092/#1109: batching ≤256 packets per tx deadlocked and was reverted (#1097, #1104 rejected fixes).
- #967 concurrent map writes on world viewers (fixed by requiring a tx); PR #1232 bounded channel send under
  RLock; PR #1417 (open) inventory slot callback read after unlock; #1364 (open) join TOCTOU on duplicate UUID.
- Cross-world transfer historically fragile: #701 (unlock of unlocked mutex in blob tracking), #639 (stuck in
  nether), #276 (open) chunk-membership update once per tick causes races.
- #995: an apparent deadlock was huge velocity → near-infinite collision checks; clamp velocities.
- Perf: PR #1094 (open, unmerged) network chunk cache — confirms per-viewer re-encode; PR #1449 (open) per-column
  heightmap cache, 18×18 skirt 39.2 → 10.4 ms; PR #1191 palette compact −88 % allocs; PR #1192 light BFS ring
  buffer ~4400× fewer allocs; #141 never hold the chunk map during generation/lighting; #658 long teleports can
  OOM (bound load radius); PR #1412 (open) non-`Ent` entities leak a handle per chunk reload; PR #1400 (open)
  dirty flag misses container contents → item dupes on unload.
- No issues found on many-player benchmarks, GC tuning or LevelDB save latency.

## 6. Weaknesses and criticisms

- **No plugin system by design** (#400, #314, wiki FAQ): dragonfly is a library; users write `main`
  (https://github.com/df-mc/template).
- **One handler per player/world/inventory, no priorities** (wiki `dev/Event-Handlers.md` "Multiple simultaneous
  handlers": unsupported; points to third-party Peex). Composing plugins is the user's problem.
- **API churn**: two breaking concurrency redesigns in ~18 months — v0.10 (PR #942, +5579/−5430, everything
  through `Tx`; cmd target selectors became same-world only) and v0.11 (PR #1305; `Exec` removed,
  `ctx.Val()` → `ctx.Player()`, commands may get a nil Tx; wiki `dev/v0.11.0-Migration-Guide.md`).
- **Cross-world/entity sync complexity**: `EntityHandle` cond-var + weak transactions (`world/entity.go:201-321`)
  is intricate and the source of the deadlock family above.
- **Vanilla gaps** (#566 open tracker): no mob AI/pathfinding, riding (PR #1119 still open), boats, fishing,
  cauldrons; redstone missing repeaters/comparators/observers/pistons (#1228); only flat/void generation and no
  `PopulateChunk` with neighbour context (#1263).
- **Storage abstraction leaks LevelDB**: providers must return `leveldb.ErrNotFound` to trigger generation;
  `Provider.Close` called once per dimension (#1098, #1295 open). Container state behind block pointers breaks
  dirty tracking (#984 rework proposal, #1400).
- **UI**: one bossbar per player (#893); sidebar-only scoreboard with past flicker/line bugs (#367, #427, #469);
  no objectives/teams.
- **Cancellation doesn't always resync the client** (#749: cancelled place). Network and disk IDs shared (#221).
- **Client-authoritative movement**, no validation (`handler_player_auth_input.go`).
- **Fixed 3 worlds per server** (`server.go:48`); extra worlds and transfers between them are manual.
- Compression not configurable (#698, refused). No proxy/transfer support in core.

## 7. Recommendations for sapling (shard + bevy_ecs)

| Area | Port from dragonfly | Do differently in sapling |
|---|---|---|
| Ownership model | One owner thread per world = shard; `Tx`-scoped access; `Defer` FIFO; "never store a tx-scoped ref" rule | Make it a compile-time guarantee: handlers get `&mut ShardCtx` (lifetime-bound), not a runtime-checked `closed` flag. Cross-shard access only by message, never by blocking `Call` from a shard (no API that can self-deadlock) |
| Off-shard API | `Do`/`DoAfter`/`Call` + `Task` with typed errors (`WorldClosed`, `EntityClosed`) and panic capture | Shard mailbox = MPSC of boxed `FnOnce(&mut ShardCtx)`; `call` returns a future, only usable off-shard; catch_unwind per task |
| Entity identity | `EntityHandle` (UUID) separate from tx-scoped view | Global `PlayerId` → shard directory (`DashMap`/arc-swap); bevy `Entity` stays shard-local. Messages to a player route via directory and are **forwarded** if the player moved — no cond-var/weak-tx machinery |
| Transfer | Remove-then-add with fallback to source and "never leave a player worldless" (#990) | Two-phase handoff: source serialises player components → dest acks → source drops. Session task's sink pointer swaps atomically. Test both-shards-closing paths |
| Packet ingress | Per-packet handler table (`registerHandlers`) | Session task decodes off-shard and pushes into the shard's inbox; shard drains all inbox packets at tick start (no per-packet round-trip, no one-in-flight limit) |
| Packet egress | Per-session writer task | Bounded queue that **never blocks the shard**: drop/kick on overflow. Encode broadcast packets once (`Bytes`), fan out shared buffers |
| Movement | — | Batch per tick: collect moved entities, one `MoveActorDelta` set per viewer per tick; validate positions (acacia-physics exists) |
| Visibility | Chunk-column viewer lists; per-session runtime-ID maps; `ViewLayer` overrides | Keep chunk-based interest management but as an ECS system run once per tick (diff sets); runtime IDs can be global-per-shard since a client only sees one shard |
| Chunks to network | Sub-chunk request mode, blob cache (xxhash), heightmap prep | Shared per-shard encoded sub-chunk cache keyed by (pos, version), invalidated on block change (dragonfly PR #1094 never merged); hub templates can pre-encode at load. Cache heightmaps per column (PR #1449) |
| Storage | `mcdb` key layout, chunk version 42, `leveldat`, palette `Compact`, light BFS ring buffer (#1192) | Load/save on a blocking pool, never on the tick thread; snapshot dirty columns then write off-thread. CoW arenas: template chunks `Arc` shared, copy on first write. Provider returns `Option`, not a LevelDB error. Dirty tracking at component level (avoid #1400) |
| Generation | Flat generator | Same for milestone 1; when needed, a `populate(neighbours)` phase (#1263) |
| Handlers | Per-subject handler traits with `NopHandler` defaults, pointer-mutable params, `ctx.cancel()` | **Multiple handlers with priority + ordered dispatch** (dragonfly refuses this); handler registry per shard; cancel paths must resync the client (#749) |
| Plugins | — (refused upstream) | Rust crates registering handlers/commands/systems at startup; no hot reload in v1 |
| Commands | Struct-as-command, fields = params, overloads, `Allower` | Derive macro instead of reflection; per-player permission changes push an `AvailableCommands` diff instead of 5 s polling; registry per server, not global |
| Forms | Struct-defined custom forms, menu/modal | Derive macro; form callbacks keyed by id per session, run on the player's current shard |
| Scoreboard/UI | Sidebar writer | Full objectives (sidebar/list/belowname), multiple bossbars, diff-based line updates |
| Blocks/items | Value types + hash → runtime id; per-rid flag table; embedded `block_states.nbt` | Generate block/state enum tables at build time (acacia codegen); flag table as `Vec<u16>` indexed by rid |
| Testing | `Synchronous` world + `AdvanceTick` | A shard is a plain struct with `tick()`; drive it headless in tests from day one (dragonfly added this late) |
| Join | — | Atomic UUID reservation before StartGame (#1364) |
| Scope | Fixed 3 worlds | N shards created/destroyed at runtime (arenas) |

