# BDS actor move sync: MoveActorDelta and SetActorMotion

BDS `bds-molang` build (cache stamp 6ab54bee), read with bdsre on 2026-10-26. VAs are this build's.
Sure = read in pseudo-C and confirmed in disassembly or by the capture fit below. Likely = read but not
cross-checked. Unknown = not pinned.

## Verdict on the hypothesis

It is not dead reckoning. Two separate rules apply:
1. A **global change test** runs once per actor per tick, against the last delta sent to anyone.
2. A **per-viewer rate limit** comes from the entity's `minecraft:conditional_bandwidth_optimization` and
   the viewer's distance. `ticks` is that viewer's allowed drop count, not the ticks since its last packet.
   A viewer gets at most one delta per `ticks` ticks. `ticks: 0` means "no optimisation for this send".

## Call chain (sure)

- `ActorMotionSyncSystem::tick` 145e2d9f0, for each actor: calls actor vtable slot 0x300. In vtable
  14a656a50 (stored by 14028dfd0) that slot is 140d50930, `Actor::sendMotionPacketIfNeeded` (the header
  order places it 4 slots after `Actor::canAddPassenger` 140d503b0).
- 140d50930 builds a `MoveActorDeltaData` (142d9aa80) and tests it with 142d9af10. Then it calls the
  delta sender 1413e6960 (packet vtable 14a7d93f0). After that it runs the SetActorMotion rule.
- 1413e6960 walks every player via the dimension (vt+0x50), with the per-viewer lambda 1414407f0. The
  lambda calls the decision function 1413e7730 for each viewer. Viewers that pass get the shared packet,
  with `ticks` set per viewer (entry +0xb8).

## MoveActorDelta rule

```
// per actor per tick (140d50930); sync = actor+0x158, last = sync+0x10 (last delta sent to ANYONE)
if sync.byte9 == 0: return                                   // unknown flag, likely "initialised"
cur = build(actor)        // 142d9aa80: runtime id, flags{onGround=1, teleported=2}, pos (StateVector pos),
                          // rot bytes: [0x18]=pitch, [0x19]=yaw, [0x1a]=head yaw (0 unless actor+0x210 & 2)
                          // byte = int(wrap(deg+180)-180)/360*256
force = has ForceSendMotionPacketComponent
if force or changed(cur, last, motion=StateVector.posDelta):   // 142d9af10
    send_delta(force)                                          // 1413e6960

changed(cur,last,motion) =                                     // 142d9af10
    cur.runtimeId != last.runtimeId
 or |cur.pos-last.pos|^2 > 0.0001                              // f32 @142d9af58: 1 cm
 or sum over pitch,yaw,headYaw of |cur.byte*360/256 - last.byte*360/256| > 4.0   // f32 @142d9b00a
 or (|cur.pos-last.pos|^2 > 1e-6 and |motion|^2 < 1e-6)        // f32 @142d9b038: settle at rest
// on_ground is NOT compared here (sure: only the qword at +0 is compared)

send_delta(force):                                             // 1413e6960
  header bits: x 1,y 2,z 4,pitch 8,yaw 0x10,headYaw 0x20 = (field != last field) (float/byte compare);
               onGround 0x40, teleported 0x80 (|=2 when force), 0x100 = arg3, 0x200 = arg5 (force comp byte)
  for each player P in the dimension:                           // lambda 1414407f0
    if actor has HasTeleportedFlagComponent: ticks = 0, no visibility test, no rate limit
    else:
      if !dim.vt[0x118](P, actor): skip                         // visibility/range test, unknown name
      d = decide(P); if d is none: skip                         // 1413e7730
      ticks = d
    send to P with ticks; viewerLastTick[P] = level tick        // map at sync+0x68 (1414428e0)
  if at least one player was sent to: last = cur; sync+0x60 = ForceSendAll counter
  // otherwise last stays, so the change is still pending next tick

decide(P):                                                      // 1413e7730
  drop = 0
  if flighting service present and its vt+0xc0 flag is off: return 0   // service absent on BDS: go on
  cbo = ConditionalBandwidthOptimizationComponent (or none)
  if cbo and on_ground(last) == on_ground(now)                  // jump or landing forces 0
     and ServerSideForceSendAllPlayersSpatialActorNetworkDataComponent value == sync+0x60:
      (maxDist, maxTicks, hints) = cbo.eval()                   // 141b95cb0: first conditional_values
                                                                // entry whose filter passes, else default_values;
                                                                // cached per level tick
      if !flight_flag(): maxTicks *= 0.75                       // 1413e7460; f32 @1413e7e85; true when no service
      dist = |P.pos - actor.pos|  (f32 sqrt; P.pos is the player's eye position)
      drop = trunc_u64(f32(maxTicks) * (dist <= maxDist ? dist/maxDist : 1.0))
  lastP = viewerLastTick[P] or 0
  if levelTick < lastP + drop: return none                      // rate-limited, viewer skipped
  return drop
```

Constants and defaults:
- `default_values` defaults: max_optimized_distance 80.0, max_dropped_ticks 10, use_motion_prediction_hints
  false. These are set in the parser 141bc1ed0 (80.0 @141bc1ef1/141bc1f38, 10 @141bc1ef8/141bc1f5c). Sure:
  every mob in the capture that declares `{}` fits 80/10.
- Per kind (vanilla packs, newest wins). `{}` covers 85 kinds, including drowned, polar_bear, slime,
  magma_cube, blaze, witch, ravager, hoglin, zoglin, piglin_brute and endermite. Enderman, xp_orb,
  shulker, eye_of_ender_signal, fireworks_rocket and lightning_bolt: 80/10, hints on. Arrows and most
  projectiles: 80/7, hints on (snowball 100/7). Potions, tnt, xp_bottle: 80/5. Minecarts and boats: 60/20,
  with a conditional entry `is_moving` → 0/0 (no optimisation while moving).
  These kinds have no component, so every send gets `ticks: 0` (every tick, to every viewer): axolotl,
  creaking, ender_dragon, frog, glow_squid, goat, nautilus, npc, ominous_item_spawner, tadpole, warden,
  zombie_nautilus. Dump: `py -3 <scratchpad>/cbo.py` (reuses sapling's mob-loot.py brarchive reader).
- Initial `last` flags for a fresh actor: unknown. The first delta after spawn went out with `ticks: 0` in
  every case in the capture, which fits `last.onGround` starting at 1 for a mob spawned in the air. Unverified.

## SetActorMotion rule (140d50930, after the delta; sure unless noted)

```
if !flight_flag(): return                                       // 1413e7460, true on BDS
if VehicleInputIntentComponent byte+3 set: return
m = StateVector.posDelta; d2 = |m - actor.lastSentMotion(+0x160)|^2
if d2 > 0.04 (f32 @140d50e08/140d50e6d, i.e. 0.2 b/tick) or (d2 > 0 and m == (0,0,0) exactly):
    lastSentMotion = m
    if hints and no forced delta this tick and FeatureToggle id 0x27 enabled:   // likely
        MotionPredictionHintsPacket(runtime id, m, onGround)    // vtable 14a7d92d0
    else:
        SetActorMotionPacket(runtime id, m, tick)               // vtable 14a7b3bc0
    sent through dim.vt[0x20] to every player that sees the actor (no per-viewer rate limit)
```
The FeatureToggles lookup (id 0x27 in the toggle list; 1 when absent) is a likely reading but not pinned.
The capture cannot test the hints branch: slimes and magma cubes have hints off. The capture tool may also
not decode MotionPredictionHints.

## Fit against bds-kinds2.txt

`<scratchpad>/fitw.py watcher` replays the watcher's MoveEntityDelta stream. It tracks each entity's
position from AddEntity plus deltas and grid-fits the watcher's position.
- All 364 watcher deltas with `ticks > 0` match `trunc(10*min(d/80,1))` (enderman/small_fireball with their
  own settings) at watcher eye (6.0, -16.88, 0.75), i.e. feet y -18.5 + 1.62. 0 mismatches.
- Of the 222 with `ticks: 0`, 213 are teleports (`force_move`) or on_ground changes. The other 9 are each
  entity's first delta after spawn (see "Initial flags" above).

The captured cases:
- Drowned walking from rest: the watcher's ticks go 1 → 2 as the drowned's distance passes 16 (10*d/80 ≥ 2),
  not because of its speed. It changes every tick, so the watcher gets one every `drop` ticks exactly. The
  near "actor" viewer (d < 8) gets every tick with 0.
- Polar bear: head yaw (`rot_z`) turns by more than 4° per tick. Every watcher delta in the file carries
  `rot_z`, so none is empty. d≈68 → 8, and the watcher gets one at most every 8 ticks. Gaps are longer when
  no change is pending at the moment it is allowed.
- Enderman standing: d≈88 > 80 → its own maxTicks 10. Rotation-only, because the 4° rule fires on look.
  Its spawn teleport went out with ticks 0.
- Slime hop: the jump flips on_ground → 0. Mid-air, d≈68 → 8, the first send allowed after the jump. The
  landing flips on_ground → 0. SetEntityMotion goes whenever |Δmotion| > 0.2 or motion becomes exactly 0.

Gotcha for viewers: a delta's fields are compared against the last packet sent to anyone. A rate-limited
viewer can therefore miss a field change that only the near viewer received (e.g. `x: None` while x moved).
BDS accepts the drift until the next change of that field.

## Unknown

- What `dim.vt[0x118]` tests (visibility/range) and what `sync.byte9` means.
- The flighting flags behind 1413e7460/1413e7730 (IFlightReader service at 14cc33370; absent on BDS here).
- FeatureToggle 0x27's name.
- Whether `last.onGround` starts set on spawn.
