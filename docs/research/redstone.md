# Bedrock redstone (first piece), from BDS (bdsre)

Read 2026-10-08 from `acacia/.testserver/bds-molang/bedrock_server.exe` (PE stamp 6ab54bee) with a private copy of
bdsre (`c|dis|vt|refs|callers`), layouts from LeviLamina `world/redstone/circuit/**`, `BlockRedstone*Component.h`,
`Dimension.h`, `BlockComponentDirectData.h`. No `name` labels added. Scope: plates, buttons, levers, redstone
block, torch, dust, powered blocks, lamp, doors, trapdoors, fence gates, hopper lock. Plate pressing itself is in
`pressure-plates.md`. **Every timing/order claim is [decompile]**: past bdsre readings of order and lag have been
wrong; a drill must confirm them (see Open checks).

Notation: `comp+0xNN` = field of a circuit component; type ids are `CircuitComponentType` (`Consumer 0x20000`,
`PoweredBlock 0x40000`, `Producer 0x80000`, `Transporter 0x100000`, `Capacitor 0x200000`, `TorchCapacitor
0x200003`). Facing: 0 down, 1 up, 2 north(-z), 3 south(+z), 4 west(-x), 5 east(+x) (offset table 14a712760);
opposite table 14a682118.

## 1. Scheduling: when the circuit runs

| VA | Suggested label | What |
|---|---|---|
| 140747560 | `ServerLevel::_subTick` | calls `Level::_subTick`, then forEachDimension(lambda_62 = `dim->tickRedstone()`) |
| 1407cc4c0 | `Level::_subTick` | early: forEachDimension(`Dimension::tick`, lambda_73); late: lambda_76 |
| 1407ffe80 | `_subTick` lambda_76 | `circuit.sceneGraph.update(region)` (144bce7c0); `circuit.mHasBeenEvaluated = 0` |
| 14187f890 | `Dimension::tickRedstone` | vt+0x88 of Dimension vtable 14a75bd80 |
| 141878869 | in `Dimension::Dimension` | `CIRCUIT_TICK_RATE (+0x208) = 2`, `mCircuitSystemTickRate (+0x20c) = 2` |
| 144bd6240 | `CircuitSystem::evaluate` | four passes, then `mHasBeenEvaluated = 1` |
| 144bd6510 | `CircuitSystem::updateBlocks(region, chunkOrigin)` | fires `BlockRedstoneUpdateEvent`s |
| 1409fcab9 | in `LevelChunk::tickImpl` (1409f98a0) | calls `updateBlocks` at the end of each chunk tick (after random + queued ticks) |
| 144bce7c0 | `CircuitSceneGraph::update` | pending adds/removes/updates; builds source lists (BFS) |
| 144bd6a10 | `CircuitSystem::createComponent` | **drops the component if `mLockGraph` is set** |
| 144bd6b90 | `CircuitSystem::setStrength(pos, s)` | `comp->setStrength(s)` if not removed |

Rules:
- R1 `tickRedstone` (14187f890): `if counter >= 2 and region: evaluate(); counter = 0`; then `counter += 1`. Counter
  starts at 2, so evaluation runs on the first tick and **every 2nd game tick** after. [decompile]
- R2 `evaluate` (144bd6240), over chunks in `mActiveComponentsPerChunk`, each marked active iff
  `region.areChunksFullyLoaded(chunk, 32)` (vt+0x1b8): pass 1 `cacheValues` on items with flag CacheValues(1);
  pass 2 `evaluate` on items with EvaluateAsProducer(4) whose group is Producer/Capacitor; pass 3 `evaluate` on
  items with EvaluateAsConsumer(8) whose group is not Producer/Capacitor; pass 4 `checkLock` on CheckLock(2).
  An `evaluate` returning true sets `comp.mNeedsUpdate` (+0x32). Then `mHasBeenEvaluated = 1`. Iteration order inside
  a pass is the chunk list then each chunk's component vector (insertion order). [decompile]
- R3 `updateBlocks` (144bd6510), called from each ticking chunk's `tickImpl` after its queued ticks: only if
  `mHasBeenEvaluated`; for each component of that chunk with `needsUpdate` and not removed: clear the flag; secondary-
  powered consumers are deferred to after the rest; `s = getStrength(); old = mOldStrength; setOldStrength(s)`;
  if `s != -1` and not (`mIsFirstTime && mIgnoreFirstUpdate`) fire block event 13 `BlockRedstoneUpdateEvent{pos,
  region, signal=s, previous=old, isFirstTime}` on the block at pos; `mIsFirstTime = 0`. [decompile]
- R4 lambda_76 resets `mHasBeenEvaluated = 0` near the end of every `Level::_subTick`, after `Dimension::tick`, and
  applies the scene graph's pending adds/removes/updates. Net effect: an evaluation at the end of tick E is applied
  to blocks during the chunk ticks of tick E+1, once. [decompile; that chunk ticking runs inside `Level::_subTick`
  before lambda_76 is inferred from `LevelChunkTickingSystem` being an ECS system, not traced]
- R5 Producers change strength immediately (button/lever/plate state write → `BlockRedstoneProducerComponent`
  (148a82ae0 / onEvent 148a82db0) → `setStrength` = `mNextStrength`); the value is taken into `mStrength` by
  `ProducerComponent::evaluate` (144bd8b60) at the next evaluation. First-time components also set `mStrength`
  directly (144bd8b70). [decompile]
- R6 Block writes done from a redstone update (lamp, hopper, torch) set `mLockGraph = 1` around `setBlock`, so
  `setupRedstoneComponent` called by that write does not create components (144bd6a10 returns null). [decompile]

Capture (`research/captures/bds-plates.txt`):
- Press → lamp on. Model: plate state set in tick T; evaluated at T (if T is an evaluation tick) or T+1; lamp written
  in the following tick → **1 or 2 ticks**. Chicken: plate 24397, lamp 24446 (1 tick) fits. Walking player: watcher
  plate 15587 / lamp 15745 (~3 ticks) does not, but the actor received the same two packets at 15943/15944, 1 ms
  apart, so the 158 ms gap is likely delivery batching to the watcher, not server ticks. Open check C1.
- Release → lamp off: plate's queued tick at R clears the state; evaluated at R or R+1; lamp gets signal 0 at R+1/R+2
  and queues a 4-tick tick (R12) → off at **R+5 or R+6**. Capture: 20588→20838 = 5 ticks, 26440→26745 = 6 ticks.
  Fits. The 25398 plate-off / 25446 re-press (chicken still on it) did not turn the lamp off: the lamp's queued tick
  rechecks the strength and finds it back on (R12).

## 2. Components

Layout (BaseCircuitComponent): `mSources` vector 0x08 (items 0x20 bytes: `comp* +0, dampening +8, pos +0xc, dir
+0x18, directlyPowered +0x19, data +0x1c`), `mIgnoreFirstUpdate 0x30, mIsFirstTime 0x31 (=1 at ctor), mNeedsUpdate
0x32, mPos 0x34, mStrength 0x4e, mOldStrength 0x50, mDirection 0x52 (=6 none), mAllowPowerUp 0x53 (=0),
mAllowPowerDown 0x54 (=1), mRemoved 0x55`. A source entry always names the **original producer** (`info.mPower`)
with the summed dampening of the path; `strength = producer.strength - dampening`.

| Type (vtable) | Created by | Key behaviour |
|---|---|---|
| ProducerComponent / DirectionalProducer (14a8523c0, ctor 144bd8570) | `BlockRedstoneProducerComponent::setupRedstoneComponent` 141a4a160 | `evaluate`: `mStrength = mNextStrength`, returns false; `allowConnection` (144bd8690): only toward faces in mask `+0x70` |
| ProducerComponent (14a852660) | 14257d0e0 (jukebox, tripwire hook, …; not in scope) | |
| TransporterComponent (14a851e00) | dust, `RedStoneWireBlock::setupRedstoneComponent` 144bc3300 | `cacheValues` 144bdbd20: `next = max(src - damp, 0)`; `evaluate` 144bdbd80: `strength = next`, changed? |
| ConsumerComponent (14a769c10) | 1425772b0 (+ per-block callback) | `evaluate` 144bd8260 (below) |
| PoweredBlockComponent (14a852580) | scene graph, 144bce7c0 (~line 1830) | `getStrength` 144bd8ac0: `max(src - damp)` over all sources; `evaluate` no-op |
| RedstoneTorchCapacitor (14a852830, ctor 144bd8d50) | `RedstoneTorchBlock::setupRedstoneComponent` 144bc5870 | §5 |

Consumer fields: `mSecondaryPowered 0x78, mPropagatePower 0x79, mPromotedToProducer 0x7a, mAcceptHalfPulse 0x7b,
mAcceptSameDirection 0x7c (=1 default)`. Defaults from 1425772b0: all 0 except acceptSameDirection 1.

- R7 `ConsumerComponent::evaluate` (144bd8260): `s = max over sources of (src.getStrength() - damp)`, or
  `15 - damp` when `src.isHalfPulse() && !mAcceptHalfPulse`; clamp ≥0; `mSecondaryPowered = item.data == 1` of the
  max item; returns `s != old || mIsFirstTime`. [decompile]
- R8 Flags per block (setup callbacks):

| Block | Setup | Consumer flags | Initial strength |
|---|---|---|---|
| door (both halves) | 148a23d80 | acceptHalfPulse 1 | 15 if open or other half's comp > 0 |
| trapdoor | 142574ee0 | allowPowerUp 1, acceptHalfPulse 1, ignoreFirstUpdate 1 | open_bit ? 15 : 0 |
| fence gate | 147154ed0 | allowPowerUp 1 | open_bit ? 15 : 0 |
| hopper | 142fd0fe0 | allowPowerUp 1 | 0 |
| redstone lamp | not found (Open C6) | | |

Producers via `BlockRedstoneProducerComponent` (data per block state; setup 141a4a160, on state change 148a82db0,
immediate signal 148a82ae0): DirectionalProducer with `direction = mStronglyPoweredFace`, `strength = mPower`,
`mAttachedAllowed (+0x64) = connectedFaces != 0`, face mask `+0x70 = connectedFaces`, `allowPowerUp/Down` copied.

| Block | Hard-coded component (VA) | power | strong face | faces | allowPowerUp |
|---|---|---|---|---|---|
| stone/wood/blackstone plate | 144bbbd50 | `redstone_signal != 0 ? 15 : 0` | 0 (down) | 0x3f | default |
| light/heavy weighted plate | 141f0b500 (not read; see pressure-plates.md for the signal) | signal | | | |
| button (all) | 141a322c0 | `button_pressed_bit ? 15 : 0` | `opposite(facing_direction)` = the attached block | 0x3f | 1 |
| lever | 142fe42a0 | `open_bit ? 15 : 0` | `opposite(LeverBlock::_getFacing)` (142fe3590) = attached block | 0x3f | 1 |
| redstone block | none found (Open C7) | 15 expected | | | |

## 3. Search (scene graph, 144bce7c0) and powered blocks

- R9 Neighbours without a component: if the block is not air and has `Block+0x90` (`BlockRedstoneComponent{
  mRedstoneConductor, mAllowsWireToStepDown}`, the `minecraft:redstone_conductivity` data) with
  `conductor || !allowsWireToStepDown`, a PoweredBlockComponent is created (`mAllowAsPowerSource 0x61 = 1`); if
  `!allowsWireToStepDown` instead it gets `allowPowerUp = 1, allowAsPowerSource = 0` (only cuts/doesn't cut dust).
  Blocks with no component pointer get nothing (never powered). Which vanilla blocks carry which values: Open C5.
  [decompile]
- R10 BFS from producers: for the component A being expanded and each of the 6 neighbours N (skipping the cell it came
  from): if `A.allowConnection(info, &dp)` and `N.addSource(info, &damp, &dp)` → enqueue N. `addSource` appends
  `(info.power, damp, dir, dp, data)` to N's sources via `trackPowerSource` (144bd76e0: stored damp =
  `max(damp,1) - 1`; an existing entry for the same producer keeps the smaller damp). [decompile]
- R11 PoweredBlock (Bedrock's "strong/weak" model). A block accepts power (`addSource` 144bd8930) only if
  `allowAsPowerSource && !allowPowerUp`; never from a Consumer or another PoweredBlock. `dp` ("directly powered") is:
  - from a Producer: `producer.mAttachedAllowed && step dir == producer.direction` (lever/button: attached block;
    plate: block below). Also sets `mPromotedToProducer (0x60)`.
  - from dust: true if dust is on top of it (step dir down) or the step continues the dust's own direction on the
    same level (dust points into it); else false. Never promotes.
  - from a capacitor (torch): `step dir == poweroutDirection` (= up, 1) → promoted; any other side of a torch: not a
    source at all.
  - `mPromotedToProducer` is reset to 0 at the start of each `addSource` call (last caller wins; Open C9).
- R12 Out of a PoweredBlock (`allowConnection` 144bd8900): to dust only if promoted ("strongly powered"); to any
  other non-block neighbour always. A Consumer reached through a block accepts only if `dp` was set on the way in
  (`Consumer::addSource` 144bd8320: nearest PoweredBlock and `!dp` → reject). So: strongly powered block (lever/button
  attached, plate below, torch above) powers dust, lamps, doors, torches; dust-powered ("weak") block powers
  lamps/doors/torches but not dust; a block merely adjacent to a lever powers nothing. [decompile]

## 4. Dust

| VA | Suggested label |
|---|---|
| 144bc23f0 | `RedStoneWireBlock::RedStoneWireBlock` (vt 14a84f9a0) |
| 144bc3300 | `setupRedstoneComponent`: creates the Transporter, then writes `redstone_signal = comp.strength` |
| 144bc2860 | onPlace: setup + game event 14cc8aa04 |
| 144bc2c20 | queued tick: if `!canSurvive` → drop resources, remove block |
| 144bc3120 | `_onRedstoneUpdate`: `setBlock(pos, wire[redstone_signal = event.signal], flags 3)`, no first-time skip |
| 144bdbc60 | `TransporterComponent::addSource` |
| 144bdb510 | `TransporterComponent::allowConnection` |
| 144bdb890 | "dust at pos connects toward dir" (wire, producer, capacitor, piston; repeater/observer by facing; not lamps) |
| 144bd6f40 | "dot or straight line" check |

- R13 Signal: each dust step `damp += 1` before tracking, so the first dust next to a producer/strong block is 15,
  then 14, 13… (`max(src - (n-1), 0)`); a consumer after dust n reads the same as dust n. [decompile]
- R14 Into dust (`addSource`): from a Producer always (dp = 1); from a PoweredBlock only if the path's `dp` is set
  (and per R12 only when promoted); from a torch (capacitor) on any side; from dust. [decompile]
- R15 Out of dust (`allowConnection`): never up; always down (block/component below); to a component that consumes
  from any direction (vt+0x38) always; to dust on the same level always; step **up** to dust at y+1 unless the cell
  above the lower dust holds a component that is not removed and has `allowPowerUp == 0` (a solid block above cuts
  the step); step **down** to dust at y-1 requires the cell above the lower dust to be free of such a block and the
  block under the upper dust to allow power down (`allowPowerDown`, step-down flag). Horizontally into a non-dust
  neighbour: only if the dust continues straight from where the power came and has no side connection (R13 checks
  144bdb890 on both perpendicular sides), or the dust is a dot (no connection on any of the 4 sides) → dp = 1.
  [decompile; the shape is decided per search path, not from a stored shape]
- R16 The client draws dust shape; the server only stores `redstone_signal`. [decompile]

## 5. Torch

| VA | Suggested label |
|---|---|
| 14879ec50 | `RedstoneTorchBlock::RedstoneTorchBlock` (vt 14a850600; `mOn` +0x380) |
| 144bc5870 | `setupRedstoneComponent` (vt+0x1c0): capacitor at pos, direction from `torch_facing_direction` |
| 144bc49d0 | onPlace: setup; if `!canSurvive` queue tick 1 |
| 144bc51b0 | `_onRedstoneUpdate`: swap lit/unlit block |
| 144bc6120 | `neighborChanged`: reignite reset, then TorchBlock's support check (queue tick 1) |
| 144bd9130 / 144bd95c0 | `RedstoneTorchCapacitor::cacheValues` / `evaluate` |
| 144bd8ef0 / 144bd9090 | `allowConnection` / `addSource` |
| 144bd9610 | `getStrength = State[0].on ? 15 : 0` |
| 144bd9120 | `isHalfPulse = State[0].halfFrame` |

- R17 Direction = towards the support: `torch_facing_direction` 0 unknown→0, 1 west→4, 2 east→5, 3 north→2,
  4 south→3, 5 top→0 (byte table `0x302050400`). New capacitor: `on = (block is lit torch)`, `changed = 1`.
  [decompile]
- R18 Input: when the search expands from the torch toward its support side (step dir == torch.direction) and finds a
  PoweredBlock/Producer/Consumer there, the torch records that component as its own source; it does not power its
  support. A standing torch never outputs down (`allowConnection`: `dir != 0 || torch.direction != 0`). [decompile]
- R19 Output: dust on any side (R14), the block above strongly (R11), consumers on any side except via
  `acceptSameDirection` rules (default accept). [decompile]
- R20 cacheValues (no clock chain): `input = max` over sources: producers by strength; PoweredBlock/Consumer sources
  by their own sources with `dp == 1` (`strength - damp`). If the max path starts at the torch itself (self-loop):
  `count += 1` (cap 33, frozen while `canReignite`); `count < 16` → toggle with `halfFrame = 1`; `count >= 16` →
  off, `halfFrame = (count == 16)`, `count > 32` → `canReignite = 1`. Otherwise `count = 0, canReignite = 0,
  next_on = (input == 0)`. `evaluate`: `State[0] = State[1]`, returns changed. [decompile]
- R21 Burnout = 16 consecutive self-powered evaluations (8 game-tick pairs = 32 ticks at R1 rate); a burnt torch
  stays off until `neighborChanged` with `canReignite` resets the count (needs count > 32 first). Not Java's "8
  toggles in 60 ticks". Clock chains of torches use `mNextOrder` (updateDependencies 144bd9630, not read). [decompile]
- R22 `_onRedstoneUpdate` (lock graph held): signal 0 → if `count > 15` play LevelSoundEvent 27 `Fizz` at pos (call
  shape inferred), then `setBlock(unlit_redstone_torch, same facing, flags 3)`; signal > 0 → `setBlock(redstone_torch)`.
  No write when already in that block. [decompile]

## 6. Users

- R23 Lamp (ctor 1487a6b30, vt 14a8501e0, `mIsLit` +0x378; redstone update 144bc43f0, tick 144bc3f40): signal 0 →
  queue tick on the lamp with delay **4**; signal > 0 → remove queued ticks for pos, and if unlit `setBlock(
  lit_redstone_lamp, flags 3)` (lock graph held). Tick: if the component exists, not removed, `getStrength() == 0`
  and lamp is lit → `setBlock(redstone_lamp, 3)`. No sound. [decompile]
- R24 Door (`_onRedstoneUpdate` 148a24370): skip if `isFirstTime`; `powered = signal > 0 || other half's component
  strength > 0`; if `isOpen != powered` → `setToggled` (148a25010): write lower half `open_bit`, flags 3, update upper,
  LevelSoundEvent `DoorOpen` 452 / `DoorClose` 453 at the event cell centre `(x+.5, y+.5, z+.5)`, extra data = new
  block runtime id; game event block_open 14cc8a92c / block_close 14cc8a924. Old-version branch: LevelEvent 1003.
  Wooden and iron identical (iron is not hand-openable elsewhere). [decompile]
- R25 Trapdoor (142575c80 → setOpen 142576830): skip first time; `powered = signal > 0`; if differs from `open_bit`
  → toggle, flags 3, `TrapdoorOpen` 454 / `TrapdoorClose` 455, same position/extra rule. [decompile]
- R26 Fence gate (147155fc0 → `_setOpen` 1471568c0): skip first time; if `open_bit != (signal > 0)` → set, flags 3,
  `FenceGateOpen` 456 / `FenceGateClose` 457. No player → no direction flip. [decompile]
- R27 Hopper (142fd18c0): no first-time skip; if `toggle_bit != (signal != 0)` → set `toggle_bit = signal != 0`,
  flags 3, lock graph held. No sound. [decompile]

## 7. Sources

- R28 Button: use (141a31b20 → press 141a32720) only if not pressed: set `button_pressed_bit=1` flags 3,
  `ButtonClickOn` 451 at centre (old version: `PowerOn` 73), game event 14cc8a93c, `setRedstoneSignal` now, queue
  tick **20 (stone/blackstone) or 30 (wooden, `mSensitive` +0x378)**. Tick (141a32080): `!canSurvive` → destroy;
  pressed: wooden → arrow check 141a34cb0 (not read), else unpress 141a352b0: bit 0, `ButtonClickOff` 450 (old: 74),
  game event 14cc8a944, signal now. [decompile]
- R29 Lever: use 142fe2920 → toggle 142fe3770: flip `open_bit`, flags 3, signal now, sound `wasOpen ^ 451`
  (on → 451 `ButtonClickOn`, off → 450 `ButtonClickOff`; old version `PowerOn/PowerOff` 73/74), game event
  activate/deactivate. Tick 142fe24f0: `!canSurvive` → destroy. [decompile]
- R30 Plates: see `pressure-plates.md`; power per R8 table (plain 15, weighted = signal). [decompile]

## Open checks (live drill)

- C1, C2: done, see the end: 2–3 ticks on, 6–7 off for `/setblock` changes.
- C3 Evaluation phase: two levers flipped in consecutive ticks should update lamps in the same tick sometimes.
- C4 Chunk ticking before lambda_76 (R4): if lamps never update, the order is reversed; capture says they do.
- C5: done, see the end. Not drilled: ice, TNT and pistons (the TNT went off and took the pistons).
- C6 Lamp's consumer component source (no setup callback found) and its flags (acceptHalfPulse?).
- C7 Redstone block's producer component (none found in its vtable): confirm it powers dust, lamp, adjacent blocks
  (weakly or strongly?) — e.g. redstone block next to a solid block next to a lamp.
- C8: done, see the end.
- C9 `mPromotedToProducer` overwrite order (R11): block strongly powered by lever and also touched by dust.
- C10: done, see the end. Not drilled: a torch clock seen through a lamp (half-pulse).
- C11 `redstone_signal` of dust after placement next to a lit source (setup writes current strength, R16).
- C12 Door: power one half only (top vs bottom), iron door; door sound position = which half.
- C13 Wooden button with arrow (141a34cb0 not read).

## Checked on BDS (2026-10-08, `bds_command_probe redstone`, research/captures/bds-redstone.txt)

- Dust from a block of redstone: every dust and the lamp at the end change in one batch, 2 ticks after the
  block; 15 dust reach a lamp, a 16th does not. Taking the block away: all dust to 0 two ticks later, the lamp
  out 4 ticks after the dust (R23).
- Lever on top of a block: the lamp beside the block lights 2 ticks after, goes out 6 after it is thrown back.
- A block of redstone beside a torch's block does not put the torch out (C7: it powers no block).
- Iron door beside a block of redstone: only the lower half's `open_bit` changes, the upper is sent again
  unchanged, `door.open`/`door.close` from the lower half's centre. Hopper: `toggle_bit`. `/setblock` of a
  lower door half also places its upper half, shut.
- A button put pressed by `/setblock` stays pressed: no queued tick springs it back.
- C1/C2 (`bds_command_probe redstone-phase`, research/captures/bds-redstone-phase.txt): a block of redstone
  put by `/setblock` 20 times, 10.5 ticks apart. Lamp, door and hopper all change in the same tick, 2 or 3
  ticks after the block, alternating with the phase (one stray 1); the lamp goes out 4 after. So a change is
  first evaluated in the tick after it (not R4's 1–2), and doors and hoppers lag lamps by nothing — the
  2-against-3 above was phase.
- C5 (`bds_command_probe conductors`, research/captures/bds-conductors.txt; 80 blocks, three rigs each:
  dust pointing into the block with a lamp behind, dust on the block with dust at its foot, dust under the
  block with dust a step up beside it):
  - Power passes through the full cubes, soul sand and mud, but not through glass (plain, stained, tinted),
    glowstone, sea lantern, leaves, observer, jukebox, beacon, scaffolding, copper grate, slime or a spawner,
    nor through anything lower or thinner (slabs, stairs, path, chests, tables, cauldron, walls, fences, panes).
    A double slab and a shulker box do pass it.
  - A block over dust cuts its step up exactly when it passes power, and also slime, honey, a spawner and a
    lectern, which pass none. So R9's reading is the wrong way round for those: glass cuts nothing.
  - Dust on a block steps down its side for every block that holds dust, but for glowstone and a top slab.
  - Dust does not stay on leaves, a bottom slab, path, chests, an enchanting table, cauldron, composter,
    carpet, anvil, shulker box, walls, fences, panes; it stays on soul sand, mud, glass, a top slab and
    upside-down stairs. Sapling follows this by name (ground.rs `NO_CENTRE`, `LOW_CENTRE`); plain stairs,
    ice, TNT and pistons were not judged.
- C8 (`bds_command_probe shapes`, research/captures/bds-redstone-shapes.txt; ten rigs, lamps around one dust):
  dust lights only the lamp straight on from a power block or the line it ends; neither lamp at a corner, only
  those past the ends; at a T the lamps past the two far ends; a dot on a lever-powered block all four; with
  one dust beside that dot, only the lamp opposite it. A step up to the side counts as a connection (the lamp
  straight on stays dark) unless stone over the dust cuts it; glass does not cut it. Sapling's rule by stored
  shape gives the same on all ten.
- C10 (`bds_command_probe burnout`, research/captures/bds-redstone-burnout.txt; a torch on the side of a block,
  a block over it, dust on the first):
  - The torch flips at every evaluation, 2 ticks apart, from the first one after it is put: the dust written at
    an evaluation shows the torch as it was before it, the torch block as it will be after.
  - Put lit, it flips 15 times and is then out for good, with no sound: R20's count reaches 16 with the torch
    already out. Relit, it flips 16 times counting the relight, and the last one, out, plays `fizz`.
  - A block put beside it 120 ticks after it burnt out relights it at the next evaluation; one taken away 18
    ticks after does nothing (R21: the count must first pass 32, 17 evaluations on).
  - The lamp past the dust stays lit through the flips. After the first burnout it went out 18 ticks after the
    dust lost its signal, not 4 as after the later ones; sapling puts it out after 4 every time.

## Repeaters (2026-10-09, `bds_command_probe repeater-facing` and `repeaters`, research/captures/bds-redstone-repeaters.txt)

No decompile: all of this is from the captures. An evaluation is 2 ticks.

- `minecraft:cardinal_direction` names the side a repeater takes power on; it gives 15 out of the opposite side,
  whatever came in (1 after 15 dust). The lit block is `powered_repeater` with the same states.
- In: dust on its input side, a power block or a block that dust or a lever powers directly behind it. Dust along
  its side does nothing to it and gets nothing from it.
- Out: the lamp in front, the block in front (strongly: dust on and beside that block gets 15, a lamp behind it
  lights), dust in front.
- Who sees a flip when. The repeater's block flips at an evaluation; lamps it powers, straight or through dust or
  a block, change in that same evaluation; the dust it powers shows it one evaluation later; another repeater
  held by it (below) acts on it one evaluation later. The torch drill shows the same of a torch. So dust,
  torches and repeaters are searched from the sources as they were, lamps and the like as they are after.
- Delay, steady input: the repeater flips `repeater_delay` evaluations after the evaluation its input dust
  changes at: 0, 2, 4, 6 ticks after the dust. Delay 0 flips with its dust.
- Delay, input flipping at every evaluation (the burnout torch): delay 0 follows it; delay 1, 2, 3 come out
  on for 2, 3, 4 evaluations and off for as many, first on 1, 2, 3 evaluations after the input, and stop within
  one such step of the input stopping. Fitted (sapling `RepeaterState::next`): a repeater flips `delay`
  evaluations after its input first differs from it, whatever the input does meanwhile; if the input differs
  from it again at the evaluation it flips, it flips back `delay + 1` evaluations later.
- Lock: a lit repeater pointing into the side of another holds it as it is, lit or not, whatever is behind it.
  When the holding one goes out, the held one takes up its input one evaluation later.
- Not drilled: a click's sound, a comparator as the one that holds, a pulse shorter than the delay from a
  steady state other than the 2-tick clock, a held repeater that was counting down.

## Comparators (2026-10-09, `bds_command_probe comparators`, research/captures/bds-redstone-comparators.txt)

No decompile: all of this is from the capture.

- `minecraft:cardinal_direction` names the side it takes power on, as a repeater's. Lit it is
  `powered_comparator` with `output_lit_bit=1`; `output_subtract_bit` is its mode.
- It gives out the strength behind it: 8 after 8 dust, and the dust in front shows 8, then 7. It changes at
  the evaluation its input dust changes at (no delay), and the dust in front an evaluation later, as a
  repeater's.
- Comparing, with 11 behind: 13 at a side puts it out, 9 at a side leaves 11. Subtracting: 13 gives 0, 9 gives 2.
  A side is dust ending at it. It answers a side in the evaluation the side's dust changes at.
- A chest behind it: 1 for the first full stack, then `floor(1 + 14 * stacks / 27)` (2 at 2, 3 at 4, ... 14 at
  26, 15 at 27), Java's formula. A chest behind a block of stone behind it reads the same. Taking the chest away
  puts it out.
- Other blocks (`bds_command_probe reads`, research/captures/bds-redstone-reads.txt): a hopper, furnace,
  dispenser, barrel, brewing stand, shulker box and trapped chest read by the same formula over their own
  slots (5, 3, 9, 27, 5, 27, 27); half a stack counts half, a sword as a full slot. A cake reads 14 less 2 a
  bite; a cauldron of water half its `fill_level`, rounded up; a composter its level, 0 to 8; a respawn anchor
  3, 7, 11, 15 by its charges; a beehive its `honey_level`; an end portal frame 15 with its eye.
  sapling's run of the drill reads the same of every one (research/captures/sapling-redstone-reads.txt was
  taken before it knew a dispenser as a container).
- Not drilled: equal strengths behind and at a side (sapling passes it, as Java), a repeater or power block at a
  side, jukebox, lectern, item frame, a cauldron of lava or powder snow, a chiseled bookshelf, a click's sound.

## Observers (2026-10-09, `bds_command_probe observer-facing` and `observers`, research/captures/bds-redstone-observers.txt)

No decompile: all of this is from the captures.

- `minecraft:facing_direction` names the side it watches (up and down too); it powers what is on the opposite
  side, `powered_bit=1` while it does.
- A change of the watched cell turns it on 2 ticks later and off 2 ticks after that. Seen to count as a change:
  a block put or taken, dust put, dust changing signal, a lamp lighting or going out, another observer turning
  on. Putting the observer itself does nothing.
- Two in a row: the second turns on 2 ticks after the first does, once, though the first changes twice.
- Lit, it powers a lamp behind it, the block behind it strongly (a lamp past that block lights, dust on and
  beside it gets 15), and dust behind it.
- Lamps answer at the evaluation after it turns on (1 or 2 ticks), dust one evaluation later than lamps, and
  again when it turns off: the lag a torch and a repeater show. So a block that changes its own power (not one
  put already powered by `/setblock`) is seen by lamps before dust.
- A lever and a stone button worked by hand show no such lag (`bds_command_probe byhand`,
  research/captures/bds-byhand.txt): the lamp beside the switch, both dust and the lamp past the dust
  change in one evaluation, a tick or two after the click. Off, the dust goes out at the next
  evaluation and both lamps four ticks after it, as lamps do; the button lets go 22 ticks after the click.
  Sapling gives them no lag either.
- Not drilled: a change while it is on, a piston moving it, a plate stepped on.

## Pistons (2026-10-09, `bds_command_probe piston-facing` and `pistons`, research/captures/bds-redstone-pistons.txt)

No decompile: all of this is from the captures.

- `facing_direction` 0 to 5 pushes down, up, south, north, east, west. The arm is
  `piston_arm_collision` (`sticky_piston_arm_collision`) with the same `facing_direction`, in the cell in front.
- Power: a power block on top of it, under it or beside it, or dust ending at its back. It answers at the
  evaluation a lamp would.
- Its block entity is `PistonArm`: `State` and `NewState` 0 drawn back, 1 going out, 2 out, 3 coming back;
  `Progress` and `LastProgress` 0 for 0 and 1, 1 for 2 and 3; `Sticky`; `isMovable` 1 only in state 0;
  `AttachedBlocks` and `BreakBlocks`, flat lists of the x, y, z of the cells it takes and breaks, farthest first
  (an empty list has tag 0). Sent when the piston is put, and at each change of state.
- Going out, tick by tick from the evaluation: (0) the entity in state 1 with its lists, and the burst of what
  it breaks; (1) each block it takes, farthest first, becomes `moving_block` one cell on and air where it was,
  then the arm is put, and what it breaks leaves its item; each moving block has a `MovingBlock` entity
  (`expanding` 1, `movingBlock` and `movingBlockExtra` as `{name, states, version}`, the second air,
  `pistonPosX/Y/Z`, and `movingEntity` holding the block entity of a chest); (2) `piston.out` at the piston's
  own corner (not its centre) and the entity in state 2; (3) each moving block becomes its block again, a chest
  with its entity.
- Coming back: (0) state 3; (1) the arm cell becomes air; (2) `piston.in` and state 0. A sticky one names the
  block two cells out in `AttachedBlocks`, and at (1) that block becomes a moving block (`expanding` 0) in the
  arm's cell and air where it was, a block again at (3).
- 12 stones go, 13 do not (nothing is sent at all), obsidian does not. A torch in front is broken. A chest goes,
  with what it holds. Dust on a pushed stone falls as an item a tick after the stone starts to move.
- The block changes of (1) and (3) come in `UpdateSubchunkBlocks`, those of (3) with flags 19; sapling sends
  the same changes as `UpdateBlock`.
- Not drilled: a mob or player in the way, which other blocks stay or break, a piston put
  beside power already on, power going while it moves, the place a real client's piston faces.

## Slime and honey (2026-10-10, `bds_command_probe slime`, research/captures/bds-slime.txt)

Built in `sapling-shard/src/blocks/pistons/plan.rs`.

- A slime or honey block that moves takes the blocks touching it, and those take what is before them: a
  stone on, under and beside slime all went with it. Slime takes slime, and what that holds.
- Slime does not take honey nor honey slime; neither takes glazed terracotta or obsidian, which stay where
  they are and do not stop the move. A chest is taken.
- All that goes counts to the 12: slime, a stone on it and 10 before it went, with 11 before it nothing did.
  Obsidian before a block slime holds stops the whole move.
- A sticky piston drawing back brings slime with what it holds (the stone on it and the one beyond it); a
  stone it brings does not bring slime that is beyond the stone.
- `AttachedBlocks` lists the cells farthest first in the way they go; cells equally far come in the order
  they were found: the block, what is before it, then what it holds up, down and south of it.
- Not drilled: the order north, west and east of a sticky block, what has no body beside slime (sapling
  leaves it), a sticky piston whose slime is held back (sapling brings nothing).

## Pistons and entities

From `bds_command_probe shove` (research/captures/bds-shove.txt): armour stands and pistons
pushing east, powered by a block of redstone above.

- A stand in the cell the arm goes into (its middle half a block from the piston's face) moved
  0.25 three ticks after the power and 0.5 a tick later: each tick it is put before the arm's
  front, which goes half a block a tick.
- A stand behind a stone the arm pushes ended with its box against the stone's new face, 0.75
  from where it stood.
- A sticky piston pushes it the same and leaves it there as it draws back.
- A stand on a piston that pushes up rose half a block a tick with the arm and fell back when
  the arm went.
- A stand with obsidian behind it moved 0.25, to the obsidian, and stayed in the arm's cell.
- The item of the drill fell beside the arm's way and shows nothing.

Sapling (`blocks/pistons/shove.rs`): every body in the way of the arm or of a pushed block is put
before it in the same two ticks, stopped by the blocks behind it. Players are not pushed.
