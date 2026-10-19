# bds-ghast findings

Capture: `research/captures/bds-ghast-free.txt` (drill `ghast`, free-flying ghast, no slowness); all
sections down to Other are that run. The held-still run (`bds-ghast.txt`) is the last section.
Times are the capture's first column (ms).
**T** = ghast ticks counted from its MoveEntityDelta stream, T0 = AddEntity (10283). Before the server
stall at 13019, actor tick (UpdateAttributes `tick`) = T + 231; after it the actor counter runs 1 ahead
(player ticks follow client input, not the server). Actor runtime 1, unique ...735; watcher rt 2, ...734.
Actor stands at (0.5, -19, 0.5) on obsidian (top -19). Both streams are identical (both bots watch it).
The `instant_health` heals (health 20 every second, VisibleMobEffects 12 -> 0) are left out below.

**Setup gap:** `/fill ~-20 ~ ~-20 ~20 ~20 ~20 air` failed (`tooManyBlocks` 35301 > 32768), so the air box
was never cleared. One leftover block at (2, -18, 1) was destroyed by the explosion (see Explosion).

## Ghast (rt 3, unique ...733)

- AddEntity 10283 at (0.5, -11, -15.5), velocity (0, -1e-5, 0), health 10, movement 0.03, box 4.02 x 4.
  Flags 21 (CAN_FLY), 22, 48, 49, 50; bit 35 (breathing) added T1, bit 34 (moving) at 10426.
  `spawn` sound at ghast + (0, 2, 0). EntityEvent EntitySpawn on the **actor** (rt 1), data 2686978 =
  0x290002 (41 = ghast type id), 41 ms before the AddEntity.
- Never hurt, never fired again, never came back. Clients drop it (RemoveEntity 28771) at
  (-56.47, 1.62, -14.84), 59.0 blocks out horizontally (chunk -4). Still alive: the final `/kill` hits "Ghast".

## Flight model (fits to 1e-5 where the stream is one delta a tick)

- **Every tick: `v = 0.91 * v + 0.03 * unit(target - pos)`, then `pos += v`.** The 0.03 is the movement
  attribute. No gravity, no y special case, and the push runs every tick (Java pushes 0.1 only every
  2-6 ticks). Speed limit 0.03 / 0.09 = 0.333 a tick. Fastest seen 0.3264 (T42).
- SetEntityMotion carries that tick's v exactly (10731 = (-0.13287944, 0.07816256, 0.13287942) = v at T11).
- Steering starts at T2. The T1 step is only the spawn velocity (y -1e-5).
- **Wander targets are integer block coordinates**, compared against the ghast's feet position:

| leg | from T | target | offset from pos | ends | why |
|-----|--------|--------|-----------------|------|-----|
| 1 | 2 | (-8, -6, -7) | (-8.50, +5.00, +8.50) | T49 | reached (< 1) |
| 2 | ~49 | ? | ? | T59 | not fitted (see below) |
| 3 | 59 | (-20, 0, -11) | (-9.55, +5.23, -5.07) | T99 | reached: 0.74 after the T98 move |
| 4 | 99 | (-28, 3, -20) | (-8.22, +3.24, -9.67) | T130 | **reselected 3.2 short**, after 31 t |
| 5 | 130 | (-28, 6, -23) | (-1.91, +3.47, -5.14) | T149 | reached: ~0.9 after the T148 move |
| 6 | 149 | (-38, 2, -22) | (-9.61, -3.28, +0.59) | ~T190 | reached |

- Offsets are within xz +-10, y +-7 (float_wander's default xz_dist/y_dist), and both up and down. A
  new point is taken the tick after the distance drops below ~1 (must_reach). Leg 4 was cut short at
  31 ticks with no event nearby: random_reselect, or the float_duration roll.
- Legs 1/3/6 ran 47/40/41 ticks. Fit residual: leg 1 <= 3e-5 (all 3 axes), legs 3-6 ~1e-3. Legs 3-6
  start from the observed position and the interval-fitted v, so they carry that start error.
- **Leg 2 (T49-T58) fits no model:** no 1- or 2-target integer path stays under 0.015. Over T51-52 the
  needed acceleration (|R| 0.0929) exceeds the most two 0.03 ticks can give (0.0873). This spans the
  13019 server stall (fireball hit, explosion, fire). Unresolved.
- After T190 the deltas are 6-7 ticks apart and the implied push wanders (0.015-0.05), so targets are
  not resolvable there. It keeps drifting -x, peaks at y 6.06 and ends at y ~1.6.
- **Delta send interval** (`ticks`) grows with distance to the bots: 1 below ~17 blocks, then +1 about
  every 8 blocks (2 at 17.4, 3 at 25.8, 4 at 34.1, 5 at 41.5, 6 at 49.0, 7 at 57.5). Each delta still
  carries the true position.
- Rotation: rot_x (pitch) 19 at the target tick, rising to 36 by T48 while it looks down at the actor,
  and back to 0 (15321) after the target is lost. rot_y/rot_z track the actor: 231 (= -34.7 deg) at
  the shot, matching ghast -> actor.

## Targeting and the shot

- T1 (10285, 1 tick after spawn): TargetEid = actor, FlagsExtended gains bit 24 (same as the shulker).
  Distance 17.9. T2: `mob.warning` at ghast + (0, 2, 0).
- **T21 (+20 t = charge_charged_trigger 1 s):** Flags bit 27 (CHARGED, the open-mouth face) +
  **ChargeAmount 1**, in one batch with the `warn` sound. The `warn` is at the **T19** position +
  (0, 2, 0), 2 ticks stale.
- T40: bit 27 cleared, ChargeAmount 0. **T42 (+41 t, about charge_shoot_trigger 2 s): shot.** One batch:
  AddEntity, `disappeared` (fireball), `undefined` (no entity), `shoot` (ghast). The ghast's own T42
  delta follows it.
- Fireball AddEntity at **ghast + (0, 0.5, 0) + 1.0 * unit(aim xz)**, using the ghast position at the
  start of T42. 0.5 = middle (y + 2) + offset -1.5. Exact: (-5.4850407, -6.644012, -8.123259).
- **Velocity (0.6462309, -1.129361, 0.93109065), |v| = 1.6000 (power 1.6)**, aimed from ghast + (0, 0.5, 0)
  (not from the spawn point) to the **actor centre, feet + 0.9**: the line hits (0.5, -18.1, 0.5) to 1e-6.
- `disappeared` at ghast xz, y + 0.5. `undefined`/`shoot` at ghast + (0, 2, 0).

## Fireball (rt 4, unique ...732)

- Metadata: OwnerEid = ghast, no TargetEid, FireballPowerX/Y/Z 0, box 1.0 x 1.0. Flags at AddEntity:
  5 (INVISIBLE), 48, 49. Same tick, SetEntityData adds bit 0 (ONFIRE). T43: bits 0, 14, 34, 48, 49
  (bit 5 cleared). `reappeared` at T44, at the T43 position + (0, 0.5, 0) (the box centre). This is
  the same disappeared/reappeared pattern as the shulker bullet.
- **Constant velocity: no drag, no acceleration, no gravity** (inertia 1, power vector 0, unlike Java's
  +0.1 * dir and 0.95 drag). It moves on its spawn tick (delta in the same batch, 12333).
- **Ghast-fireball entity push, equal and opposite, horizontal only, 2 ticks:** fireball +(0.0442508, 0,
  0.0343749) at T42 and +(0.0500003, 0, 0.040278) at T43. The ghast gets exactly the negatives one
  tick later (T43, T44: the only deviations from leg 1's straight line). The x terms (0.0443 = 0.05 *
  sqrt(0.783), then 0.05) look like Java's 0.05 push with d = max(|dx|, |dz|), but the dx/dz they imply
  do not match the centre offsets. The push direction is not pinned here (Held still pins it).
- From T44 on the per-tick step is (0.74048, -1.12936, 1.00574) (|v| 1.684), constant through T51.
- **The push makes it miss.** Without it the line passes 0.71 from the actor's centre, inside the
  1.0-box vs 0.6 x 1.8 overlap (half-extents 0.8 / 1.4 / 0.8), so it would very likely hit. With it, the fireball lands 3.12 blocks away.

## Hit and explosion (T52, 13019-13020)

- Hit the **floor**, not the actor: last step clipped to fraction 0.9407 (all 3 axes), so the hit
  point is (2.5663407, -19.0, 2.8399916). 11 moves (T42-T52), 10 ticks after spawn.
- 13019: `undefined` (no entity) + LevelEvent **ParticleExplosion data 1** at the hit point.
- 13020, in this order: fireball SetEntityData Flags + bit 10 (IGNITED), FuseLength 0. Then
  UpdateSubchunkBlocks, then RemoveEntity, then `explode` (no entity) at the hit point.
- UpdateSubchunkBlocks (flags 3, transition Entity): **fire** (2464443264) at (3,-19,1), (3,-19,4),
  (2,-19,3), (1,-19,4), (2,-19,2), all on top of the obsidian within 1.7 of the blast. (2,-18,1) went to
  air (3690217760): a leftover block, destroyed. No obsidian broke.
- Fire burn-out (UpdateBlock to air, flags 3) at +6.85 s (1,-19,4), +18.1 s (2,-19,2), +18.3 s
  (3,-19,1), +20.9 s (2,-19,3), +25.4 s (3,-19,4).
- **Actor untouched**: health stays 20 (only the heal attributes), no HurtAnimation, no SetEntityMotion
  for rt 1, not set on fire. 3.12 blocks from a power-1 blast is outside the 2-block damage radius, and
  no impact damage happened because it never touched the actor.
- The ghast was ~19 blocks away: no impulse from the blast. The leg-2 misfit sits on this tick, though.

## Why one shot, and losing the target

- Target dropped at 15114 (T96): TargetEid 0, FlagsExtended bit 24 cleared. No sound. The ghast is at
  29.1 from the actor's feet. It crossed **28 (max_dist) at T93**, so the drop is ~3 ticks late
  (periodic target check). It went out of range because float_wander ignores the target: legs 3-4
  carried it -x/+y away. Range explains the drop. must_see is unchecked: the failed air fill means
  line of sight is not proven clear, but nothing in the stream points at it.
- The target was held 54 ticks after the shot (T42-T96) and **no second charge started** (no bit 27, no
  ChargeAmount, no `warn`). So the post-shot cooldown is > 54 ticks (Held still: 60). Java would warn
  ~50 ticks after a shot (T92) and fire at 60. No re-target later: it never came back within 28.
- To get more shots: keep it in range (summon closer or tp it back, or a smaller arena), or raise the
  target range.

## Other (noise)

- `block.campfire.crackle` from earlier drills' campfires: (8.5, -39.8, 4.5), (-9.5, -23.8, 6.5),
  (7.5, -18.8, 44.5). Opening `/kill` matched nothing. SyncWorldClocks at 4044, 5132, 18018, 30823, 43622.

## Held still (`bds-ghast.txt`)

Same drill plus `effect @e[type=ghast] slowness 60 255`. **W** = server tick, (ms - 4041) / 50 + 1
(SyncWorldClocks 426001 at 4041, 50 ms a tick). Fills worked: obsidian 1681, air 63 (leftovers), upper
air 0 changed. Actor feet (0.5, -19, 0.5); the watcher (creative, never hurt) stands on the same spot
until shot 9 (both get identical blast impulses). Ghast rt 3 AddEntity W121 at (0.5, -13, -11.5).

- **Ghast:** slowness sets movement to 0.0 (modifier MovementSlowdown -38.4, op 2) at W141. After that
  it never steers: velocity only decays `v *= 0.91` a tick (y settles at -8.302294). Its sole motion is
  the reaction to each fireball push, -z: 1.109 per shot for shots 1-4 (two pushes), 0.554 after (one),
  about 0.05 / 0.09 per push. Target W126 (5 t after spawn), `mob.warning` W127 at the last delta + (0,2,0).
- **Shot cycle, no randomness (+-1 t is ms jitter):** first `warn` + bit 27 + ChargeAmount 1 at target
  +20 (W146); bit 27 / ChargeAmount 0 at warn +19; shot at warn +21; next warn at shot +60. **Period
  81 t**, 10 shots at W167, 248, 329, 410, 491, 572, 653, 734, 815, 896; an 11th warn at W956, then
  `/kill` (W962: TargetEid 0; RemoveEntity W983).
- **Spawn:** ghast + (0, 0.5, 0) + 1.0 * unit(aim xz), every shot. |v| = 1.6000, no spread: each shot
  passes through the actor's reconstructed feet + (0, 0.900, 0) to 1e-3 (actor moved after shots 5, 8, 9).
- **Ghast push = Java `pushAwayFrom`, exact:** d = max(|dx|, |dz|) of ghast -> fireball, push =
  0.05 * (dx, dz) / sqrt(d), divided again by sqrt(d) only when d >= 1. Spawn tick (d 0.993):
  (-0.0059844, 0, 0.0498227). Shots 1-4 get a second one next tick (d >= 1, z exactly 0.05) while the
  1.0 box still overlaps the 4.02 ghast. The ghast takes the negative. Then constant velocity.
- **Every fireball freezes 6 ticks next to the players:** after the step that crosses the players' box
  it sends no moves for 6 t (moving bit 34 off after 2), then makes one more step, and explodes.
  Shots 1, 6, 7, 8 add an xz kick on that step along player -> fireball (SetEntityMotion shows it, y
  untouched): |dv| 1.137, 0.900, 0.708, 0.413. Per-axis it fits about 24 Java pushes (shots 7, 8; 26
  and 20 for 1, 6), so push builds up while frozen. Unresolved.
- **Direct hits: shots 1, 3, 5, 8, 10** (HurtAnimation 3 = projectile) on the last step; `undefined`
  sound at the frozen point. 6 damage (20 -> 14, 19 -> 13); shot 10 does 5 (18 -> 13), 1 t after a
  burn tick: invulnerability takes the difference. Shots 2, 4: the step ends past the box. Shots 6
  (barely), 7, 9 overlap the actor box but hurt nobody: probably the co-located creative watcher took them.
- **Impact order:** W+0 hurt + actor ONFIRE (bit 0) + last move; W+1 ParticleExplosion data 1 at the
  last move position (box bottom, mid-air y -17.7..-18.4, no floor hit), fireball IGNITED + FuseLength 0;
  W+2 block updates, RemoveEntity, `explode`. No knockback on impact (knockback false).
- **Explosion (power 1), fits all 4 actor impulses to 1e-3 and the one damage to 2e-4:**
  w = |feet - E| / 2, a = (1 - w) * exposure (exposure 1 here), impulse = a * unit(feet + 1.62 - E),
  **damage = 7 * (a^2 + a)**, float, no Java "+1", no easy scaling. Shot 9 (|feet - E| 1.829):
  a 0.0855, 0.650 damage (19 -> 18.350132, HurtAnimation 11), knockback first: SetEntityMotion
  (-0.0477, **0.4**, 0.3971) = 0.4 horizontal away from the **ghast** (the owner), y set to 0.4, then
  + impulse (0.00946, 0.0319, -0.0788). Shots 5, 8, 10: a 0.211, 0.277, 0.179 (1.79, 2.48, 1.48
  damage) lost to the impact's invulnerability, impulse still applied. Others were > 2 from the feet.
- Player body: ground v.y -0.0784; knocked velocity sums with ground friction 0.546 (shot 5 moved it
  (0.052, 0, -0.434) to (0.552, 0.066), confirmed by shot 8's impulse). `land` sound after shot 8.
- **Fire:** 0-6 fires a blast on air over obsidian within ~2 (shot 3 none); blasts clear older fires;
  the blast list also sends air -> air at y -18/-17. No obsidian broke. Burning: 1 hp (HurtAnimation 7)
  every 20 t from ignition +20, bit 0 cleared at +99 (shots 1, 3). From shot 5 it stands on fire
  cells: burns every 10 t, a few HurtAnimation 6 (in fire), never goes out before the end.
