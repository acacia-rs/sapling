# BDS ender dragon entity: AI, movement, damage (bdsre reading)

Build 6ab54bee (BDS 1.26.52.3, `.testserver/bds-molang`). Static reading plus a few fits to
`captures/bds-dragon-*.txt`. Same house style as dragon-fight-decompile.md (the fight manager). "Unverified"
marks a reading with no second source. Every function below is labelled in bdsre.

Conventions: `D` = the EnderDragon actor. `sv` = StateVectorComponent (`D+0x218`: pos +0, posPrev +0xc,
posDelta +0x18; f32). `rot` = `D+0x228` (+0 pitch, +4 yaw). `flag(24)` = actor flag 24 (SITTING on the wire),
read with 142535e40. All maths is f32. `sin`/`cos` are UCRT sinf/cosf, `atan2` atan2f, `wrap(a)` is
`fmodf(a+180,360)`, plus 360 if that is negative, minus 180. "level random" = `level vt 0xa68` (Random*):
`vt 8` nextInt(n), `vt 0x28` nextFloat. Every random draw in this file uses it unless stated otherwise. The
dragon has no random of its own.

## 0. Layout and entity JSON

- Vtable 14a88bde0 (ctor `EnderDragon::EnderDragon` 145993350). The overrides that matter here: slot 55
  `setSitting` 145998920, 63 `isInvulnerableTo` 145999f20, 133 `_hurt` 145997ba0, 121 `kill` 145997d10,
  35 `getHeadLookVector` 145998960. The slots follow the LeviLamina Actor.h declaration order
  (setTarget = 0x190, performRangedAttack = 0x1a8, isInvulnerableTo = 0x1f8 all agree).
- Fields: +0x778 ring index, +0x478 ring buffer of 64 x {yaw, y, 0}; +0x780 head, +0x788 neck, +0x790 body,
  +0x798/+0x7a0/+0x7a8 tail 1..3, +0x7b0/+0x7b8 wings: pointers to AABBs (min xyz, max xyz). +0x7c0 `slowed`
  (byte). +0x7c4 growl timer. +0x7c8 damage taken while sitting. +0x7cc yaw-factor numerator. +0x7d0..+0x7d8
  target Vec3 (all three zero means "no target"). +0x7dc flame count. +0x7e0 nodes-built latch. +0x7e8 node
  vector (0x38 bytes per node, x/y/z ints at +0x28/+0x2c/+0x30). +0x800 neighbour masks (24 ints). +0x818 A*
  heap, +0x830 heap size. +0x838 healing crystal id. +0x840 crystal count (from the fight). +0x844
  previouslyKilled. +0x880 kill callback. `D+0x290` is the target actor id (Mob target, `vt 0x190`
  setTarget).
- Components from the code: AirSpeedComponent (141cd5a90 get / 141cd5b70 set) is Java's
  `getMaxRiseOrFall`, set from JSON `flying_speed` 0.6. The MobRotationComponent yaw speed (141cd5710 get,
  141cd57f0 set) is Java's `randomYawVelocity`. WingFlapDataComponent {prev, anim} holds Java's
  `prevAnimTime`/`animTime`.
- `setSitting(b)` 145998920: sets flag 24 = b and **zeroes posDelta x/y/z**.
- ender_dragon.json (packs `vanilla_1.26.50`): health 200, collision box 13 x 4, `attack` 3 (unused here),
  flying_speed 0.6, movement 0.3, knockback_resistance 100, fire/freezing immune, fall damage ignored,
  physics `has_collision false, has_gravity false`, `game_event_movement_tracking emit_flap`, boss
  hud_range 125.
  - Groups: `dragon_flying` = takeoff (prio 0), chargeplayer (1), strafeplayer (2), holdingpattern (3), plus a
    shooter `minecraft:dragon_fireball`. `dragon_sitting` = landing (0), flaming (1), scanning (2).
    `dragon_death` = dragondeath (0).
  - Events: spawned → +flying. `on_start_landing` → `minecraft:start_land`: +sitting −flying.
    `on_start_takeoff` → `minecraft:start_fly`: +flying −sitting. `on_death` → `minecraft:start_death`:
    +death −sitting −flying.
  - In this file, **"fire landing"** means the code looks up the OnStartLanding trigger (static id
    0x14ccb964c) and runs it, and **"fire takeoff"** means the same for OnStartTakeoff (0x14ccb9654). Both
    go through 140d29190/140d29010.
- dragon_fireball.json: projectile power 1.3, gravity 0, inertia 1, uncertainty_base 10, offset (0,0.5,0),
  hit sound explode. on_hit: `spawn_aoe_cloud` radius 6, radius_on_use 0, duration 120, potion 23,
  particle `dragonbreath`, reapplication_delay 20, affect_owner false.

## 1. Path graph

Nodes are built in `EnderDragon::findClosestNode` 145997df0, once (latch +0x7e0), the first time
`region vt 0x1b8 (BlockPos(0,0,0), 60)` (hasChunksAt within 60, unverified) is true. Until then the node
vector stays empty and every node index is 0.
- For i in 0..23:
  - i < 12: k = i, step 0.2617994 (pi/12), r 60, yoff 5.
  - 12 ≤ i < 20: k = i−12, step 0.3926991 (pi/8), r 40, yoff 15.
  - i ≥ 20: k = i−20, step 0.7853982 (pi/4), r 20, yoff 5.
  - `a = (k*step − 3.1415927)*2`, `x = (int)(cos(a)*r)`, `z = (int)(sin(a)*r)`. These are C casts, so they
    truncate (Java floors). The truncation is assumed from the decompile and was not checked in `dis`.
  - `y = max(heightmapY(x, z) + yoff, dim[+0xcc] + 10)` then `min(y, 128)` (if y > 127 then 128). The
    heightmap is `region vt 0x170`; `dim+0xcc` is presumably sea level. The cap at 128 is not in Java.
- Neighbour masks are Java's: 0x1802 0x2005 0x200a 0x4014 0x8028 0x8050 0x100a0 0x20140 0x20280 0x40500
  0x80a00 0x80401 0x182001 0x305006 0x20a008 0x614030 0x428040 0xc50180 0x8a0200 0x941c00 0xe83000
  0xd0e000 0xb38000 0x7e0000.
- **Closest node** (the tail of 145997df0, and the same loop inlined in the goals): `p = floor(target)`
  as ints. Scan from index `crystals != 0 ? 0 : 12` to 23, keeping the node with the smallest squared
  distance, f32, starting from 10000 with a strict `<`. Returns 0 if nothing is under 10000.
- **`findPath(start, end, extra)`** 145999380 is Java's A*. It resets all nodes; g = 0 and h = |start−end|
  (Euclidean f32); a binary heap on f = g+h. Neighbours are scanned only from `crystals != 0 ? 0 : 12`, in
  index order; a neighbour updates when it is unassigned or `g + |cur−n| < n.g`. It stops when the popped
  node's coordinates equal end's. `extra` (a node, may be null) is appended after end. No path to end
  returns the path to the node closest to end, or null if that node is start. Path points are 16 bytes
  (x, y, z ints); the index is at path+0x18 (`[3]`). Reconstruction is 145999970. Whether
  `changeDistance` (142f67e10) uses g+h as in Java is unverified.
- **Navigate to the next node** (inlined in each goal): `n = path[idx]`, idx++,
  `y = n.y + nextFloat()*20` (the do-while can never repeat), target = (n.x, y, n.z) as f32. There is **no
  +0.5**. One nextFloat per node.

## 2. Phases (goals)

Goal vtables are 14a942390 + k*0x60 (no RTTI; names come from their debug-name methods). Slots:
canUse / canContinueToUse / start / stop / tick, labelled `DragonXGoal::*`. The goal's `+0x30` (or `+0x40`)
is D.

### HoldingPattern (flying, prio 3): 148a3c910 / c970 / c9c0 / ca90 / cb30
- canUse: `!flag(24)` and no target actor. canContinue: no target actor.
- start: target := 0, `setTarget(null)`, path := null, `setSitting(false)` (vt 0x1b8). stop: target and path
  cleared.
- tick:
  1. If the target is set, `100 ≤ dist²(pos, target) ≤ 22500`, and D has neither
     HorizontalCollisionFlag nor VerticalCollisionFlag: return.
  2. If a path exists and is finished:
     - `podY = heightmapY(0,0)` (vt 0x170 at BlockPos 0,0,0).
     - `nextInt(crystals+3) == 0`: **fire landing**, return. The new path is not chosen this tick.
     - `P` = nearest attackable player to (0, podY, 0) within **64** (§2.9).
       `j = P ? (int)(distSq((0.5, podY+0.5, 0.5), P.pos) * 0.001953125) : 64`.
     - Draw `r1 = nextInt(|j|+2)`. It is always drawn, even without a player.
     - If P exists, and either r1 == 0 or `nextInt(crystals+2) == 0` (the second draw happens only when
       r1 != 0): if P is not a spectator (game type 6, or 5 with level default 6) and
       `canSee(P.getAttachPos(3))` (140d4f980): `setTarget(P)` and return. Strafe takes over.
  3. If the path is null or finished, build a new one:
     - `i = findClosestNode()`, j = i.
     - `nextInt(8) == 0`: clockwise (goal+0x38) ^= 1 and j += 6.
     - j += clockwise ? 1 : −1.
     - crystals ≥ 1: `j = j mod 12` (+12 if negative). crystals < 1: `j = ((j & 7) ^ 4) + 12`, i.e. Java's
       `((j−12)&7)+12`. **Unlike Java** (whose `>= 0` test is always true), with no crystals BDS circles the
       40-radius ring, nodes 12..19.
     - `path = findPath(i, j, null)`, then idx++ (the start node is skipped).
  4. Navigate to the next node (§1).

### Landing = Java LandingApproach + Landing (sitting group, prio 0): 148a3d5c0 / d5e0 / d670 / d730 / d780
- canUse `!flag(24)`.
- start/stop: target := 0, path := null, AirSpeed := 0.6, +0x7cc := 0.7, "landing" byte goal+0x40 := 0.
- tick (while goal+0x40 == 0): the same 100/22500/collision early-out as the holding pattern. Then, if the
  path is null or finished:
  - start = closest node, `podY = heightmapY(0,0)`, P = nearest player to (0, podY, 0) within **128**.
  - P alive: end = closest node to (floor(−n.x*40), 105, floor(−n.z*40)), where n = P's xz position
    normalised (zero if shorter than 1e-4). Without P: end = closest to (40, podY, 0). Both scan from the
    crystal start index. The decompile is garbled here and the formula is assumed from Java.
  - `path = findPath(start, end, extra = node(0, podY, 0))`, idx++.
- Then navigate to the next node. When idx ≥ size: goal+0x40 = 1, target = (0, podY, 0) (podY is the
  heightmap value again, the first air above the pillar = 66 on BDS's island), AirSpeed := 1.5, +0x7cc := 0
  (Java's landing yaw factor).
- canContinue: once +0x40 is set and `dist²(pos, target) < 1`: flame count +0x7dc := 0, `setSitting(true)`
  (which zeroes posDelta), return false. Drill: sat at (0.0615, 66.236, 0.134), with the flags change in the
  same tick.

### Scanning (sitting, prio 2): 148a3e130 / e190 / e6e0 / e740 / e760
- canUse: flag(24) and no target actor.
- start: timer (goal+0x38, f32) := 0, target := 0, `setTarget(null)`, `setSitting(true)`. stop: target := 0.
- tick: timer += 1. P = nearest player to floor(D.pos) within **20**. If P is alive:
  - `n1 = norm2(P.xz − D.xz)`, `n2 = norm2(sin(yaw), −cos(yaw))`, `ang = acos(dot)*57.295776 + 0.5`.
  - If ang < 0 or ang > 10:
    - `dx, dz = P.xz − floor(D.xz)` (the decompile shows floors; Java uses the head's position, unverified).
    - `t = wrap(180 − atan2(dx,dz)*57.295776 − yaw)`, clamped to [−100, 100].
    - `s = sqrt(dx²+dz²) + 1`.
    - `yaw = yaw + t*(1/min(s,40)/s) + yawSpeed*0.8`, then yaw speed is set (141cd57f0). The numerator is
      **1.0, not Java's 0.7**. As read, yaw speed is not decayed separately (unverified).
- canContinue:
  - P within 20 found: P removed (+0x269), or timer ≤ 25, continues. Otherwise `setTarget(P)` and stop;
    Flaming then starts.
  - No P: continue while timer < 100. At 100, P2 = nearest player within **150**. If P2 is not a spectator
    and visible (attach pos 3): `setTarget(P2)`. Then **fire takeoff** and stop. With a target, the flying
    group's Charge wins; without one, Takeoff.

### Flaming (= Java SittingAttacking + SittingFlaming; sitting, prio 1)
Functions: 148a3baa0 / baf0 / bd30 / bd80 / bdd0, ctor 148a3b8f0, factory 148972370.
- The definition defaults are read from the fill lambda 148971c30 (const 14a935af0). JSON sets none of
  them:

  | field | default | ticks | goal field |
  |---|---|---|---|
  | ground_flame_count | 4 | | +0x38 |
  | cooldown_time | 10 s | 200 | +0x3c |
  | flame_time | 0.5 s | 10 | +0x44 |
  | roar_time | 2 s | 40 | +0x4c |
  | radius | 5.0 | | +0x54 |
  | cloud duration | 10 s | max(0, 200) | +0x50 |
  | colour | "#DC00EF" | | +0x58 |

- canUse: a target actor exists (set by Scanning).
- start: tick counter +0x48 := 0, target := 0, `setSitting(true)`, flame count D+0x7dc += 1. stop:
  counters := 0, target := 0, `setTarget(null)`.
- tick:
  - **Roar:** `t = +0x48++`. While t < 40: LevelSoundEvent **19 (Mad)** at `getAttachPos(1)` every tick,
    then return.
  - After the roar: `c = ++(+0x40)`. If c < 10 and c is even, `level vt 0x6b8` (broadcast entity event) with
    **EntityEvent 68 DragonPuke**, data 0.
  - At c == 10, the breath cloud:
    - H = floor(head AABB centre) (int xyz).
    - `n = norm2(H.x − D.x, H.z − D.z)`, `px = H.x + n.x*radius*0.5`, `pz = H.z + n.z*radius*0.5` (2.5).
    - Cell = (floor px, H.y, floor pz). While the cell is air, y−1 (stops at the dimension minimum).
    - Spawn `area_effect_cloud` (actor type 0x5f) at (px, y+1, pz); owner = D. Synched data: duration 200,
      radius 5, a −0.5 field (data slot 0x310, probably radius-on-use, unverified), potion 23. Effects
      cleared, then effect id 7 (instant damage; amplifier and duration not read). Particle 49, colour
      string, two bytes at +0x77/+0x79 := 0. The cloud's own rules are generic (not read).
- canContinue: while c < 200. At c ≥ 200, if flame count ≥ 4, **fire takeoff**. Otherwise the goal just
  ends: target is null, so Scanning runs again.

### Takeoff (flying, prio 0): 148a3ffe0 / 40040 / 40140 / 401f0 / 40290
- canUse: flag(24) and no target actor (flag 24 is still set right after start_fly).
- start: target := 0, path := null, `setSitting(false)`, firstTick := 1.
- tick, first tick only:
  - i = closest node. v = `getHeadLookVector(1.0)` (vt 0x118).
  - j = closest node to (floor(−v.x*40), 105, floor(−v.z*40)) from the crystal start index. Garbled; the
    formula is assumed from Java.
  - j fixed as in the holding pattern (crystals ≥ 1: mod 12; else `((j&7)^4)+12`).
  - `path = findPath(i, j, null)`. Navigate to the first node: **no idx++ before it**, so the target is the
    start node, with one nextFloat*20.
- canContinue: `dist²(pos, (0.5, podY+0.5, 0.5)) ≤ 100`. Past 10 blocks it stops and HoldingPattern takes
  over (the target is still set, so that goal's first check usually passes).

### StrafePlayer (flying, prio 2)
Functions: 148a3ed70 / edd0 / eef0 / f4c0 / f520, ctor 148a3ec50, factory 148974f20.
- Defaults (fill 148974860, consts 14a935ae0/14a935b10): active_speed 0.6 (+0x38), flight_speed 0.6 (+0x3c),
  turn_speed 0.7 (+0x40), target_zone 10..150 (+0x44/+0x48), switch_direction_probability 0.125 (+0x84),
  target_in_range_and_in_view_time 0.25 s → 5 (+0x78), fireball_range 64 (+0x80), view_angle 10 (+0x88),
  +0x7c = 3.0 (ctor only; passed to performRangedAttack).
- canUse: `!flag(24)` and a target actor exists. canContinue: not fired (+0x71) and the weak ref to the target
  is alive and not removed.
- start:
  - fired := 0, charge (+0x74) := 0. Keep a weak ref to the target and `setTarget(T)`.
  - i = closest node to D; j = closest node to floor(T.pos).
  - `extra = node(floor T.x, (int)(T.y + min(dist2(T,D)/80 + 0.4 − 1, 10)), floor T.z)`, dist2 being
    horizontal.
  - `path = findPath(i, j, extra)`, idx++, navigate.
  - Then `setSitting(false)`, AirSpeed := 0.6, +0x7cc := 0.7. stop: `setTarget(null)`, target := 0,
    AirSpeed := +0x3c, +0x7cc := +0x40.
- tick (T resolved through the owner ref):
  1. If the path is finished: target := (T.x, T.y + min(dist2/80 + 0.4 − 1, 10), T.z), exact floats.
  2. d² = dist²(D, target), 0 if there is no target. If not 100 ≤ d² ≤ 22500, a new path: as the holding
     pattern, except the direction switch is `nextFloat() ≤ 0.125`, drawn only if the probability != 0,
     from **`level vt 0xa70`**, a different random accessor (unidentified). Then navigate.
  3. If dist²(T, D) ≥ 64²: charge−− if > 0. Otherwise, if T is not a spectator and is visible at attach
     pos 3: charge++, and `ang = acos(dot(norm2(T−D), norm2(sin yaw, −cos yaw)))*57.295776 + 0.5`. If
     charge ≥ 5, 0 ≤ ang < 10 and not fired: ShooterComponent prep (1424324f0), `performRangedAttack(T, 3.0)`
     (vt 0x1a8, so the shooter spawns `dragon_fireball`; aim and spawn point are not read), charge := 0, path
     idx := size, fired := 1. canContinue then fails and HoldingPattern resumes. Not visible: charge−− if > 0.
- Java's level event 1017 (fireball sound) was not seen; the shooter path may do it.

### ChargePlayer (flying, prio 1)
Functions: 148a3ad70 / add0 / ae20 / b020 / b080, factory 14896f880.
- Defaults (14896f1d0): active_speed **3.0**, flight_speed 0.6, turn_speed 0.7, target_zone 10..150,
  continue_charge_threshold_time 0.5 s → 10 (+0x54).
- canUse: flag(24) and a target actor exists (Scanning set one at 100 ticks).
- start: counter (+0x50) := 0, `setSitting(false)`, AirSpeed := 3.0, +0x7cc := 0.7. Target := the position
  of the nearest player within 150 of floor(D.pos), or (0,0,0) if there is none.
- canContinue: target non-zero and (counter < 1, or counter++ < 10).
- tick: if dist² < 10² or > 150², or a collision flag is set: counter++.
- stop: target := 0, AirSpeed := 0.6, `setTarget(null)`.

### Death (death group, prio 0): 148a3b2a0 / b2c0 / b320 / b350
- canUse: health > 0. `_hurt` keeps health at 1 (§4).
- start: target := 0, AirSpeed := 3.0, +0x7cc := 0.7, timer := 0. stop: AirSpeed := 0.6.
- tick:
  - Every 10 ticks (old timer % 10 == 0): 3 nextFloats, then `level vt 0x7e8` addParticle(type 17,
    D.pos + ((r1−.5)*8, 2 + (r2−.5)*4, (r3−.5)*8)). Probably not sent by a server (unverified), but it
    consumes 3 draws.
  - Target unset: target := (0, heightmapY(BlockPos 0,100,0), 0).
  - If dist² < 100, or > 22500, or a collision flag is set: health := 0 and **EntityEvent 37
    EnderDragonDeath**, data 0. Else health := 1.
  - D flies there under aiStep (AirSpeed 3). At health 0 aiStep stops and `tickDeath` runs (fight doc §5).
- The drills saw DeathAnimation (3), not 37, with no MoveEntity. So those deaths probably came through
  `kill` 145997d10: unless EditorActorPaused, it calls the kill callback +0x880 (and removes). Or the dragon
  was already near the portal. Unverified which.

### 2.9 Player search (`fetchNearestAttackablePlayer`, predicate 14188aec0)
- Candidates fail an ability check across six ability layers. Probably Invulnerable (creative); unverified.
  `vt 0x170` must also be true.
- Range r: ×0.8 if sneaking (flag 1). Invisible: × max(armour cover, 0.1) × 0.7 unless the seeker has
  CanSeeInvisible. ×0.5 for a matching mob head.
- 3D squared distance from the integer centre. r < 0 means unlimited. Nearest wins.
- Spectators are filtered separately by the goals (game type 6, or 5 with level default 6). Creative
  players are not filtered there.

## 3. Movement physics: `EnderDragon::aiStep` 145994090

Called from `ActorPostAiStepTickSystem::tick` 145e3d3a0 for each entity with EnderDragonFlagComponent, after
the goals have ticked. Server and client share it. In order:

1. `health < 1`: return (the death is tickDeath's job).
2. Flap sound (not when flag 17 is set, probably silent): if `cos(prev*2pi) ≤ −0.3 ≤ cos(anim*2pi)`,
   LevelSoundEvent **28 Flap** at `getAttachPos(1)`. Sitting gives pos+(0,2,0) on the wire. These are the
   values from before this tick's update, as in Java.
3. Growl, not sitting: `if (growl-- < 1)`: LevelSoundEvent **19 Mad** at `getAttachPos(3)`, then
   `growl = 200 + nextInt(200)`. +0x7c4 starts at 0 per the ctor, unread.
4. Heal: if the healing crystal (+0x838) is loaded and `tick(+0x198) % 10 == 0` and health < max:
   health := health+1 (attribute current value). No range check after picking.
5. `nextInt(10) == 0`: crystals (type 0x47) in D's AABB grown by 32. Nearest by 3D dist², which must be
   < **1024** (Java has no limit). +0x838 := its id, or −1 if none. One draw every tick.
6. Animation: `prev = anim`. f = `0.2/(sqrt(Δx²+Δz²)*10+1) * exp2(posPrev.y − pos.y)` (14a5bcf90 on a
   double), where Δ = pos − posPrev at this point (≈ this tick's generic move, below).
   - The sign of the exp2 argument is as read: Java's is `pow(2, motionY)`.
   - Sitting: anim += 0.1 (a flap every 10 ticks, matching the drill). Else anim += f, halved if slowed.
7. `yaw = wrap(yaw)`. Ring buffer: on first use (index < 0) fill all 64 with {yaw, y, 0}. Then
   `idx = (idx+1) % 64`, `ring[idx] = {yaw, pos.y}`.
8. **If the target is non-zero** (a target at (0,0,0) counts as none):
   1. `dx, dy, dz = target − pos`. `k = AirSpeed`. `posDelta.y += clamp(dy/sqrt(dx²+dz²), −k, k) * 0.1`.
   2. `yaw = wrap(yaw)`. `t = clamp(wrap(180 − atan2(dx,dz)*57.295776 − yaw), −50, 50)`.
   3. `a = norm(dx,dy,dz)`. `b = norm(sin yaw, posDelta.y, −cos yaw)` (zero below length 1e-4).
      `f7 = max((dot(a,b) + 0.5)/1.5, 0)`.
   4. `s = sqrt(Δ.x² + Δ.z²) + 1`, with posDelta xz here. `factor = (+0x7cc != 0) ? +0x7cc/min(s,40)/s :
      min(s,40)/s`. The second form is landing (+0x7cc = 0).
   5. `ys = yawSpeed*0.8 + t*factor` (stored). `yaw += ys*0.1`.
   6. `f8 = 2/(dx²+dy²+dz² + 1)`. `sp = 0.06*((1−f8) + f7*f8)`. `posDelta.x += sin(yaw)*sp`,
      `posDelta.z −= cos(yaw)*sp` (new yaw; Java's moveRelative(0,0,−1,sp)).
   7. `Actor::move(slowed ? posDelta*0.8 : posDelta)` (140d24550; no block collision, has_collision false).
   8. `c = norm(posDelta)`, `f10 = 0.8 + 0.15*(dot(c, b) + 1)*0.5`. posDelta.x,z *= f10; posDelta.y *= 0.91.
9. 1429eb610 (presumably body yaw = yaw; unread).
10. Parts. `L(n)` = ring[(idx−n)&63]. Interpolation factor 0, since the partial tick is 1.0. `hh` = half
    the main AABB height (2).
    - `f12, f13 = cos, sin((L5.y − L10.y)*10*0.017453292)`. `S, C = sin, cos(yaw*0.017453292)`.
    - Each part keeps its own size and is moved to a new centre:
      - body: (x + S*0.5, y + hh, z − C*0.5).
      - wing 1: (x + C*4.5, y + 2 + hh, z + S*4.5). Wing 2: (x − C*4.5, y + 2 + hh, z − S*4.5).
    - Server, hurtTime (`**(D+0x428)`) == 0 and **not sitting**:
      - wing knockback on entities in each wing box grown by (4,2,4) then shifted −2 in y (§5).
      - head and neck boxes grown by 1: bite (§5).
    - `h = yaw*0.017453292 − yawSpeed*0.01`; `Sh, Ch = sin h, cos h`. `f17 = sitting ? −1 : L5.y − L0.y`.
      - head: (x + Sh*6.5*f12, y + f13*6.5 + hh + f17, z − Ch*6.5*f12). Neck: the same with 5.5.
    - Tails k = 0..2: `L = L(4+2k)`. The code says **4/6/8** (`lea r8d,[rdi*2+4]`); Java uses 12+2k.
      - `g = yaw*0.017453292 + wrap(L.yaw − L5.yaw)*0.017453292`, `w = 2(k+1)`.
      - centre = (x − (S*1.5 + sin g*w)*f12, (L.y − L5.y) + y − (w+1.5)*f13 + 1.5,
        z + (C*1.5 + cos g*w)*f12). No +hh on the tails.
11. Server: `slowed = checkWalls(head) || checkWalls(neck) || checkWalls(body)`. This **short-circuits**,
    while Java's `|` does not: once the head is blocked, the neck and body are not cleared that tick.
12. The boss bar fraction is updated when it changes (BossEvent 4).

**Two moves per tick (fitted, exact to 4 digits on the first 5 ticks of bds-dragon-fight).**
- The observed y steps are −0.06, −0.1692, −0.2686, −0.359, −0.4413. They fit:
  - the generic actor movement moves D by the stored posDelta w (no drag), then
  - aiStep adds a = −0.06 (clamp −0.6 × 0.1), moves by w+a, and damps.
- So `d_n = 2w_n + a`, `w_{n+1} = 0.91(w_n + a)`.
- The generic mover's address was not found. It runs before aiStep in the same tick. The horizontal axes
  presumably double the same way (unverified).
- setSitting zeroes posDelta, so a sitting dragon does not drift. While the target is zero in flight, the
  generic move keeps carrying the last posDelta.

**Pitch.** Not touched by any dragon code. The ctor (fight `_createNewDragon`) gives `moveTo(pos,
(random*360, 0))` with Vec2 = (x pitch, y yaw), so the **pitch** is random and the yaw 0. AddEntity showed
pitch 30.94, yaw 0. The wire pitch then falls 10°/tick to 0 (30.9→21.1→11.25→1.41→0). That is generic
look/body control, approaching 0 at 10°/tick; its function was not read. Body and head yaw equal the yaw on
the wire.

## 4. Damage taken: `EnderDragon::hurtPart(result, part, source, dmg)` 145997370

`_hurt` (slot 133, 145997ba0) calls it with part = `D+0x180` (the main AABB, which is not head or neck).
`onCrystalDestroyed` calls it with the head. In order:
1. Sitting, cause **Projectile (3)**, and the damaging entity type is Arrow 0xC00050, Trident 0xC00049 or a
   wind charge (0x40008D / 0x40008F, by low byte & 0xfd): `142f1db30(projectile, 5, …)` (probably
   sets it on fire for 5 s, unverified), dmg := 0.
2. Part is not head and not neck: `dmg = min(dmg, 1) + dmg*0.25`. A fist (1) gives 1.25, as the drill
   showed; Java's d/4+1 differs below 1.
3. Rejected (no damage, result zero) when dmg < 0.01, or cause is **Suffocation (4)**, or `vt 0x330(D) > 0`
   (unidentified; perhaps a hurt cooldown).
4. Accepted only when **the source entity is a player** (source vt 0x68 == 0x13f), or cause is
   **BlockExplosion (10)**.
   - Cause SelfDestruct (13) runs the kill callback, then `remove()`.
   - Anything else is ignored: mobs, Override, EntityExplosion without a player, magic, /damage's cause.
     This is why `/damage` fails.
5. `Mob` hurt core 14298f530 applies it (the generic hurt-time rules). A player source also adds dmg to a
   per-player stat (141bd05d0).
6. Health < 1 after the hit: health attribute := **1.0**, `D+0x418` byte := 1, the on_death trigger runs
   (start_death, so the Death goal), boss 141b78a70. The dragon flies to the portal at 1 HP (§2).
7. Sitting: `+0x7c8 += hpBefore − hpAfter`. If it is > maxHealth*0.25 (50): reset it and **fire takeoff**.
- `isInvulnerableTo` 145999f20: sitting and cause Projectile and source `vt 0x90` == 0 → invulnerable; else
  the Actor rule.
- The hurt sound, HurtAnimation (EntityEvent 2) and knockback are generic Mob. Knockback resistance 100. The
  `fall` damage_sensor sets no damage.

## 5. Damage dealt
- **Wings**, `EnderDragon::knockBack` 145996130: for each entity in the box that is a mob type (0x100) or a
  player (0x13f):
  - If PushableByEntity: `dx, dz = e.xz − bodyCentre.xz`, `d = dx²+dz²` (no Java `max(.., 0.1)`, and no
    sqrt), push (dx/d*4, 0.2, dz/d*4) via 142fa31d0 (add-velocity, unverified).
  - Then, unless D is sitting or `e.tick − 2 ≤ e.lastHurtByMobTime (+0x34c)`: hurt **5.0**, cause
    EntityAttack (2) from D. If dealt, 140d39660 (enchant post-hurt effects) with 5.0.
- **Head and neck**, `EnderDragon::hurtEntities` 145996570: mob or player: hurt **10.0**, cause EntityAttack
  from D, plus 140d39660. There is no tick or revenge gate, but `Actor::hurt`'s own invulnerability time
  applies.
- No difficulty scaling in dragon code. Player difficulty scaling of mob damage is generic, not read here.
- Breath cloud (§2 Flaming) and dragon_fireball → aoe cloud (JSON, §0). The cloud damage values are the
  instant-damage potion's (generic).

## 6. Crystals and healing
- Healing: §3 steps 4 and 5. +1 HP every 10 ticks of the dragon's own tick counter, while the chosen crystal
  is loaded. It is re-chosen when `nextInt(10)==0`: the nearest crystal within 32 of D's position, searched
  in AABB ±32.
- Beam: no BlockTarget or actor-data write for it was found in aiStep. How the client draws the healing beam
  is **not found**; it may be client-side from +0x838, which is not synced as far as was read.
- **`onCrystalDestroyed(D, crystal, pos, source)`** 145998d00, called from the fight after `+0x840` is
  updated:
  - attacker = the source's player if the source entity type is a player; else the nearest player within
    **64** of the crystal's block pos.
  - If D's healing crystal is loaded and its id == this crystal's id: `hurtPart(head, src, 10.0)`, src =
    cause BlockExplosion (10) from the attacker (or with no entity if the attacker is null or removed). This
    passes the §4 gate through cause 10, so it lands even without a player. The head is not reduced by /4.
  - Not sitting: `setTarget(attacker)` (null clears) and **fire takeoff**. The target makes Strafe (prio 2)
    beat HoldingPattern. Whether re-adding the flying group resets the running goals is unverified.
  - Java's creative check (`disableDamage`) is absent.

## 7. Blocks: `EnderDragon::checkWalls(aabb)` 1459967e0
- Loops x (outer) from floor(minX) to floor(maxX), then y, then z (inclusive ints).
- Skip structure_void, the light_block_0..15 list (14ccd3370), and Material Air (0) or Fire (10).
- Otherwise, with GameRule 14 (mobGriefing) on, and the block is none of:
  - bedrock, obsidian, crying_obsidian, respawn_anchor, end_stone, end_portal_frame, end_portal,
    end_gateway, iron_bars, reinforced_deepslate, barrier, allow, deny, border_block;
  - command, repeating and chain command blocks, structure_block, jigsaw;
  - the copper_bars family (8, list 14ccd2518);
- then a gameplay-event check (level vt 0x618 handler; a veto or a missing handler counts as blocked), then
  `region vt 0x110` (remove/destroy block, sending UpdateBlock). The result ORs into `destroyed`.
- Excluded blocks, or griefing off: `blocked = true`.
- After the loop, if destroyed: 3 nextFloats, a point in the box, `level vt 0x698` LevelEvent **2009** (shown
  as ParticleProjectileHit) with **data 65** at that point. One event per box per tick, as seen by the
  pillar torches.
- Returns blocked, which becomes `slowed` (×0.8 move, half flap speed).
- Runs every tick on the server, sitting included (only the wing and bite step is gated by sitting). Called
  in the order head, neck, body, short-circuited.

## 8. Death and XP
- Two paths:
  - hurt → health clamped to 1 → start_death → Death goal flies to the podium → health 0 + EntityEvent 37 →
    `EnderDragon::tickDeath` (fight doc §5).
  - `kill` (slot 121) → kill callback at once.
- tickDeath: `move(0, 0.1, 0)` each tick is called, but the drills saw no MoveEntity. Explosion particles come
  only from the Death goal's addParticle (client). The death sound is event 14 at t == 2.
- The XP drop 141171200 does **not** spawn orbs: it queues a request (pos/amount 0x18 bytes, kind) in a
  deque, max 0x200 entries, logging when over. The consumer and the orb split were **not traced**.

## 9. Sounds (LevelSoundEvent ids)
- 28 Flap: §3 step 2, at attach pos 1.
- 19 Mad: the in-flight growl every 200..399 ticks (attach pos 3); every tick of the 40-tick roar in Flaming
  (attach pos 1); the fight's respawn sound at (0,128,0) (this closes "sound 0x13" in the fight doc).
- 14 Death: tickDeath.
- Hurt: generic.
- Spawn: AddEntity time, at pos + (0, 1, −5.69) in the drill (generic, not read).

## Left open
- The generic per-tick move before aiStep and the pitch decay (§3).
- `vt 0x330` in hurtPart; `142f1db30` on deflected projectiles.
- The ability tested by the player search; the random behind `level vt 0xa70` (strafe).
- The healing beam; the XP consumer and orb split; performRangedAttack's aim.
- Landing and Takeoff end-node formulas are inferred from Java because the decompile is garbled there;
  `dis` them before relying on them.
