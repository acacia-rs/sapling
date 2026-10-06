# Bedrock dropped items (`minecraft:item`, ItemActor), from BDS (bdsre)

Read 2026-10-06 from `acacia/.testserver/bds-molang/bedrock_server.exe` (PE stamp 6ab54bee; the default `bds/`
path is gone, so every command ran with `BDS_EXE` pointing here). Pseudo-C from `py -3 bdsre.py c <va>`, the rest
from `dis`/`win`/`vt` and two small read-only byte scans of `.text` (constant stores to `[reg+0x44c]`, and virtual
calls with a constant stack int, to find every `spawnItem` caller). Field names from LeviLamina `ItemActor.h`,
offsets confirmed by use. No `name` labels were added; suggested labels are below. Nothing tested on a live server,
and there is no recording of a moving item.

| VA | Suggested label | What |
|---|---|---|
| 14a888000 | `ItemActor::vftable` (+0) | slot 114 = read, 115 = add save data (`refs` of the save functions land on +0x390/+0x398) |
| 145988d50 | `ItemActor::ItemActor` | defaults: size, lifetime, random yaw and motion |
| 145988800 | `ItemActor::_addComponents` | PushableByBlock, PushableByEntity, ConditionalBandwidthOptimization |
| 1459892e0 | `ItemActor::postNormalTick` | the whole movement, merge and despawn rule |
| 145e2e2f0 | `ActorPostAiStepTickSystem::fn?` | calls 1459892e0 for entities with `ItemActorFlagComponent` |
| 14598a8a0 | `ItemActor::playerTouch` | vtable slot 18 |
| 145989240 | `ItemActor::tryCreateAddActorPacket` | slot 3; allocates the 0xe0-byte AddItemActorPacket |
| 14598a980 / 14598ab90 | `ItemActor::{add,read}AdditionalSaveData` | string `ItemActor has no Item.` |
| 1415ebf80 -> 14118ce10 | `Spawner::spawnItem` | `BedrockSpawner` vtable 14a753720 slot 9 |
| 141a241c0 | `BlockType::popResource` | block drops |
| 140d4dad0 | `Actor::spawnAtLocation` | mob loot |
| 140d5e640 -> 140d5dda0 | `Actor::drop` (slot 95) -> `Actor::_drop?` | thrown items (`randomly` = 3rd arg) |
| 140d5e660 | `Actor::dropTowards?` | throwTime 40, aims at a target; inferred, not read |
| 140d281f0 | `Actor::isInFallingLiquid?` | `liquid_depth > 7` at floor(pos) or the cell below |
| 142ed6730 | `Block::isFallingLiquid?` | state 14cd25490 (`liquid_depth`, shared with `isLiquidSource` 142ed68c0) > 7 |
| 140d51370 | `Actor::pushOutOfBlocks?` | nearest open face, overwrites the velocity |
| 141cd2c70 | `getHeightOffset` (OffsetsComponent) | |
| 140d50930 | `Actor::sendMotionPacketIfNeeded?` (slot 76) | per-actor sync, from `ActorMotionSyncSystem::tick` 145e2d9f0 |
| 1413e6960 | `ActorMovementSync::sendMove?` | MoveActorDelta (server) / MoveActorAbsolute |
| 142d9aa80 / 142d9af10 | `MoveActorAbsoluteData::{ctor(Actor), isDifferent?}` | position change thresholds |
| 1413e7600 | `useMotionPredictionHints?` | reads ConditionalBandwidthOptimizationComponent |
| 1402b4b10 | `Player::take` | called by playerTouch |

`ItemActor` fields (Actor ends at 0x3b0): `mItem` 0x3b0 (count byte at 0x3d2), `mAge` 0x448, `mPickupDelay` 0x44c,
`mThrowTime` 0x450, `mBobOffs` 0x454, `mHealth` 0x458, `mLifeTime` 0x45c. StateVector (Actor+0x218): pos 0..2,
prev pos 3..5, velocity (`mPosDelta`) 6..8.

## Constants (verified on BDS)

| | value | evidence |
|---|---|---|
| actor type | 0x40 (64) `minecraft:item` | 14118ce94 `create(.., 0x40, ..)`; merge search filter |
| box | 0.25 x 0.25; height offset 0.125 (half the height) | ctor `setSize(0.25, 0.25)`, `setHeightOffset(bbH * 0.5)` |
| health | 5 | ctor |
| lifetime | 6000 ticks (`age >= mLifeTime` removes; items whose Item flag `+0x152 & 0x40` is clear reset age to 0 instead) | ctor, 145989ecb |
| spawn motion (every item) | `(r*0.2-0.1, 0.2, r*0.2-0.1)`, yaw `r*360` | ctor |
| gravity | `vy -= 0.04`, only while actor flag 49 (HAS_GRAVITY) is set | 145989c6d |
| drag | `vy *= 0.98`; `vx, vz *= f`, f = 0.98 in air, `blockFriction * 0.98` on ground (air below: 0.588) | 145989e9b |
| bounce | none | no `-0.5` on vy anywhere in postNormalTick |
| water (head in water, not falling water) | `vy = vy + 0.0005 + (0.06 - (vy + 0.0005)) * 0.01`; `vx, vz *= 0.99`; gravity off | 145989503 else-branch |
| lava (box touches Lava, not falling lava) | same vy rule; `vx, vz *= 0.95`; gravity off; Fizz sound (LevelSoundEvent 27) unless fire-resistant | |
| resting skip | on ground, `|v|^2 <= 1e-4`, no WasInWater: physics only when `age % 4 == 0` | 145989c60 |
| push out of blocks | velocity := direction * (0.25, or 0.1 for blocks with `BlockType+0x130 & 1`) | 140d51370, tables 14a72ca40, 14a682180 |
| merge | every 25 ticks (`age % 25 == 0`, age > 0), box grown 0.5 | |
| pickup | needs `mPickupDelay == 0` **and** `mThrowTime == 0`, for every player (no owner test) | 14598a912 |
| pickup delay "infinite" | `mPickupDelay == 0xffff`: never decremented, never merged | |

| who spawns | position | motion | mThrowTime | mPickupDelay | earliest pickup |
|---|---|---|---|---|---|
| block (`popResource`) | `blockPos + (r*0.7+0.15)` per axis (x, y, z drawn in that order) | ctor default | 10 | 0 | 10 ticks |
| mob loot (`spawnAtLocation`) | actor pos + yOffs on y | ctor default | 10 | 10 | 10 ticks |
| player throw (`drop`, randomly=false) | `origin - (0, 0.5, 0)`, see below | see below | 40 | 10 | 40 ticks |
| `drop` randomly=true (death spill) | same | `(-sin(a)*r1*0.5, 0.2, cos(a)*r1*0.5)`, a = r2*2pi | 40 | 10 | 40 ticks |

r = `level.getRandom().nextFloat()` (Level vt+0xa70, then vt+0x28) for drops; the ctor uses the actor's random.
Both counters drop by 1 at the start of every postNormalTick, so the counts above are the ticks until pickup.

## Pseudo-code (verified on BDS unless marked)

```
post_normal_tick(it):                                                 # 1459892e0, after the AI step
    if it.pickup_delay != 0xffff and it.pickup_delay > 0: it.pickup_delay -= 1
    if it.throw_time > 0: it.throw_time -= 1
    if client: goto age                                               # clients never simulate items
    v = it.velocity; falling = falling_liquid(floor(pos)) or falling_liquid(floor(pos) - y)
    if HeadInWater:
        gravity = falling
        if not falling: v.y = v.y + 0.0005 + (0.06 - (v.y + 0.0005)) * 0.01; v.x *= 0.99; v.z *= 0.99
    elif aabb touches Lava (material 6):                              # fcn.140d2a4c0, inferred
        gravity = falling
        if not falling: same v.y; v.x *= 0.95; v.z *= 0.95
        if not item.fire_resistant (Item+0x152 & 0x20): LevelSoundEvent 27 Fizz at pos
    else: gravity = true
    set_flag(HAS_GRAVITY 49, gravity)
    if age > 0 and age % 25 == 0: merge(it)
    push_out_of_blocks(it, (pos.x, pos.y - 0.125, pos.z))            # may overwrite v
    if OnGround and |v|^2 <= 1e-4 and not WasInWater and age % 4 != 0: goto age
    if HAS_GRAVITY: v.y -= 0.04
    move(v)                                                           # Actor::move 140d24550, collisions
    f = 0.98
    if OnGround or (VerticalCollision and v.y <= 0):
        b = block(floor(x), floor(aabb.min.y) - 1, floor(z))
        f = b is air ? 0.588 : b.friction * 0.98                      # BlockType+0xb0, 0.6 default
    v.x *= f; v.y *= 0.98; v.z *= f
age:
    it.age += 1
    if it.age >= it.lifetime (6000):
        if item missing or item.should_despawn (Item+0x152 & 0x40): remove(it) else it.age = 0

merge(it):                                                            # inline in 1459892e0
    for o in actors of type 0x40 in it.aabb grown 0.5:
        skip if o == it, either not alive (vt+0x170), either pickup_delay == 0xffff,
             items not stackable together (141a57fd0), or (item vt+0x118) aux values differ
        big, small = o, it; swap if o.count < it.count                # on a tie, o absorbs it
        if big.count + small.count <= big.item.max_stack (vt+800; 255 if no item):
            big.count += small.count; big.pickup_delay = max(both); big.age = min(both)
            remove(small)                                             # RemoveActor; then a Level event, 0x45
    # no partial merges: two 40-stacks of a 64-item never merge

push_out_of_blocks(a, p):                                             # 140d51370
    c = floor(p); if block(c) is not solid (BlockType+0x164 & 4) or its box misses a.aabb:
        clear entity flag 3; set HAS_COLLISION; return
    free[d] = neighbour of c in direction d is not solid, d in {-x,+x,-y,+y,-z,+z}
    pick the free face nearest to p's fraction (fx, 1-fx, fy, 1-fy, fz, 1-fz), ties to the later test;
    none free: up, but only after the overlap test below (inferred)
    a.velocity = DIR[d] * (block+0x130 & 1 ? 0.1 : 0.25)               # all three axes replaced
    DIR = [down, up, -z, +z, -x, +x] (14a682180)
```

Spawn (`Spawner::spawnItem` 14118ce10): create type 0x40 at `pos`, copy the stack, `mThrowTime = throwTime`, add to
level. It sets no motion and no pickup delay; the caller does. `popResource` (141a241c0) passes `spawner = null`.

Player throw (`Actor::_drop?` 140d5dda0, `randomly = false`):

```
origin = interpolated_pos(actor, 1.0) (+1.62001 on y when isType(0x13e), minus VanillaOffset)   # 142c17110
item = spawnItem(region, stack, actor, origin - (0, 0.5, 0), throwTime 40)
item.rotation = actor (pitch, yaw); item.pickup_delay = 10
pitch, yaw in radians (sin/cos from the 65536-entry table 14cc495f8)
a = r1 * 2pi; s = r2 * 0.02                                           # level random, two nextFloat
v = (-sin(yaw)*cos(pitch)*0.3 + cos(a)*s,
     -sin(pitch)*0.3,                                                 # no +0.1, no (r-r)*0.1: unlike Java
      cos(yaw)*cos(pitch)*0.3 + sin(a)*s)
```

## Answers

1. **Tick** (`ItemActor::postNormalTick` 1459892e0, high): order is counters, buoyancy, merge, push-out, gravity
   (`vy -= 0.04`), `move`, friction (`0.98` air; `blockFriction * 0.98` on ground, so 0.588 on ordinary blocks;
   `vy *= 0.98` always), then `age++`. **No landing bounce** (Java's `vy *= -0.5` is absent). Pushed out of solid
   blocks: yes, every server tick, by setting the velocity to 0.25 (0.1 for some blocks) toward the nearest open
   face. Water: floats up (vy eases toward +0.06 at 1%/tick, gravity off, horizontal *0.99) unless the water is
   falling. Lava: same rise, horizontal *0.95, Fizz sound; burning is the generic actor fire damage on 5 health
   (not read). Lifetime 6000 ticks; items whose Item has the `+0x152 & 0x40` flag clear never despawn.
   Resting items (on ground, |v|^2 <= 1e-4, not just out of water) only run physics every 4th tick.
2. **Box** (high): 0.25 x 0.25, height offset 0.125. The actor position is the box **centre**, 0.125 above its
   bottom; the tick itself uses `pos.y - heightOffset` for the feet. MoveActor data copies that position, so the
   wire y is feet + 0.125 (medium: wire side inferred from 142d9aa80 reading StateVector pos).
3. **Spawn** (high): every item gets `(r*0.2-0.1, 0.2, r*0.2-0.1)` from its ctor. Block drops land at
   `blockPos + rand(0.15..0.85)` on all three axes. Player throws: position `origin - 0.5 y`, velocity 0.3 along
   the look direction (pitch included) plus a horizontal random spread of radius `<= 0.02`, no upward bias.
   `origin` for a standing player is believed to be eye height (feet + 1.62), so about feet + 1.12; see Unverified.
4. **Merge** (high): every 25 ticks within 0.5 blocks around the box, same stackable item, whole stacks only
   (sum <= max stack), the larger absorbs the smaller, keeps max pickup delay and min age.
5. **Packets** (medium): items use the generic per-actor sync, `ActorMotionSyncSystem::tick` -> 140d50930, every
   tick for every actor, nothing item-specific in the logic:
   - **Position**: `MoveActorDelta` (acacia-proto `MoveEntityDelta`; `MoveActorAbsolute` only on the client side or a
     teleport) when the position moved more than 0.01 (`d^2 > 1e-4`), or the byte-quantised rotations moved more
     than 4 degrees in sum, or the item has stopped (`|v|^2 < 1e-6`) with any residual difference > 0.001. Only
     changed fields are flagged (x, y, z, rot x, rot y, head = bits 0..5); OnGround/teleport/force are the header
     bits. Sent per viewer through a filter that applies `ConditionalBandwidthOptimization`.
   - **Motion**: only when `|v - lastSentV|^2 > 0.04` (a change of 0.2 blocks/tick), or when v becomes exactly 0.
     So a falling item sends motion rarely: at the first tick and at sharp changes such as landing.
     Packet: `SetActorMotion` (id 40) when a move went out this tick or a feature toggle (id 0x27, unresolved) is
     off; otherwise `MotionPredictionHints` (id 157, runtime id, motion, on_ground), which items request:
     `_addComponents` gives them `ConditionalBandwidthOptimization {max_optimized_distance 80,
     max_dropped_ticks 10, use_motion_prediction_hints true}`. Mobs get the same component from their JSON (horses,
     llamas) or not at all.
   - Spawn: `AddItemActor`; pickup: `TakeItemActor` then `RemoveActor` (from `Player::take`; packet names from the
     strings, not traced).
   - The client never simulates items (`postNormalTick` stops at the isClientSide test), so the movement packets
     are all it sees.
6. **Pickup delays** (high): block drop 10 (throwTime 10, delay 0); mob loot 10 (both 10); player throw **40**
   (throwTime 40, delay 10). Both must reach 0 and there is no owner exception, so nobody, the thrower included, can
   take a thrown item for 40 ticks. sapling's `PICKUP_DELAY = 40` and `SPILL_DELAY = 10` (`drops.rs:18-20`) match.

## sapling mapping (one shared body)

The item fits a generic body with: box 0.25 x 0.25, position = centre (feet + 0.125), gravity 0.04 then move then
drag 0.98 (horizontal `friction * 0.98` on ground), no bounce, a push-out step, and per-medium hooks (water and
lava buoyancy replace gravity). Falling blocks (`falling-blocks.md`) share gravity/move/drag but bounce
(`vxz *= 0.7, vy *= -0.5`) and ignore block friction; orbs (`experience.md`) are the item rule plus the Java bounce.
Send `MoveEntityDelta` when moved > 0.01, `SetEntityMotion` when v changed by > 0.2 or stopped.

## Unverified / inferred

- The throw origin: `142c17110(player, 1.0)` = interpolated StateVector pos + DynamicRenderOffset, +1.62001 when
  `isType(0x13e)` (0x13e = TripodCamera; mask semantics would also match Player 0x13f), minus `VanillaOffsetComponent`.
  Whether a player's StateVector pos is already eye height (then the two cancel through VanillaOffset) was not
  settled. One capture of a thrown item's AddItemActor y against the thrower's MovePlayer y settles it.
- Whether the spawn position given to `spawnItem` becomes the centre (pos) or the feet of the new item.
- `fcn.140d2a4c0('\x06', region, aabb)` being "AABB touches Lava material"; `vt+0x170` being `isAlive`.
- The meaning of `BlockType+0x130 & 1` (push speed 0.1 vs 0.25) and `+0x164 & 4` (solid); the push-up fallback
  when no face is free; entity flag 3 set by push-out.
- `Actor::move` collision response (assumed to zero vy on landing; not read).
- Which motion packet a client actually gets (SetActorMotion vs MotionPredictionHints: toggle 0x27 at 14cc224c0) and
  what the per-viewer bandwidth filter (1414407f0, 1413e7730) drops beyond 80 blocks.
- Player-side touch: which box overlap calls `playerTouch` (Java inflates the player box by 1, 0.5, 1); not read.
- Item+0x152 bits 0x20 (fire resistant) and 0x40 (should despawn) from use, not headers.
- 140d5e660 (`dropTowards?`, throwTime 40, pickup 10, uses 0.1 and 0.2) was not decompiled.
