# Bedrock sculk sensors and shriekers (vibrations), from BDS (bdsre)

Read 2026-10-25 from `acacia/.testserver/bds-blocks/bedrock_server.exe` (PE stamp 6ab54bee, the same build as
`redstone.md`) with bdsre (`c|dis|refs|callers|str|rtti|find`) plus raw capstone/pefile reads of tables and vtables.
Layouts and virtual order from LeviLamina `world/events/gameevents/*`, `SculkSensor*`, `SculkShrieker*`,
`CalibratedSculkSensor*`, `VibrationListener*`, `IGameEventListener.h`, `BlockProperty.h`, `GameEventType.h`.
No `name` labels added. Scope: game-event dispatch to block listeners, sculk sensor, calibrated sensor, shrieker
trigger. Every rule is [decompile] unless marked otherwise; tick ordering was not checked on a live server.

Notation: `ev.type` = `GameEventConfig::GameEventType` (1-based, `GameEventType.h`); `ev.cat` = the event's category
byte. Facing 0 down, 1 up, 2 north(-z), 3 south(+z), 4 west(-x), 5 east(+x) (offset table 14a712760, opposite table
14a682118, as in `redstone.md`).

## 1. Functions and data

| VA | Suggested label | What |
|---|---|---|
| 1411e62a0 | GameEventRegistry static init | writes every `GameEvent{type, cat}` global (BSS 14cc89840..14cc8aa08) |
| 1411ad490 | game event name map init | `"block_place"` → global etc. |
| 14a7445a8 | `VibrationListenerUtils` frequency table | `int[65]`, index `type-1` |
| 1411b3910 | `VibrationListenerUtils::getGameEventFrequency` | `type-1 < 65 ? table[type-1] : 0` |
| 1411b3800 | `VibrationListenerUtils::ignoreVibrationIfSneaking` | |
| 1411b2390 | GameEventListenerRegistry dispatch | chunks in range → listeners → range check → `handleGameEvent` |
| 1411b2e50 | `GameEventListenerRegistry::registerListener` | per chunk, `LevelChunk+0x1518` |
| 14a740010 | `VibrationListener` vtable | dtor 1411cb460, handleGameEvent 1411b4140, getPositionSource 140a7ca60 (`this+0x58`), getRange 1411b5a60 (`this+0x74`), getDeliveryMode 140076010 (0 = immediate) |
| 1411b4140 | `VibrationListener::handleGameEvent` | filters, occlusion, `addCandidate` |
| 1411e5d40 | `_isVibrationOccluded` lambda | `block.legacy.properties & CanOccludeVibrations (1<<36)` |
| 1411b4b60 | `VibrationSelector::addCandidate` | |
| 1411b5940 | `VibrationListener::tick` | |
| 1411b4e40 | `VibrationListener::_tryAdvanceInFlightVibration` | |
| 1411b4f90 | `VibrationListener::_areAdjacentChunksTicking` | |
| 1411b5340 | `VibrationListener::_requestVibrationParticle` | |
| 1411b3150 | `PositionSource` → CompoundTag | |
| 14487a6e0 | `SculkSensorBlockActor::SculkSensorBlockActor` | range 8, owner SculkSensor, config vt 14a841680 |
| 141f25960 | `CalibratedSculkSensorBlockActor` ctor | range 16, config vt 14a7867a0 |
| 14487c610 | `SculkShriekerBlockActor` ctor | range 8, owner SculkShrieker, config vt 14a8416b0 |
| 14487ad20 | `SculkSensorBlockActor::tick` (shrieker shares it) | lazy listener registration, `listener.tick` |
| 14487aff0 | `SculkSensorVibrationConfig::onSignalReceive` | |
| 14487b090 | `SculkSensorVibrationConfig::shouldListen` | |
| 141f25860 | `CalibratedSculkSensorVibrationConfig::shouldListen` | |
| 1411b3930 | `VibrationListenerConfig::isValidVibration` (default) | `ev.cat & 1` |
| 14487b150 | `*VibrationConfig::onSerializableDataChanged` | marks the block actor changed |
| 1400a2880 | `canReceiveOnlyIfAdjacentChunksAreTicking` (sensor, shrieker) | returns true |
| 148b43cc0 | `SculkSensorBlock::SculkSensorBlock(name, id, type, activeTicks)` | `mActiveTicks` at +0x378; vtable 14a94c050 |
| 14856101f / 1485806ee | block registry calls | sensor: id 0x232, BA type 0x2d, **activeTicks 30**; calibrated: id 0x343, BA type 0x36, **activeTicks 10**, vtable 14a766b60 |
| 148b45960 | `SculkSensorBlock::setActivePhase(region, pos, source, strength, freq)` | |
| 148b44490 | `SculkSensorBlock::tick` (queued tick) | |
| 148b452e0 / 148b45830 | `SculkSensorBlock::isActive` / `isOnCooldown` | `sculk_sensor_phase == 1` / `== 2` |
| 148b44f50 | `SculkSensorBlock::setupRedstoneComponent` | |
| 141a38830 | `CalibratedSculkSensorBlock::setupRedstoneComponent` | |
| 141a38bf0 | `CalibratedSculkSensorBlock::getInputStrength` | |
| 148b45650 | `SculkSensorBlock::getComparatorSignal` | |
| 148b45410 / 148b45530 | `getLightEmission` / `getEmissiveBrightness` | active: 1 / 15, else 0 |
| 148b44cf0 | `SculkSensorBlock::onStandOn` | posts `sculk_touch` |
| 14487b900 | `SculkShriekerVibrationConfig::shouldListen` | |
| 14487c600 | `SculkShriekerVibrationConfig::isValidVibration` | `ev.type == 58` |
| 14487bf70 | `SculkShriekerVibrationConfig::onSignalReceive` | |
| 14487ba30 | `SculkShriekerBlockActor::tryGetPlayerInHierarchy` | |
| 14487c090 | `SculkShriekerBlockActor::tryShriek` | |
| 14487c8e0 | `SculkShriekerBlockActor::_canRespond` | reads `can_summon` (state global 14cd24220) |
| 14487b370 | `SculkShriekerBlockActorInternal::_shriek` | |
| 148b469d0 | `SculkShriekerBlock::tick` (queued tick) | `active = 0`, then `tryRespond` 14487ca80 |
| 148b470e0 | `SculkShriekerBlock::onStandOn` | |
| 148b46d20 | `SculkShriekerBlock::getInitialDefaultState` | `can_summon = 0` |

State globals: `sculk_sensor_phase` 14cd242e0, `minecraft:cardinal_direction` 14cd25130, shrieker `active` 14cd24340
(name inferred from use), `can_summon` 14cd24220 (inferred).

## 2. Game events: frequency and categories

Category bits (`ev.cat`, from 1411e62a0 and the tests in 1411b4140/1411b3800):
`1` = is a vibration (default `isValidVibration`), `2` = ignored when the source is sneaking, `4` = movement:
ignored when the mover stands in/on a dampening block, `8` = ignored when the affected block dampens,
`0x10` = not blocked by `VibrationDamperComponent`.

| type | name | freq | cat |
|---|---|---|---|
| 1 | block_activate | 10 | 1 |
| 2 | block_attach | 10 | 1 |
| 3 | block_change | 11 | 1 |
| 4 | block_close | 9 | 1 |
| 5 | block_deactivate | 9 | 1 |
| 6 | block_destroy | 12 | 9 |
| 7 | block_detach | 9 | 1 |
| 8 | block_open | 10 | 1 |
| 9 | block_place | 13 | 9 |
| 10 | bounce | 2 | 1 |
| 11 | container_close | 9 | 1 |
| 12 | container_open | 10 | 1 |
| 13 | dispense_fail | 10 | 1 |
| 14 | drink | 8 | 1 |
| 15 | eat | 8 | 1 |
| 16 | elytra_glide | 4 | 1 |
| 17 | entity_damage | 7 | 1 |
| 18 | entity_die | 15 | 1 |
| 19 | entity_dismount | 5 | 1 |
| 20 | entity_interact | 6 | 1 |
| 21 | entity_mount | 6 | 1 |
| 22 | entity_move (Java "step") | 1 | 7 |
| 23 | entity_place | 14 | 1 |
| 24 | entity_roar | 4 | 1 |
| 25 | entity_act | 4 | 1 |
| 26 | equip | 5 | 1 |
| 27 | explode | 15 | 1 |
| 28 | flap | 1 | 1 |
| 29 | fluid_pickup | 12 | 1 |
| 30 | fluid_place | 13 | 1 |
| 31 | hit_ground | 2 | 7 |
| 32 | item_interact_finish | 3 | 3 |
| 33 | item_interact_start | 0 | 0 |
| 34 | lightning_strike | 14 | 1 |
| 35 | multi_item_swap | 6 | 1 |
| 36 | note_block_play | 10 | 1 |
| 37 | piston_contract | 9 | 1 |
| 38 | piston_extend | 10 | 1 |
| 39 | prime_fuse | 10 | 1 |
| 40 | projectile_land | 2 | 9 |
| 41 | projectile_shoot | 3 | 3 |
| 42..56 | resonate_1..15 | 1..15 | 1 |
| 57 | sculk_touch | 1 | 0x11 |
| 58 | sculk_sensor_tendrils_clicking | 0 | 0 |
| 59 | shear | 6 | 1 |
| 60 | shriek | 0 | 0 |
| 61 | single_item_swap | 3 | 1 |
| 62 | splash | 2 | 1 |
| 63 | swim | 1 | 3 |
| 64 | teleport | 14 | 1 |
| 65 | unequip | 4 | 1 |
| 0 | invalid | 0 | 0 |

## 3. Rules

Dispatch (1411b2390, 1411b4140)

- S1 A posted game event at exact position `P` visits listeners registered in chunks around `P`. A listener at
  center `L` (block pos + 0.5, set in the ctors) is skipped when `d2 = (u64)|P - L|²` is `> 256` or `> range²`
  (sensor and shrieker 8, calibrated 16). Delivery mode 0: `handleGameEvent` is called at once.
- S2 `handleGameEvent` returns at once while the listener has a vibration in flight (S9). Otherwise, in order:
  1. (side note) a source player within 2.5 blocks (`dist² <= 6.25`) whose event is sneak-ignored fires a stat/
     achievement event (the "Sneak 100" hook); no effect on the sensor.
  2. `config.isValidVibration(ev)`: default `ev.cat & 1`; shrieker: `ev.type == 58` only.
  3. cat 8 and the context block has `CanDamperVibrations` (`BlockProperty` 1<<35) → ignore.
  4. `ignoreVibrationIfSneaking` (S3) → ignore.
  5. not cat 0x10 and the source actor has `VibrationDamperComponent` → ignore.
  6. cat 4 and a source actor: the block it stands on, then the block at `floor(feet pos)`; either with
     `CanDamperVibrations` → ignore.
  7. `config.shouldListen` (S6, S12, S16) false → ignore.
  8. occlusion (S4) → ignore.
  9. `selector.addCandidate(level.currentTick, ev, ctx, floor(L))` (S5), then mark the block actor changed.
- S3 Sneak filter (1411b3800): only cat 2 (`entity_move, hit_ground, item_interact_finish, projectile_shoot, swim`).
  Source = the context actor; a projectile is replaced by its owner. Ignored iff that actor's SNEAKING data flag
  (bit 1) is set.
- S4 Occlusion: `from = floor(P) + 0.5`, `to = floor(L) + 0.5`. For each of the 6 directions in order up, down,
  north, south, west, east (table 14a7445a0): clip from `from + 0.001 * dir` to `to`, hitting only blocks with
  `CanOccludeVibrations` (1<<36). Occluded only if **all 6** clips hit such a block; one clear line lets it through.
- S5 Selection (1411b4b60). `selector = {firstTick (-1 = none), candidate}`. If `firstTick != -1` and the event's
  tick differs, drop the event (only events of the first candidate's tick compete). `dist = (float)(u64)sqrt(
  intDistSq(floor(P), floor(L)))` — integer, truncated. If a candidate exists: when `|old.dist - dist| >= 0.005`,
  keep the closer (new replaces only if strictly closer); when equal, new replaces only if
  `freq(new) > freq(old)`. The candidate stores dist, `floor(P)`, source id, projectile owner id (if the source is a
  projectile), the event.

Listener tick (1411b5940, 1411b4e40), every block-actor tick

- S9 If nothing is in flight and `firstTick != -1` and `currentTick != firstTick`: move the candidate to in-flight,
  clear the selector, `inFlightTicks = trunc(dist)`, and send the particle (S21) with `timeToLive = dist / 20`.
- S10 If something is in flight: `old = inFlightTicks--`; if `old < 2`: when adjacent chunks are not all ticking
  (sensor and shrieker require it) keep waiting (counter keeps going down); else set `latestReceivedVibration =
  ev`, call `config.onSignalReceive(region, floor(P), ev, sourceActor, dist, range, projectileOwner)`, clear
  in-flight, mark changed. Net: with the candidate at tick T0 and taken at T0+1, the sensor reacts at
  `T0 + max(dist, 1)`.

Sculk sensor (config 14a841680; block 148b43cc0)

- S6 `shouldListen` (14487b090), `self` = sensor pos: reject `block_destroy`/`block_place` at `floor(P) == self`
  (its own placement/break); reject `sculk_touch` unless `floor(P) == self`; reject when phase is active (1) or
  cooldown (2). So only phase 0 (inactive) collects vibrations.
- S7 `onSignalReceive` (14487aff0): if not active: `strength = max(1, 15 - floor(dist / range * 15))`,
  `freq = frequency(ev)`, `setActivePhase(region, pos, source, strength, freq)`.
- S8 `setActivePhase` (148b45960): set `sculk_sensor_phase = 1` (setBlock flags 3); circuit component at pos
  `setStrength(strength)`; LevelSoundEvent 328 `SculkSensorPowerOn` ("power.on.sculk_sensor") at pos unless the
  liquid layer is water; cancel pending ticks for this block at pos; for each of the 6 neighbours that is an
  amethyst block, post `resonate_<freq>` there and play LevelSoundEvent 1067 (skip-able); post game event
  `sculk_sensor_tendrils_clicking` (58) at pos with the original source actor; schedule a block tick in
  `mActiveTicks` (sensor **30**, calibrated **10**).
- S11 Queued tick (148b44490): cancel pending ticks at pos. Phase 1 → phase 2 (flags 3), component
  `setStrength(0)`, schedule a tick in **10**. Phase 2 → LevelSoundEvent 329 `SculkSensorPowerOff`
  ("power.off.sculk_sensor") unless waterlogged, then phase 0 (flags 3). So active 30 (calibrated 10), cooldown 10
  for both.
- S13 `onStandOn` (148b44cf0): server side, entity not a warden (ActorType 0xb83) and not a projectile (type flag
  0x400000) → post `sculk_touch` at the sensor pos with that entity as source. Distance 0 → strength 15, freq 1,
  reacts the next tick. Not filtered by sneaking (cat 0x11 has no bit 2) nor by `VibrationDamperComponent`.

Redstone and comparator

- S14 Output (148b44f50): plain `ProducerComponent` (type 0x80000) created with direction 0 (down),
  `setStrength(0)`, `mAttachedAllowed (+0x64) = 1`, `mAllowPowerUp (+0x53) = 1`. Per `redstone.md` R11 this powers
  dust and consumers on every side and **strongly powers the block below** (direction down + attached). Strength =
  S7 value while active, 0 from the active→cooldown tick on.
- S15 Comparator (148b45650): `hasComparatorSignal`; value = `frequency(latestReceivedVibration)` while phase == 1,
  else 0 (so for 30 ticks, 10 for calibrated).

Calibrated sculk sensor (config 14a7867a0, block vtable 14a766b60)

- S12 `shouldListen` (141f25860): `input = getInputStrength(region, pos)`; listen only if `input == 0 ||
  frequency(ev) == input`; then the same checks as S6.
- S16 Facing: state `minecraft:cardinal_direction`, value → facing via table 14a6820f0: 0 south, 1 west, 2 north,
  3 east. Input side = the back: neighbour at `pos + offset(opposite(facing))`; `getInputStrength` (141a38bf0) asks
  the circuit system for a temporary consumer there and returns the strength of the component at that neighbour.
- S17 Output (141a38830): producer like S14 with face mask `+0x70 = 0x3f & ~(1 << opposite(facing))` (no output
  into the input side). Range 16, active 10 ticks, cooldown 10.

Sculk shrieker (config 14a8416b0)

- S18 It is a vibration listener (range 8) that accepts **only** `sculk_sensor_tendrils_clicking` (S8): any sensor
  activating within 8 blocks of it (S1 range, S2 filters and S4 occlusion apply; travel delay S10 applies). It does
  not hear ordinary vibrations. `shouldListen` (14487b900): shrieker `active == 0` and the event source resolves to a
  player (`tryGetPlayerInHierarchy`: the player itself, the controlling rider's player, or a projectile's player
  owner).
- S19 `onSignalReceive` (14487bf70) and `onStandOn` (148b470e0, any non-projectile entity resolving to a player,
  server side) both call `tryShriek(region, pos, player)` (14487c090): no-op if `active`; the player must have
  `WardenSpawnTrackerComponent`. If `_canRespond` (can_summon and world rules) is false → shriek with no warning.
  Else warning path (warden summoning, out of scope): players within 16 blocks plus the provoker; fails (no
  shriek) if 142255db0 says so (likely "warden nearby") or the chosen tracker has `threatIncreaseCooldown > 0`;
  else `threat = min(threat + 1, 4)`, cooldown 200, decrease timer 12000, copied to every nearby player; queued
  response = threat; then shriek.
- S20 `_shriek` (14487b370): `active = 1` (setBlock flags 2); schedule a block tick in **90**; LevelEventGeneric
  2035 `ParticlesSculkShriek` with tag `{originX, originY, originZ}` (ints); LevelSoundEvent 366
  `SculkShriekerShriek` ("block.sculk_shrieker.shriek") unless waterlogged; post game event `shriek` (60, not a
  vibration). Queued tick (148b469d0): `active = 0` (flags 3), then `tryRespond` (warden / warning sound).

Particle (1411b5340)

- S21 `LevelEventGenericPacket` (id 124), event 2027 `ParticlesVibrationSignal`, tag:
  `origin: {type:"vec3", x, y, z}` (source block + 0.5), `target:` the listener `PositionSource` (block listeners:
  `{type:"vec3", x, y, z}` = listener center; actor listeners: `{type:"actor", uniqueID: i64, attachPos: i32}`),
  `speed: 20.0f`, `timeToLive: dist / 20` (float, seconds). Sent through the dimension to players near the source
  block pos. No separate "clicking" sound: Bedrock uses the power on/off sounds (S8, S11).

## Unverified

- Names of state globals 14cd24340 (`active`) and 14cd24220 (`can_summon`): inferred from use, string not traced.
- Which blocks carry `CanOccludeVibrations` / `CanDamperVibrations` (Java: wool occludes; wool and carpet dampen)
  and which entities get `VibrationDamperComponent` (entity JSON).
- Arg order of 14257d0e0 (the `0` read as direction down); the strong-power-below claim rests on that and
  `redstone.md` R11.
- `getInputStrength` returns -1 on one path (component missing); effect on S12 not checked.
- Order of block-actor ticks vs. events within a tick (T0+1 assumption in S10); chunk ticking requirement timing.
- 142255db0 meaning in `tryShriek`; `_canRespond` beyond the `can_summon` read (difficulty, gamerule).
- Place sounds (`SculkSensorPlace` 381, `SculkShriekerPlace` 382) assumed client/place-event driven, not traced.

## Left out

- Warden summoning and warnings: `SculkShriekerBlockActor::tryRespond` 14487ca80, queued response, warden spawn.
- Entity listeners (warden 148f8bfd0 owner 2 range 16, allay owner 3), `VibrationListenerSystem` 1484556e0.
- Amethyst resonance details, client particles (`animateTick` 148b44ff0), save/load tags
  (`VibrationListener`, `QueuedResponseThreatLevel`; 14487a960/14487ac80, 14487d3f0/14487d530).

## Catalyst

Read 2026-10-28 (parity/sculk), same build, with bdsre `dis`/`c`. The catalyst runs the same `SculkSpreader` as
`SculkPatchFeature` (worldgen-bds-caves.md, "Sculk"): one function, `updateCursors` 148b48870, branching on the
spreader's `worldgen` byte and on whether a level is passed. sapling runs the generator's port
(`sapling-worldgen/src/feature/sculk/`) over the live world (`sapling-shard/src/blocks/sculk/catalyst.rs`).

| VA | Label | What |
|---|---|---|
| 148802230 | `SculkCatalystBlockActor` ctor | listener centre = pos + 0.5; spreader at +0x170 via 148b47480 |
| 148b47480 | `SculkSpreader` level ctor | `{worldgen 0, growth cost 10, no-growth radius 4, charge decay 10, additional decay 5}` (14a94acc0) |
| 148b47600 / 148b47d30 | replaceable sets | `sculk_replaceable` = the worldgen set less 6 (deepslate bricks, tiles, both cracked, cobbled, polished) |
| 144879f70 | `SculkCatalystBlockActor::handleGameEvent` | below (C1) |
| 140214560 | `getRange` | returns 10 |
| 144879bc0 | `SculkCatalystBlockActor::tick` | server only: `updateCursors(region, level, pos, level random, spreadVeins 1)` every tick |
| 148b435b0 | `SculkCatalystBlock::bloom` | below (C3) |

Spreader layout: +0 worldgen, +4 growth spawn cost, +8 no-growth radius, +0xc charge decay rate, +0x10 additional
decay rate, +0x18 cursors (`{pos, charge +0xc, updateDelay +0x10, decayDelay +0x14, facings +0x18}`, 0x1c bytes),
+0x30 replaceable set.

- C1 `handleGameEvent`: only `entity_die` (type 18) from an actor with category bit 2 (mobs, players) that has no
  `WasHandledBySculkCatalystFlagComponent`. Reach is S1's with range 10. If it has an enabled
  `ExperienceRewardComponent` (+0x30 == 1) and is not a player under `keepInventory` (game rule 14): it sets
  the actor's last-hurt-by-player time (+0x114) to 400 around `getExperienceReward` (140d2a3d0), so any death pays
  as a player's kill; a reward > 0 becomes cursors (`addCursors`: parts of at most 1000, kept while under 32
  cursors) at the lower of (block pos + 1 up) and floor(attach pos 0); then the component's flag is cleared (no
  experience drops). In every case it blooms (C3) and flags the actor, so one catalyst takes a death.
- C2 `updateCursors` with a level (vs. worldgen): per cursor, one already spent pops; a cursor whose chunk does not
  tick keeps its state; no sqrt(224) leash. After the update a spent cursor pops (`LevelEventGeneric` 2038
  `{x, y, z}` ints). Cursors left on one cell merge into the first while the sum is at most 1000 (charge added,
  update delay the lower), else stay separate (the cell's shown cursor becomes the weaker). Then per cell 2037
  `{x, y, z: int, charge: short (sum), facing: short (the shown cursor's facings)}`. Sounds at the cursor's cell
  (float, no +0.5, extra data -1): 379 `block.sculk.spread` when veins spread (148b48d3f) or a vein turned a block
  to sculk (148b4271d); 381 `block.sculk_sensor.place` / 382 `block.sculk_shrieker.place` for a growth (148b41b85,
  148b41d8d). A growth shrieker's `can_summon` is the spreader's `worldgen` byte (148b41ca1): 0 from a catalyst.
- C3 `bloom` (148b435b0): `bloom = 1` (setBlock flags 3), a block tick in 8, `LevelEventGeneric` 2036
  `{originX, originY, originZ}` ints. The queued tick puts `bloom = 0` (inferred from Java; the lambda not read).
- C4 Sculk `attemptUseCharge` (148b41810): the growth test is `r² > d²` → no growth (d² float, r the no-growth
  radius), but the decay penalty is 1 whenever `r² >= d²`. The worldgen port had `d² < 1` for both; it now follows
  BDS, which changes worldgen only for a cursor at d² = 1 with charge >= 89 (not re-run against the oracle).

sapling: the nearest catalyst in range takes a death (BDS: the first in dispatch order); a mob's reward is its
`Slain` roll, else rolled at death; a player's is what it would drop. Not done: cursors are not saved with the
catalyst (BDS saves them in the block actor), live blocks the generator never places read as bedrock, the
"It Spreads" advancement hook (142d3c8f0) is left out.

## Not drilled

- Everything in "Catalyst" (C1-C4): decompiled only. In particular range 10 rather than Java's 8, the cursor start
  cell, the bloom end tick, the merge and event order, and that a baby or zero-reward death still blooms.
