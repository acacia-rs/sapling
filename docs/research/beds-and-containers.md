# Beds and container screens: BDS ground truth for sapling (2026-10-06)

Tags: **BDSRE** = current exe (`acacia/.testserver/bds-molang/bedrock_server.exe`, bdsre cache `6ab54bee`), high
confidence unless marked; **REC** = recorded from BDS (`acacia/docs/research/vanilla-actions-2026-10-02.md`
"Bed", `survival-signs-beds.md` §5); **Java** = 26.3 server jar (javap); **mem** = PocketMine/dragonfly as
remembered, medium; **inf** = inferred. Read a function: `BDS_EXE=<exe> py -3 C:/Users/hexay/acacia/bdsre/bdsre.py c <va>`.
Names are acacia-proto's. Container basics (open, lid, contents) are in `containers-crafting.md` §A; not repeated.

## 1. Clicking a bed: checks in order

`BedBlock::use` (142cd8100, the `BlockPlayerInteractEvent` handler registered in the BedBlock ctor 142cd7470):

0. Resolve the **head** cell from the clicked half (142cd8540, reads `head_piece_bit` + `direction`). Every later
   step uses the head position. BDSRE
1. Dimension cannot hold a bed (dimension vt+0xd0 false: nether, end): if game rule `respawnBlocksExplode`, both
   halves become air (flags 3) and an explosion of power 5.0 with fire goes off at the midpoint of the two cell
   centres. No message. BDSRE
2. Otherwise `BedBlock::use_sleep` (142cd8730) with `setsRespawn = true`:
   1. `occupied_bit` set → look for **another** sleeping player whose `PlayerBedPosition` == head pos (lambda
      142cfb440). Found → `tile.bed.occupied`. Not found → stale bit: clear it (setBlock flags 2), go on. BDSRE
   2. `Player::startSleepInBed(head, setsRespawn, sleepOffset)` (1402a24a0) returns `BedSleepingResult`; checks
      in this order:
      1. Water in the bed cell's liquid layer and no water-breathing (13) / conduit-power (26) effect and no
         respiration on armour → result 4, silent. (BDSRE; reading of the block layer is inf.)
      2. Already sleeping, dead, or riding → 4, silent. BDSRE
      3. **Distance**, per axis against the head cell's integer corner (not its centre), using the player's
         position (Bedrock player position = eyes, feet + 1.62): `|px-bx| > 3 || |py-by| > 4 || |pz-bz| > 3` →
         3 `tile.bed.tooFar`. BDSRE
      4. Dimension refuses beds → 1, silent (already handled by step 1). BDSRE
      5. No stand-up position around the bed (`BedBlock::findStandUpPosition` 142cd9510) → 6
         `tile.bed.obstructed`. BDSRE
      6. **Respawn point set here, before the time check**: `setBedRespawnPosition` (1402be4d0) sends
         `SetSpawnPosition {spawn_type 0 (player), player_position = floor(player feet), dimension 0,
         world_position = bed head}`; only if it changed, also the `tile.bed.respawnSet` message. So a daytime
         click still moves the spawn. BDSRE; REC (SetSpawnPosition +86 ms after the click).
      7. Dimension has a day cycle and `skyDarken < 4` → 2 `tile.bed.noSleep`. BDSRE
      8. Not creative and difficulty ≠ peaceful and any actor of type family Monster (0xb00) in the box
         `bed ± (8, 5, 8)` → 5 `tile.bed.notSafe`. No line-of-sight or "is hostile now" filter seen (so a
         zombified piglin counts: inf). BDSRE
      9. Success → 0.
3. Result 0 → `BedEnterPosition` (actor data 133, Vec3) = the player's position before lying down; set
   `occupied_bit` on the **head** cell only (setBlock flags 2). BDSRE

Refusal and `respawnSet` messages (142cdabf0, `ServerPlayer::displayTextObjectMessage` 140758350):
`Text {type Json (10)}` with rawtext `[{"text":"§7"},{"translate":"tile.bed.<key>"}]`. BDSRE (PM/PNX send
`Translation`; mem). `tile.bed.notValid` is not reachable from this path.

### Night window (`skyDarken`, `Dimension` tick 14187f9c0)

`skyDarken = int((1 - b) * 11)`, `b = clamp(cos(2π·timeOfDay)·2 + 0.5, 0, 1) · (1 - rain·5/16) · (1 - thunder·5/16)`
(plus an extra `· (1 - w·6.1/16)` from a weather field not identified, and `· 0.6875` when a dimension flag at
+0x390 is set; both 0/off in a plain overworld: inf). `timeOfDay` = the Java formula (`d = frac(t/24000 - 0.25)`,
`d + ((1 - (cos(πd)+1)/2) - d)/3`; inf: the call is virtual, the shape matches). Sleep needs `skyDarken >= 4`:

| weather | ticks of day (time % 24000) |
|---|---|
| clear | 12541 – 23459 |
| rain | 12009 – 23991 |
| thunder | always |

(computed from the formula; matches the wiki's 12542/23459.) The same test wakes sleepers: see 2.

## 2. Lying down: what changes (success)

`startSleepInBed`, after the checks (BDSRE):
- Bounding box 0.2 × 0.2, eye height 0.2; motion zeroed.
- Position = `(head.x + 0.5, head.y + sleepOffset, head.z + 0.5)` (actor position, i.e. eye level). `sleepOffset`:
  `minecraft:bed` 0.90625, `minecraft:straw_bed` 0.59375, other 0.5 (use_sleep 142cd8730; floats at 14a656770).
  Without a bed block there: y + 0.0625.
- Yaw from the bed's `direction`: 0 → 0°, 1 → 90°, 2 → 180° (-180), 3 → -90° (tables 14a67e1f0/200), pitch 0.
- `setSleeping(true)`; actor data `PlayerFlags` (26) |= 0x02 (sleep bit); `PlayerBedPosition` (28, BlockPos) =
  head. The `SLEEPING` bit of `MetadataFlags2` is **not** set by BDS. Others see it via the normal dirty-data
  `SetActorData` plus the move. BDSRE; REC (PlayerFlags bit 1 + PlayerBedPosition).
- `ServerPlayerSleepManager::updateSleepingPlayerList` (14075ee50) → `LevelEventGeneric` 9801 (`SleepingPlayers`)
  `{sleepingPlayerCount, overworldPlayerCount = required count, ableToSleep = percentage <= 100}`. BDSRE; REC
  ("LevelEventGeneric sleepingPlayerCount 1").
- The client then sends `PlayerAction StartSleeping` (positions 0, face 0) ~513 ms after the click; nothing
  depends on it server-side (REC; mem: PM/PNX ignore it).
- Straw bed: `setsRespawn` false (from `tick_sleepChecks`: `block != straw_bed`), and on waking BDS removes both
  halves (setBlock air, flags 3) with break effects (stopSleepInBed 1402a39b0). BDSRE, purpose inf.

## 3. Waking

`ServerPlayer::stopSleepInBed(forceful, updateList)` (14074e1c0): if sleeping, sends **`Animate {action WakeUp (3),
runtime_entity_id}` to that player**, then `Player::stopSleepInBed` (1402a39b0):
- Box back to 0.6 × 1.8, eye height 1.62001.
- Clear `occupied_bit` on the bed (142cd8bd0, flags 2).
- Stand-up cell: `findStandUpPosition(bed, BedEnterPosition)`; none → bed + (0, 1, 0). New position
  `(cell.x + 0.5, cell.y + 1.62 + 0.1, cell.z + 0.5)` (eye level; feet 0.1 above the cell).
- `setSleeping(false)`; `PlayerFlags` &= ~0x02. `PlayerBedPosition` and `BedEnterPosition` are left as they were.
- Not forceful: resets the sleep timer one way, forceful the other (141e97b60 / 141e97980; inf).
- Bed block entity notified (type 27, 141f18050).

Triggers:
- Client leaves: `PlayerAction StopSleeping` (positions 0) → stop (mem: PM `stopSleep()`; handler not located).
  REC: after a server wake the client also sends StopSleeping 105 ms later, then `ClientCameraAimAssist`.
- Per tick, `Player::tick_sleepChecks` (1402a5a90): sleeping but bed block gone → stop(forceful); sleeping and
  `skyDarken < 4` (day) → stop(not forceful). Data sleep bit set but not sleeping → it calls startSleepInBed
  again (client prediction path). BDSRE
- Everyone asleep (below).

## 4. Skipping the night

- `SleepCounterSystem::tick` (148271580): per player, sleeping → counter +1 up to **100**; awake with counter > 0
  → +1 until 110, then 0 (wake animation). "Deep sleep" = counter >= 100 (141e97800). BDSRE
- Counted players (lambdas 140787850 / 1407879c0): not spectator (game type 6, or default 5 with world default 6),
  alive, not a simulated/other excluded kind (1414ed610, inf), in a dimension with a day cycle (overworld).
- `required = max(1, ceil(counted × playersSleepingPercentage / 100))`; game rule default 100 (mem); > 100 makes
  `ableToSleep` false and the skip never happens. BDSRE
- `updateSleepingPlayerList` raises a flag when `sleeping >= required && counted > 0`. Each `ServerLevel` tick
  (140747560) with the flag: count deep sleepers; `>= required` → clear the flag, **wake every sleeper**
  (`stopSleepInBed(false, false)`, so each gets `Animate WakeUp`), then publish "all slept":
  - time (14118fe70): if `doDaylightCycle`, `time = ((time + 24000) / 24000) * 24000` (next day's tick 0 =
    sunrise), then the usual `SetTime`. BDSRE (SetTime packet: inf, through the level's time-dirty path).
  - weather (`WeatherManager::stopWeather` lambda 1411a79e0): if `doWeatherCycle` and rain or lightning level
    ≠ 0: both levels 0, `rainTime = rand(168000) + 12000`, `lightningTime = rand(168000) + 12000`,
    `LevelEvent` 3003 (StopRaining) and 3004 (StopThunder). BDSRE
- So with one player: ~100 ticks (5 s) after lying down. REC: `Animate WakeUp` 4501 ms after `StartSleeping`,
  ~5 s after the click.

## 5. Container screen validity / reach

- `ChestContainerManagerModel::isValid(pickRange)` (142fbb3c0; shared by chest-like screens, slot vt+0x98):
  every container model valid; block screen: the block actor at the screen's pos must exist **and have the
  type the screen was opened for**; then `dist²(block centre (+0.5 each axis), player position) <= pickRange²`.
  Entity screen: entity exists, same test against `floor(entity pos) + 0.5`. BDSRE
- The per-tick `broadcastChanges` (142fbad70) calls it with `FLT_MAX`: validity only, no distance. BDSRE
- **Not found**: the call that passes a real pick range and what it then sends. Earlier reading of the old pdb
  build: `ServerPlayer::doDeleteContainerManager` → `ContainerClose {window_id, window_type of the screen,
  server true}` (containers-crafting.md A5; fields mem/gophertunnel).
- Reach constants in this exe for reference: `PlayerTransactionSubject::getMaxPickRange` (142a87b30) creative 12,
  else 6.7; `GameMode::getPickRange` (1430b2610) mouse 5.7, gamepad 5.6, other 12/6.7. Whichever the close path
  uses, 6.7 (survival) / 12 (creative) is the best guess (inf).
- Java 26.3 for contrast: `Container.stillValidBlockEntity` = block entity still the same object and
  `player.isWithinBlockInteractionRange(pos, 4.0)` (interaction range 4.5 + 4 = 8.5 to the block's box), checked
  every tick by the menu; not the old 8-block / 64 sq rule. Java
- Block gone: the block actor is removed with the block, so `isValid` fails (BDSRE); what the server then sends
  is the same unknown as above. `ChestBlockActor::onRemoved` (141f35670) only unpairs neighbours. BDSRE
- To settle: open a chest on the test BDS, `/tp` 5, 7, 9 blocks away and break it; record ContainerClose.

## 6. Chests that will not open

`ChestBlockActor::_canOpenThis(region)` (141f35070, vtable slot 30; shared by chest, trapped chest (same actor),
ender chest and barrel actors; shulker box has its own AABB). Fields set by the ctor 141f0f1a0:
`mAlwayCheckObstructionComponent = 1`, `mObstructionHeight = 0.5` (shulker ctor 144880da0: 0 and 0.5). BDSRE

1. If `mAlwayCheckObstructionComponent`: the block **directly above** (pos + y1) may carry
   `minecraft:chest_obstruction` = `ChestObstructionRule` (`always` / `never` / `shape`, registered in that order,
   read as 0/1/2), looked up on the block, then its type:
   - `always` → cannot open.
   - `never` → skip the shape test (cats still checked).
   - `shape`, or no component → shape test.
   Which vanilla blocks set it: not read (data-driven). BDSRE
2. Shape test: obstruction box `getObstructionAABB` (141f33590) = `[x+1/16, x+15/16] × [y+1, y+1.5] ×
   [z+1/16, z+15/16]`. Any block (other than the chest itself) whose **collision shape** strictly overlaps it →
   cannot open. So full blocks, slabs, stairs, glass etc. block; a block with no collision (torch, flower,
   pressure plate) does not; a collision box that only touches y+1 (e.g. a 0-height shape) does not. BDSRE
   Java uses `isRedstoneConductor` of the block above instead (glass would not block there). Java
3. `_detectEntityObstruction` (141f33660): any **cat (0x534b) or ocelot (0x5316) with the SITTING flag** (24)
   inside the obstruction box → cannot open. Java checks cats only (any cat, sitting or not). BDSRE / Java
4. Shulker box (vtable at 14a841d20): no component lookup; box = one full cell next to it in its `facing_direction`
   (`x/z ± 1` or `y ± 1`) with height 0.5 from that cell's bottom (`[cell, cell + (1, 0.5, 1)]` for up/sideways;
   inf for down); collision overlap → cannot open; no cat check (slot returns false). BDSRE
5. Double chest: whether `canOpen` also tests the paired half (Java: either half blocked → refused) was not
   located in this exe; containers-crafting.md A1 says yes (mem). `ChestBlock::use` not located here.
6. Barrel: its actor inherits `_canOpenThis` unchanged, but whether `BarrelBlock::use` calls it was not checked;
   vanilla barrels open under solid blocks (mem). Treat as never blocked.
7. Refused: nothing is sent (no message, no ContainerOpen); the client runs the same shared code and normally
   does not try (inf).

## sapling gaps (vs `crates/sapling-shard/src`)

- No sleeping at all (`blocks/doors.rs:4` TODO): needs 1-4.
- `containers.rs:8`: blocked chests open; no reach close (5 is still open in BDS too).
