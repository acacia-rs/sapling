# BDS ender dragon fight manager (bdsre reading)

Build 6ab54bee (BDS 1.26.52.3, `.testserver/bds-molang`). Static reading only, no drill yet. House style as
end-dimension.md: an address next to each claim. "Unverified" marks a reading not confirmed by a second path.
Pseudo-C offsets checked against `dis` where they mattered. Functions named in bdsre (`name`) as listed.

Block ids (VanillaBlockTypeIds, init 1425f41e0): 14ccd9718 bedrock, 14cce12e8 end_stone, 14ccdb078 end_portal,
14ccdb310 torch, 14ccdd340 dragon_egg, 14cce16a8 obsidian, 14ccdb710 fire; 14ccf9950 is air by use (crystal
item, findExitPosition). States: 14cd22b28 `infiniburn_bit`, 14cd24fb0 `torch_facing_direction`.
Actor types: 0x13f player, 0xb35 ender_dragon, 0x47 ender_crystal.

## 1. State and lifecycle

**Object.** `TheEndDimension` (vtable around 14a925530, the bdsre label `SleepCounterSystem::vtable` on it is
wrong) owns the fight at `this+0x4f0`. Built inline by `TheEndDimension::init` 1482757b0 (slot 45). Its
save/load are slots 107/108 of a secondary base, so there `this` is +0x10 and the fight reads as `+0x4e0`.

Fields (byte offsets, from ctor 1482757b0, load 148279910, save 148279ee0, tick 148276910):

| off | field | init |
|---|---|---|
| 0x00 | `BlockSource*` of the End | dimension+0xf0 |
| 0x08 | `vector<int>` gateways left (indices 0..19) | empty |
| 0x28 | crystals alive (int, `updateCrystalCount`) | 0 |
| 0x2c | ticks since crystal count | 0 |
| 0x30 | ticks since exit portal scan (killed branch) | 0 |
| 0x34 | 20-tick counter (see below) | 0 |
| 0x38 / 0x39 / 0x3a | dragonKilled / previouslyKilled / dragonSpawned (bytes) | 0 |
| 0x40 | dragon ActorUniqueID (i64) | -1 |
| 0x48 | exitPortalLocation BlockPos | (-1,-1,-1) = unset |
| 0x54 | portal centre / dragon spawn BlockPos | (0,128,0) |
| 0x60 / 0x64 | respawn stage (int) / ticks in stage (int), written together as a qword | 0 / 0 |
| 0x68 | `vector<ActorUniqueID>` respawn crystals | empty |
| 0x80 | DragonFightVersion (byte) | |
| 0x88..0xc0 | current gateway job (place-main flag 0x88, sources 0x90/0xb0, main pos 0x98, place-far 0xa8, far pos 0xb8) | 0x88=1, 0xa8=1 |
| 0xc8 | gateway job state: 2 idle, 0 loading, 1 loading (relink) | 2 |
| 0xd0 | `deque` of queued gateway jobs (0x48 bytes each) | |

**Tick** (`TheEndDimension::tick` 148276910, fight inlined), in order each End tick:
1. (Not the fight) +0x4f8 object: an End sky/flash effect, event 0x22c 50 blocks along a random direction. Skip.
2. `+0x34` counter: incremented; when the old value was >= 0x13 (every 20 ticks) reset and
   `BlockSource::getDimension()->[+0x18] = 1` (148276d45..d6d; target flag unidentified).
3. Gateway job machine (section 4), runs with or without players.
4. **Player gate**: `fetchEntities(Player 0x13f, AABB(-200,-200,-200)..(201,201,201))` (AABB from 14a9255a0 +
   immediate); none: return. So with no player within ~200 of the origin the fight is frozen (counters too).
5. **Chunk gate**: `getChunkAt(+0x54)` and `getChunk(ChunkPos)` for (-1,-1) (-1,0) (-1,0) (0,-1) (0,0) (0,0)
   (0,-1) (0,0) (0,0) (148277741..95b; Java's -8..8 step 8 shifted by 4, so chunks (-1|0, -1|0)) must exist and
   have `LevelChunk+0xa8 == 0` (by the header, the size of `mSerializedEntitiesBuffer`: entities already loaded;
   unverified). Else return.
6. **Stage 0 (no respawn)**:
   - not killed and `!level->isClientSide()` (vt 0x9e8): needs `GameRules[41]` (AllowMobs, edu, default true)
     and a level setting at `level[0xf1]+0xd8` (when its +0x138 says set; unidentified), else return.
     - `level->fetchEntity(uuid)` (vt 0x1f0): if found and type 0xb35: `_initializeDragon` 148273360.
     - not found: `fetchEntities(0xb35, AABB origin ±200)`; first hit: `_initializeDragon`. None: if
       `dragonSpawned` and `level+0x438` still knows the uuid (14165acb0: an unloaded or stored actor) do
       nothing; else `_createNewDragon` 148272c10 then `spawnExitPortal(false)` 1482731d0. Runs every tick
       (no Java 1200-tick wait).
     - `+0x2c`++; after 100 ticks (old value > 0x62): `updateCrystalCount` 1482733c0, reset.
   - killed (or client): `+0x30`++; every 100 ticks: scan chunks (-8..8)x(-8..8) block actors for type 0x16
     (end portal); none found: `spawnExitPortal(true)`.
7. **Stage != 0**: respawn machine (section 6).

**`_createNewDragon`** 148272c10 (server only): ActorFactory (level vt 0x908) builds `minecraft:ender_dragon` at
`+0x54` = (0,128,0) (block corner, no +0.5); `moveTo(pos, (yaw = level random nextFloat * 360, pitch 0))`
(140d30f00); `level->addEntity(region, actor)` (vt 0x168). If added: `dragon+0x844 = previouslyKilled`, the kill
callback (`lambda_4`, `std::function` set by 145998800, stored at dragon+0x880), `uuid = dragon id`,
`dragonSpawned = 1`. Then for each spike of `getSpikesForLevel`: if `SpikeFeature::isSpikeDamaged` 149d64ef0 (a
cell of the disc `d² <= r²+1` at y = height-1 is not obsidian) re-place the spike with `SpikeFeature::place`
149d63340 at (x, 45, z) (obsidian, air, cage; **no crystal**).

**`_initializeDragon`** 148273360: `dragon+0x844 = previouslyKilled`, kill callback, uuid, `dragonSpawned = 1`.

**`updateCrystalCount`** 1482733c0: `+0x28 = sum over spikes of fetchEntities(0x47, spike AABB)`; the spike's
AABB is its record +0x14..+0x2c (record 0x2c bytes: x, z, radius, height, guarded, AABB).

**Spike crystals at fight start**: none are placed by the fight. Only `postProcessMobsAt` 148246230 or a respawn
(stage 3) place them, matching the drills (no crystals, no spike bedrock with a player present). The caller of
generator slot 16 (`ChunkSource::postProcessMobsAt`: `WorldGenerator` derives from `ChunkSource`) was **not
found**: a byte scan for `[reg+0x88]` (LevelChunk::mGenerator) then `[rax+0x80]` hit only an item-stack function.

## 2. Boss bar

Not fight code: the generic `minecraft:boss` component from ender_dragon.json (`hud_range: 125`,
`should_darken_sky: false`), BossComponent (0xa8 bytes; name string at +0, `+0x48` visible, `+0x4c` health
fraction f32, `+0x52/+0x53` style bytes, `+0x5c` range compared with squared distance, so presumably 125²).
- Add: `BossComponent::tryRegisterPlayers` 141b78bc0, from `BossComponent::tick` 148484060 once per **1 s of
  steady-clock time** (not ticks): every player of the level in the boss's dimension, not removed, within the
  range, not already tracking it, `+0x48 == 1`: BossEvent type 0 (Add).
- Health: `BossComponent::tick`: on health change `+0x4c = min(health/max, 1)`, BossEvent 4 (UpdatePercent)
  to tracked players (141b78860).
- Remove: per player tick (1402a5a90) over the player's tracked list (player+0x818): boss gone, or other
  dimension or out of range (`BossComponent::tryRemovePlayer` 141b790e0): BossEvent 2 (Remove).
- Sender `sendBossEvent` 141b782d0: packet with boss unique id, type, name, the +0x4c/+0x52 fields; to the one
  player. Client-sent register/query handled by 141b781d0 / 141b79250 (from 14097bfb0). Exact payload field
  mapping (colour, overlay, darken) not read.
- Death: `EnderDragon::tickDeath` 141004b10 sets the fraction to 0 and sends UpdatePercent at once.

## 3. Exit portal

**`spawnExitPortal(active)`** 1482731d0 (server only):
1. If exit is unset and chunk (0,0) is loaded: exit = (0, `getAboveTopSolidBlock(0, 0, false, false)` (vt
   0x178), 0), then while the block there is bedrock and y > the dimension's min height, y--. Exit stays (-1,-1,-1)
   if chunk (0,0) is not loaded (the feature still runs, at (-1,-1,-1)).
2. `EndPodiumFeature::place` 144c12de0 (feature vtable 14a853000, +0x20 = active, +0x18 no buffer) at exit with the
   level random (unused).

**Pattern** (origin O = exit; d² = dx²+dy²+dz², integers; loop dx -4..4 outer, dy -1..4, dz -4..4 inner; every
write `setBlock(pos, block, flags 3)`):
- dy = -1: d² < 7: bedrock `infiniburn_bit=1`; 7 <= d² < 13: end_stone.
- dy = 0: d² < 7: active: air, then end_portal (two writes); inactive: air. 7 <= d² < 13: bedrock (infiniburn).
- dy 1..4: d² < 13: air.
- Then the pillar O+(0,0..3,0) bedrock (infiniburn), bottom up.
- Torches at y O+2, `torch_facing_direction`: (0,2,-1) = 4 (south), (0,2,+1) = 3 (north), (+1,2,0) = 1 (west),
  (-1,2,0) = 2 (east), in that order.
On BDS's island O = (0,62,0): pillar 62..65, torches 64, floor 61 (matches the drills).

**Finding an existing portal** (`tryRespawn` only): BlockPattern built in `init` (1482757b0, 5 layers of 7x7,
'#' = bedrock): three layers centre only, then the ring layer, then the disc. Searched at every type-0x16 block actor
in chunks -8..8, then down column (0,0) from the heightmap; a match sets exit if unset; none:
`spawnExitPortal(true)`.

## 4. Dragon death in the fight

**Kill callback** `lambda_4` call 14829b510, invoked by `EnderDragon::tickDeath` at death tick >= 200 (below).
If the dragon's id == fight uuid:
1. `spawnExitPortal(true)`.
2. Gateway: if the gateway list is empty, refill 0..19 and shuffle (below). Pop the **last** entry i;
   `a = (i * 0.15707964f + -3.1415927f) * 2` (f32); pos = (`(int)(cosf(a) * 96)`, 75, `(int)(sinf(a) * 96)`),
   truncation, UCRT cosf/sinf (14829b719..763). `_spawnNewGateway(pos, placeMain=1, placeFar=1)` 148273bb0.
3. If not previouslyKilled: dragon_egg at (0, `getAboveTopSolidBlock(0,0)`, 0), flags 3 (on the pillar top).
4. `killed = 1, previouslyKilled = 1` (one word write at +0x38).

**Gateway list** (load 148279910, save 148279ee0, kill callback): `[0..19]`, MT19937 seeded with the level seed
(`level vt 0x430`, u32), forward Fisher-Yates from index 1: `j = MT() % (i+1)`, swap a[i], a[j] when different
(as the spikes). Created on load if the tag is missing, on save if empty, on a kill if empty.

**Gateway jobs** (`_spawnNewGateway` 148273bb0): exit = `findExitPosition` 1487fd350 (Java's: (x,0,z) normalised
(length < 1e-4: zero) x 1024, up to 16 steps of -16 while the chunk is not empty, then up to 16 of +16 while empty;
1487fd8c0 tests a chunk). The job runs now if state is 2, else is queued.
- Start (148273da0): two chunk-view sources (vtable 14a71b330) load regions round the main pos and the exit
  (1409ea4d0, radius arg 0xa0); state 0.
- Tick, state 0, both loaded (148272080 x2): `p = findValidSpawn(exitSource, exit)` 1487fde00; none:
  `(exit.x, 75, exit.z)` and an end island there (148274c80); then `findTallestBlock(p, 16, true)` 1487fdbb0 + 10 up
  = far gateway pos. `EndGatewayFeature` (vtable 14a925280) places the main gateway when +0x88, the far one when
  +0xa8 (144c120f0). Main gateway's block actor (type 0x18) exit = tallest block within 5 round
  `findValidSpawn(far)` + 1 up; far gateway's exit = `findValidSpawn(mainSource, main pos, 0, 3)`; both get
  `+0x106 = 0, +0x107 = 1` and are marked changed. State 2. So the far island and gateway are made **at kill
  time** (once both regions load), not on first use. Java's level event 3000 on spawn: not seen.
- A gateway block actor without `ExitPortal` (or (0,0,0)), load 1487fc680 `+0x104`: queues `(placeMain 0,
  placeFar 1)` from its tick 1487fcae0. One with an exit gets `+0x106`: queues a relink job (1482743c0 →
  148274560, state 1, 148274e90 links both ends to the nearest gateway within ±32 of the exit); with
  `EndGatewayBadPosChecked` unset, |exit|₁ < 100 and |pos|₁ < 300 it first recomputes the exit.

## 5. Experience and death animation

`EnderDragon::tickDeath` 141004b10 (from `ServerEnderDragonTickDeathSystem::fn0` 1424ebcf0), t = death ticks:
- every tick: boss fraction 0 (once), posDelta y/z cleared (+0x18/+0x20 of the state vector), `move(0, 0.1, 0)`
  (140d24550).
- total XP = 12000, or 500 when `dragon+0x844` (previouslyKilled copied from the fight).
- t > 150 and t % 5 == 0: drop `floor(total * 0.08f)` XP at the dragon's position (141171200, kind 2; orb split
  in that function, not read).
- t == 2: sound event 14 (Death) (142fa5770).
- t >= 200: drop `floor(total * 0.2f)`, call the kill callback (dragon+0x880), `remove()` (vt 0x58).
JSON: `on_death` → `minecraft:start_death` → `behavior.dragondeath`. Particles/explosions of the animation are in
that behaviour (not read).

## 6. Re-summoning

**`tryRespawn`** 148272320, from `EnderCrystalItem::useOn` 14805e190 (in the End) and from the tick:
- Only if killed and stage 0, else clear the crystal list. Exit unset: pattern search (section 3).
- Clear the list; four 1x1x1 boxes at exit + (0,1,+3), (-3,1,0), (0,1,-3), (+3,1,0) in that order; each
  `fetchEntities(0x47, box)`: every crystal gets the `lambda_5` destroyed-callback (145993230, crystal+0x3b0) and
  its id appended. An empty side: return (stage stays 0).
- All four: stage = 1 (dword write; counter not reset here), `spawnExitPortal(false)`.

**Stage machine** (148276910 from 14827796b; `t` = ticks in stage before the increment; every stage change is a
qword write so t restarts at 0):
- Gate: `level vt 0x3a0` returns an object; continue only if null or its byte +2 != 1 (unidentified).
- Crystal list empty (e.g. after load, ids are not saved): stage 0, `tryRespawn` again.
- 1 START: each listed crystal's beam target (actor data 47 BlockTarget, BlockPos) = (0,128,0); stage 2.
- 2 PREPARING: t >= 100: stage 3. t in {0, 50, 51, 52} or 95..99: level sound event 0x13 at (0,128,0)
  (`level->playSound(region, 0x13, pos, -1, ...)`, 148274aa0 builds its data). Unverified which sound 0x13 names.
- 3 SUMMONING_PILLARS: k = t / 40, s = t % 40; only s in {0, 38, 39}. k >= spike count: at s == 0 stage 4.
  Spike k (list order of `getSpikesForLevel`):
  - s 0: crystals' beam target = (x, height+1, z).
  - s 38: `removeBlock` over x-10..x+10, h-10..h+10, z-10..z+10 (x outer, y, z inner), then
    `level->explode(region, null, (x+0.5, height, z+0.5), 5.0, fire false, breaks blocks true, FLT_MAX, false)`.
  - s 39: `SpikeFeature::place` (149d63320 → 149d63340 with flag 1) at (x, 45, z), then
    `SpikeFeature::placeCrystal` 149d65290: crystal at (x+0.5, height+1, z+0.5), yaw random*360, ShowBottom (flag
    38) on, invulnerable (crystal+0x1a4) = 1, beam target (0,128,0); bedrock `infiniburn_bit=1` at (x, height, z).
- 4 SUMMONING_DRAGON: t == 0 beam target (0,128,0); t 1..4 and 80..99 sound 0x13; t >= 100: stage 0,
  killed = 0, `_createNewDragon`, an event to every End player (142d3c8f0 'B'), `resetSpikeCrystals` 1482754c0,
  then each respawn crystal: beam target (0,0,0), `level->explode(region, crystal, crystal pos, 6.0, no fire,
  no block damage, FLT_MAX, false)` (vt 0x3c8), `crystal->remove()`.
- **Crystal destroyed during a respawn** (`lambda_5`/`lambda_6` call 14829b890): stage 0, `resetSpikeCrystals`,
  `spawnExitPortal(true)`.

**`resetSpikeCrystals`** 1482754c0: every crystal in each spike AABB: invulnerable = 0, beam target (0,0,0),
destroyed-callback `lambda_6`.

## 7. End crystals

- ender_crystal.json: health 1, fire immune, 2x2 box, `on_hurt` → `crystal_explode` → `minecraft:explode` power 6,
  `causes_fire` false, fuse 0, `destroy_affected_by_griefing` true. The crystal is the explosion's source.
- `EnderCrystal::_hurt` 1459930e0: server: if `GameRules[40]` (AllowDestructiveObjects) is false the crystal is
  removed with no explosion. Else when damage > FLT_EPSILON the destroyed-callback (crystal+0x3e8) runs before
  `Actor::hurt` (140d397e0, which fires on_hurt). The invulnerable byte +0x1a4 is not read here (where it blocks
  damage: not found).
- Callback (14829b890), outside a respawn: `updateCrystalCount`, then if the dragon is loaded:
  `dragon+0x840 = count` and `EnderDragon::onCrystalDestroyed(dragon, crystal, floor(crystal pos), source)`
  145998d00 (dragon side not read).
- `EnderCrystal::normalTick` 145992a90, End only: if the cell below the floored position is not air and the
  crystal's own (floored) cell is air, fire in its own cell (flags 3).
- ShowBottom is actor flag 38 (NBT `ShowBottom`, 140d48ac3). BlockTarget save/load 145992e40 (`BlockTargetX`..).
- `EnderCrystalItem::useOn` 14805e190: clicked obsidian or bedrock; cells y+1 and y+2 air; no actor in AABB
  (x,y,z)-(x+1,y+2,z+1) (clicked y, as read); server: spawn crystal at (x+0.5, y+1, z+0.5) (ShowBottom not set),
  item-use event, game event; in the End `tryRespawn`; one item spent outside creative. Returns success 3.
- Healing: the dragon's crystal search is in 145994090 (AABB ±32 round the dragon, `fetchEntities`); rate not read.

## 8. Persistence

`TheEndDimension` serialize 148279ee0 / deserialize 148279910 write and read compound `DragonFight` inside the
dimension's tag:

| tag | type | field |
|---|---|---|
| DragonUUID | Int64 | +0x40 |
| DragonKilled | Byte | +0x38 |
| DragonSpawned | Byte | +0x3a |
| PreviouslyKilled | Byte | +0x39 |
| IsRespawning | Byte (stage != 0) | +0x60 |
| DragonFightVersion | Byte | +0x80 |
| ExitPortalLocation | List of 3 Int, only when set | +0x48 |
| Gateways | List of Int | +0x08 |

Load: without `DragonKilled` the killed and spawned bytes are cleared and the other flags, uuid and exit are
skipped (`Gateways` is still read); version 0
or missing → `PreviouslyKilled = 0`, version := 1; `IsRespawning` true → stage 1, t 0 (crystals rescanned).
Gateways missing → fresh shuffle. Not traced: the LevelDB key holding the dimension tag (probably `TheEnd`).

## 9. Exit portal travel and credits

- `EndPortalBlock::entityInside` 148a2e480: a player in the End (dimension 2) calls vt 0x590
  `Player::changeDimensionWithCredits(0)` 140759540; other actors use 0x2c0 (changeDimension).
- `changeDimensionWithCredits`: saves the position to player+0xb64. Unless `level vt 0x1b0(player)` or not
  `vt 0x680`: if player+0x70d (credits already seen) set: +0x70a = 1, `respawn()` (vt 0x5d8). Else: 1402a90c0,
  ShowCreditsPacket (vtable 14a7aca40, runtime id, state 0 = start, unverified), `level vt 0x1a0(player)`,
  `DimensionStateComponent = 1`, add `IsShowingCreditsFlagComponent`.
- Client answers ShowCredits state 1 (finished): `ServerNetworkHandler::handleShowCredits` 140979d40: only while
  flagged; `level vt 0x1a8(player)`, +0x70d = 1, +0x70a = 1, `respawn()`, remove the flag.
- `respawn()` leads to the respawn state machine in `tickWorld` 1402beeb0 (end-dimension.md, "Dying in the End"):
  bed or world spawn in the overworld. Whether +0x70d is saved in the player's NBT was not checked.

## Left open
- The caller of generator slot 16 (`postProcessMobsAt`), so when spike crystals appear in a fresh End.
- The flags written every 20 ticks (dimension+0x18), the level setting next to AllowMobs, the `vt 0x3a0` gate.
- Sound 0x13's name; BossEvent payload fields; the XP orb split (141171200); dragon heal rate (145994090).
