# Worlds, storage and ECS — research for sapling

Researched 2026-10-03. Shallow clones in `C:/Users/Hexay/AppData/Local/Temp/sapling-res/` (dragonfly, df-mc/goleveldb,
leveldb-rs, bedrock-crustaceans/rusty-leveldb, bedrock-rs, chorus, valence, Minestom). Pumpkin is **GPL-3.0** — code not
read; only README and crate names.

## Licence map

| Project | Licence | Usable as reference/code? |
|---|---|---|
| dragonfly (+ `df-mc/goleveldb`, `df-mc/worldupgrader`) | MIT | yes |
| pmmp/BedrockBlockUpgradeSchema (data) | CC0-1.0 | yes — data only; PocketMine *code* is LGPL, don't port it |
| Mojang/leveldb | BSD-3 | yes |
| dermesser/leveldb-rs (`rusty-leveldb`) | MIT | yes |
| bedrock-crustaceans/rusty-leveldb (fork) | MIT | yes |
| BE-Community-Dev/bedrock-leveldb | MIT OR Apache-2.0 | yes |
| bedrock-rs (`bedrock_level`), Chorus | Apache-2.0 | yes (keep NOTICE if code is copied) |
| Valence | MIT | yes |
| Minestom | Apache-2.0 | yes |
| hollow-cube/polar (Minestom world format) | MIT | yes |
| Amulet-LevelDB | Amulet Team License (PolyForm Shield + Noncommercial) | **no** |
| Pumpkin | **GPL-3.0** | **no** (its plugin API crates say MIT/Apache, but the runtime is GPL) |
| AdvancedSlimePaper (Slime worlds) | **GPL-3.0** | concept only |

Version naming: Mojang now uses year-based versions. Chorus targets "26.50" (`chorus/README.md` badge), so "1.26.x" means
26.x. Chunk/sub-chunk versions below are from current MIT code and docs. **Check them against a freshly saved BDS 26.x world**
(acacia already downloads BDS) before freezing constants.

## 1. Vanilla Bedrock world format

### Directory
`<world>/level.dat`, `level.dat_old`, `levelname.txt`, `db/` (LevelDB: `CURRENT`, `MANIFEST-*`, `*.log`, `*.ldb`).

### level.dat
- 8-byte header: `i32 LE StorageVersion` (currently **10**) + `i32 LE payload length`, then **uncompressed little-endian NBT**
  (root compound). Sources: minecraft.wiki "Bedrock Edition level format"; `dragonfly/server/world/mcdb/leveldat/level_dat.go`, `version.go`.
- dragonfly rejects `ver >= 10 && ver != 10` (`mcdb/conf.go:58`). Write `MinimumCompatibleClientVersion` and
  `lastOpenedWithVersion` from your protocol version (`leveldat/version.go`).

### Chunk keys
`x:i32 LE | z:i32 LE | [dim:i32 LE, omitted for overworld] | tag:u8 | [subchunk_y:i8 for tag 0x2F]`. That gives 9/10/13/14
bytes; `JigsawStructureBlueprint` adds an 8-byte id, so 21 bytes (minecraft.wiki). Dimension ids: 0 = overworld, 1 = nether,
2 = end (`mcdb/db.go:554 index()`).

Official enum `LevelChunkTag` (Microsoft Learn, "Actor Storage"), plus wiki additions:

| byte | tag | notes |
|---|---|---|
| 0x2B `+` | Data3D | 512 B heightmap (256×i16) + per-subchunk biome paletted storages; a 0x7F "copy previous" marker repeats the previous palette (`chunk/decode.go:137`) |
| 0x2C `,` | Version | 1 byte. dragonfly writes **42** (`mcdb/db.go:150`). Vanilla still reads legacy `0x76 v` |
| 0x2D `-` / 0x2E | Data2D / Data2DLegacy | legacy biomes |
| 0x2F `/` | SubChunkPrefix | + i8 Y index |
| 0x30 | LegacyTerrain | pre-1.0 |
| 0x31 `1` | BlockEntity | concatenated LE-NBT compounds (not a list); position in `x,y,z` |
| 0x32 `2` | Entity (legacy) | concatenated LE-NBT; superseded by digp/actorprefix since 1.18.30 |
| 0x33 `3` | PendingTicks | NBT `{currentTick:i32, tickList:[{x,y,z,time:i64,blockState}]}` (`mcdb/db.go:508`) |
| 0x34 | LegacyBlockExtraData | |
| 0x35 | BiomeState | |
| 0x36 `6` | FinalizedState | u32 LE: 0 = needs ticking, 1 = needs population, 2 = done (`mcdb/keys.go:26`) |
| 0x37 | ConversionData | |
| 0x38 | BorderBlocks | Education Edition |
| 0x39 | HardcodedSpawners | |
| 0x3A | RandomTicks | |
| 0x3B | Checksums | xxhash; no longer written since 1.18 |
| 0x3C | GenerationSeed | |
| 0x3D | MetaDataHash | |
| 0x3E/0x3F | GeneratedPreCavesAndCliffsBlending / BlendingBiomeHeight | unused |
| 0x40 | BlendingData | |
| 0x41 | ActorDigestVersion | |
| 0x76 `v` | LegacyVersion | |
| 0x77 | AABBVolumes | undocumented |
| 0x78 | JigsawStructureBlueprint | |

dragonfly also writes a private `0x7C '|'` "legacy states" marker (`mcdb/keys.go:41`). That marker is non-vanilla; don't copy it.

### Actors (1.18.30+)
- `digp<chunk key>` → concatenation of `i64 LE` actor unique ids in that chunk.
- `actorprefix<i64 LE unique id>` → one LE-NBT compound per actor (`UniqueID` field matches).
- On save: write new actorprefix keys, delete stale ones, rewrite digp, and delete the legacy `0x32` key (`mcdb/db.go:418-464`).
  If digp is missing, fall back to `0x32`. Source: learn.microsoft.com/minecraft/creator/documents/actorstorage.

### Global keys
`~local_player`, `player_<uuid>` → `{MsaId, ServerId, SelfSignedId}` pointing at a `player_server_<uuid>` record
(`mcdb/db.go:62-136`). Also `AutonomousEntities`, `BiomeData`, `mobevents`, `scoreboard`, `portals`, `map_<id>`,
`structuretemplate*`, `tickingarea*`, `VILLAGE_*`, `DynamicProperties`, `game_flatworldlayers`, `Overworld`.

### Sub-chunk value (disk)
- `u8 version`. v1 has a single layer. **v8**: `u8 layer_count`. **v9**: `u8 layer_count, i8 y` (Y is absolute, not an index;
  `chunk/decode.go:109-123`).
- Each layer: `u8 header = bits_per_block << 1 | runtime_flag` (bit 0 = 0 on disk = persistent palette). Then
  `ceil(4096 / (32 / bits))` u32 LE words with no index spanning words. Index order is XZY with Y fastest: `(x<<8)|(z<<4)|y`.
  Then `i32 LE palette_len`, then palette_len LE-NBT compounds `{name:string, states:compound, version:i32}`.
  Valid bit widths: 0 (single value; Bedrock still writes a 1-entry palette), 1, 2, 3, 4, 5, 6, 8, 16.
- Layer 0 = blocks, layer 1 = waterlogging liquid.
- Entries older than the current block version (or `version < 17694723`, i.e. pre-1.13 `val` meta) must be upgraded. Use the
  CC0 BedrockBlockUpgradeSchema data, as dragonfly does through `worldupgrader` (`chunk/encoding.go:76-120`). Build this
  upgrader into sapling: templates authored on older versions are the common case.
- Network encoding differs: varint runtime ids instead of NBT, and bit 0 = 1.

### Mojang LevelDB variant
- Block trailer compression ids: **0 = none, 2 = zlib (with header; legacy), 4 = raw deflate (current default)**. Mojang falls
  back to id 0 when compression doesn't shrink the block. Sources: `leveldb-rs/examples/mcpe/src/main.rs`, comments in
  `bedrock-rs/crates/level/src/db.rs:14-22`, and Mojang/leveldb branch `ymekonnen/zlib` (`kZlibRawCompression = 0x4`).
- **Gotcha:** Mojang/leveldb `master` and the crustaceans fork's optional `zstd` feature both use **id 2 for zstd**. Bedrock
  worlds mean zlib at id 2. Never enable a zstd compressor on world DBs. Map 2 → zlib.
- df-mc/goleveldb's `FlateCompression` writes id 4 (`table/table.go:154`). Block size is a free choice: dragonfly uses 16 KiB
  (`mcdb/conf.go:42`), bedrock-rs 4 MiB (`level/src/db.rs:25`).
- Mojang's fork also changed table file naming and has `reuse_manifest` tweaks. The crustaceans fork falls back to legacy `.sst`
  names and defaults reuse_logs/manifest to false.

## 2. Pure-Rust LevelDB options

| Option | Thread model | Mojang zlib | Write path | Maturity |
|---|---|---|---|---|
| `rusty-leveldb` 4.0.1 (crates.io, dermesser) | **!Send** (`Rc<CompressorList>` in `Options`, Rc throughout); compaction inline on the writer | pluggable `Compressor` with ids; mcpe example registers 0/2/4 | full | 324k downloads, but README says "I do not endorse using this library for any data that you care about"; open issue #57 "Crash with potential data lose" (2026-03) |
| **bedrock-crustaceans/rusty-leveldb** (git fork, rev `3365b32`, 2026-07) | **Send + Sync**, `&self` writes, writer queue + group commit, background compaction thread, concurrent reads, parking_lot | same Compressor API; bedrock-rs registers 0/2/4 | full | ~60 commits Jun–Jul 2026: ported C++ test suites, proptest decoders, corruption returns errors instead of panicking, `repair_db`. Not on crates.io |
| `bedrock-leveldb` 0.3.1 (BE-Community-Dev) | lock-free reads, parallel table scans | snappy/zlib/raw deflate behind features | WAL + `.ldb` flush + manifest; "correctness-first native range compaction" | 4 stars, 950 downloads, raw KV only; young |
| `bedrock_level` (bedrock-rs crate) | wraps the crustaceans fork | ids 0/2/4, level 6 | KV + lazy/greedy sub-chunk unpacking, key helpers, player/settings | Apache-2.0, git-only, API churn (moves with bedrock-rs) |
| `mojang-leveldb` 0.7.0 | FFI to Mojang C++ | native | full | not pure Rust; needs a C++ toolchain; last release 2024-11 |

**Recommendation:** put sapling's own `KvStore` trait (`get/put/delete/write_batch/prefix_iter`) under a `LevelDbStorage`
that implements `WorldStorage`. Back it with the **crustaceans rusty-leveldb fork, pinned by rev** (MIT, Send + Sync,
background compaction: the only option that fits shard threads without a dedicated actor thread). Register 0 = none,
2 = zlib, 4 = raw deflate, and write with 4. Write sapling's own codec (keys, sub-chunk v9, palettes, digp/actorprefix)
rather than depending on `bedrock_level`: it is small (dragonfly's mcdb + chunk is ~2.7k lines of Go) and keeps the
chunk representation (§3) under sapling's control. Keep `mojang-leveldb` or vanilla BDS as a **differential test oracle**:
round-trip a BDS world, then load the result in BDS. If the fork is abandoned, vendor it (MIT) or switch the `KvStore`
implementation to `bedrock-leveldb`.

Write pattern: shards never call the DB on the tick thread. Load is async: an I/O pool decodes into a `Section` value.
Saves snapshot dirty chunks (cheap with Arc sections, §3) and run one `WriteBatch` per chunk on the I/O pool, as dragonfly
batches per column (`mcdb/db.go:368`).

## 3. Copy-on-write arenas

### Prior art
- **Minestom** (Apache-2): `InstanceContainer.copy()` **deep-copies every chunk** (`InstanceContainer.java:622`).
  `SharedInstance` shares one container's chunks; setBlock, load and unload are forwarded, and only entities are separate
  (`SharedInstance.java:18-60`). So it offers either full copies or fully shared, mutable chunks. There is no CoW.
- **Polar** (MIT, hollow-cube) and the Slime format (ASP, GPL; concept only): compact single-blob template worlds, loaded
  once into memory and instantiated per game without disk I/O. This is how Hypixel-style minigame networks scale arenas:
  read-only template bytes plus a cheap per-instance world.
- **Valence** (MIT): a chunk caches its encoded init packet (`cached_init_packets: Mutex<Vec<u8>>`,
  `layer/chunk/loaded.rs:44`) and invalidates it on mutation. Paletted containers are `Single | Indirect(Box) | Direct(Box)`
  (`paletted_container.rs:11`), so uniform sections cost almost nothing. There is no CoW: `Clone` is deep.
- **Chorus**: `#[derive(Clone)] Chunk { sub_chunks: Vec<SubChunk> }` is also a deep clone (`chorus/crates/level/src/chunk.rs:3`).

### Recommended design for Rust
```
TemplateWorld (immutable, Send+Sync, Arc)        ArenaWorld (owned by one shard)
  chunks: FxHashMap<ChunkPos, Arc<ChunkData>>      base: Arc<TemplateWorld>
  ChunkData { sections: Box<[Arc<Section>]>,       overlay: FxHashMap<ChunkPos, ChunkData>  // only touched chunks
              block_entities: Arc<BeMap>,          (ChunkData here shares untouched Arc<Section>s with the base)
              net_cache: OnceLock<Bytes> }
  Section { layers: [Palette; ≤2], biomes: Palette, blob_hash: OnceLock<u64> }
```
- **Clone of an arena is O(1)**: `Arc::clone(&template)` plus an empty overlay. A read checks the overlay, then the base.
- **First write to a chunk** shallow-clones `ChunkData` into the overlay (copies one array of N Arc pointers). The write then
  calls `Arc::make_mut` on one section, cloning ≤ ~10 KB. Section (16³) granularity is the sweet spot. Per-block persistent
  structures (HAMT/`im`) are too slow on the hot path; whole-chunk copies waste memory.
- `Palette` mirrors Valence: `Single(id) | Indirect{palette, packed} | Direct(Box<[u16;4096]>)`. Use runtime-id `u16/u32`
  in memory and NBT only at the disk edge.
- **Network caching is the big win.** Encode sub-chunk payloads once per template section and share the `Bytes` across all
  arenas until a section diverges. Bedrock's client blob cache (ClientCacheStatus / blob hashes, xxhash64 of sub-chunk
  blobs) means identical template sections across arenas hit the *client's* cache too. Store `blob_hash` on `Section`.
- Lighting: the Bedrock client computes its own lighting, so don't store light. That saves memory compared with
  Java-edition designs.
- Block entities: `Arc<BTreeMap<BlockPos, Nbt>>` per chunk with `make_mut`. Template entities are *spawned* into the shard's
  ECS when an arena starts, not shared.
- Reset an arena by dropping the overlay. "Save arena back to template" means merging the overlay into a new `TemplateWorld`.
- `WorldStorage` impls: `LevelDbStorage` (vanilla, read/write) and `TemplateStorage` (in-memory, read-only base). Templates
  load from LevelDB once at startup. An optional compact format (Polar-like, zstd blob) can come later for fast boot.

## 4. Chorus, Pumpkin, Valence — architecture lessons

### Chorus (Apache-2, bevy_ecs 0.19.1, active 2026-10)
- Uses one `bevy_app::App` for the whole server and a custom `LoopRunner` (`chorus/src/lib.rs:39-75`): fixed tick, a
  `JobQueue` of one-shot `SystemId`s drained in leftover tick time, then `spin_sleep`.
- **Networking runs inside the ECS.** `bevy-raknet`/`bevy-nethernet` plugins with accept, receive and flush systems
  (`src/network/network.rs:44-89`). Each session is an Entity (`NetworkState.sessions: HashMap<SessionId, Entity>`).
- **The world is one Resource.** `Level { dimensions: HashMap<i32, Dimension> }` (`crates/level/src/level.rs:16`), and
  `Dimension { chunks: HashMap<(i32,i32), Chunk>, modified }`. Chunks are not entities. Blocks are `i32` runtime ids.
  Sub-chunks always allocate two layer palettes (`sub_chunk.rs:15`).
- Events are bevy `Message`s: ~22 message types registered in one place. One `PacketReceivedMessage { entity, packet: enum }`
  feeds every handler system (`src/network/handler/mod.rs:32-56`).
- Plugins are plain bevy `Plugin`s. No persistence yet: LevelDB is used only for BDS worldgen parity tests (`bds_parity.rs`).
- Strength: a strong worldgen parity harness against BDS output. Copy the idea of BDS-parity tests.
- **Mistakes to avoid:**
  1. `ResMut<Level>` is a global write lock in bevy terms. Every block-touching system serialises.
  2. The handler tuple is `.chain()`ed end to end (`handler/mod.rs:42-56`), so the schedule is fully serial and bevy's
     parallelism is unused.
  3. Every handler scans the whole packet stream and filters by enum variant (O(handlers × packets)).
  4. Socket I/O on the tick thread.
  5. Deep-cloneable chunks with no CoW.
  6. bevy `Plugin` as the public API ties users to bevy versions. Chorus is already on 0.19.

  sapling's design (handler traits outside, bevy_ecs inside each shard) already avoids (6). Avoid (1) with per-arena
  `ChunkLayer` components or entities, (3) by routing packets per type into typed queues, and (4) with network threads
  plus channels.

### Pumpkin (GPL-3.0 — not read beyond README and crate list)
- Java-edition server, "Bedrock W.I.P". Crates: `pumpkin-world`, `-protocol`, `-scheduler`, `-plugin-api`, `-plugin-wit`,
  `-wasm-host-v0_1`, `-wasm-host-v0_2`, `-wasm-host-common`. Formats: Vanilla/Anvil, Linear, "Pump".
- Lesson: it runs **WASM component-model plugins (WIT) with versioned host crates** to keep old plugin ABIs alive. If
  sapling ever wants sandboxed or hot-reloadable plugins, design the handler-trait API so it can be mirrored to WIT later
  (plain data in and out, no borrowed ECS types).
- No ECS mentioned in the README. Unverified: community reports describe a lock-heavy, `Arc<Mutex>` entity model; sapling's
  shard-owned ECS sidesteps that class of problem by construction.

### Valence (MIT, bevy_ecs 0.14, last push 2026-06-15, effectively stalled on old bevy)
- **Layers:** a layer is an *entity* with `ChunkLayer` and/or `EntityLayer` components (`LayerBundle`,
  `valence_server/src/layer.rs:120`). A client has `VisibleChunkLayer(Entity)` (exactly one) and
  `VisibleEntityLayers(BTreeSet<Entity>)` (many), plus `Old*` copies for diffing on change (`client.rs:564-596`).
  This is the "instance" model: many worlds in one bevy `World`, and switching an instance is a component write.
- **Layer messages:** systems write packet bytes into per-layer `Messages<Global, Local>` buffers. Local messages are
  indexed by a chunk-position BVH, then sorted and deduped at end of tick (`layer/message.rs:10-37`). Each client then pulls
  only the messages in its view. Broadcast encoding happens once per layer instead of once per client. **Copy this.**
- System sets `UpdateLayersPreClientSet` / `UpdateLayersPostClientSet` (`layer.rs:28-45`) give a clear write-then-flush
  boundary.
- Weaknesses: everything is in one `World` on one schedule, so no horizontal scaling. The bevy-as-public-API coupling left
  it stuck when bevy moved on. Chunks are not CoW.

### Mapping to sapling
- **Shard** = OS thread + its own `bevy_ecs::World` + `Schedule` (no `bevy_app` needed). Many **arenas/worlds per shard**,
  each an entity with `ChunkLayer` (an `ArenaWorld` from §3) + `EntityLayer` + `LayerMessages`. A player entity has
  `InWorld(Entity)`.
- Moving a player between arenas on the same shard is a component write. Moving to another shard serialises player state
  and hands it over through a channel.
- Packets arrive from network threads through a per-shard MPSC, split by type into typed queues or events. Outbound
  packets are encoded once per layer (Valence messages) and flushed to network threads at end of tick.
- Plugin handler traits run at defined schedule points through an adapter system. They see a sapling-owned facade, not
  bevy types, which leaves the bevy version free to change and keeps the WIT option open.

## Sources
- learn.microsoft.com/en-us/minecraft/creator/documents/actorstorage (LevelChunkTag enum, digp/actorprefix)
- minecraft.wiki/w/Bedrock_Edition_level_format (key lengths, tags 0x77/0x78, level.dat header, sub-chunk layout)
- github.com/Mojang/leveldb (branches `master`, `ymekonnen/zlib`: compression enums)
- dragonfly `server/world/mcdb/{db,keys,conf}.go`, `mcdb/leveldat/*`, `server/world/chunk/{decode,encoding}.go`
- df-mc/goleveldb `leveldb/table/{table,writer}.go`
- dermesser/leveldb-rs `Cargo.toml`, `README.md`, `examples/mcpe/src/main.rs`, `src/options.rs`
- bedrock-crustaceans/rusty-leveldb commit log (2026-06-21 … 2026-07-11)
- crates.io API: rusty-leveldb, bedrock-leveldb, mojang-leveldb; BE-Community-Dev/bedrock-leveldb README
- bedrock-rs `crates/level/src/{db,lib,traits}.rs`, `crates/level/Cargo.toml`
- chorus `src/lib.rs`, `src/network/{network.rs,handler/mod.rs}`, `crates/level/src/{level,chunk,sub_chunk}.rs`, `generator/dimension.rs`
- valence `crates/valence_server/src/{layer.rs,client.rs,layer/message.rs,layer/chunk/{loaded,paletted_container}.rs}`
- Minestom `instance/{InstanceContainer,SharedInstance}.java`
- github.com/Pumpkin-MC/Pumpkin README and `crates/` listing; hollow-cube/polar and InfernalSuite/AdvancedSlimePaper repo metadata
