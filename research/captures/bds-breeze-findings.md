# bds-breeze findings

Capture: `research/captures/bds-breeze.txt` (drill `breeze`). Times are the capture's first column (ms);
intervals are in ticks (all of them are multiples of ~50 ms). Actor runtime 1, unique -609885356031; watcher
runtime 2, unique ...030; breeze runtime 3, unique ...029. The actor starts at (0.5, -19, 0.5) on the stone
floor top -19, the breeze at (0.5, -19, -7.5). Easy. The `instant_health` heals are left out below.

**The air fill failed**: `fill ~-20 ~ ~-20 ~20 ~20 ~20 air` returned `commands.fill.tooManyBlocks` (35301 >
32768), so earlier drills' blocks stayed above the floor. Evidence of one near x = -7: shot 11 hit a block
face at x = -7.0, and the actor landed on a non-stone block (watcher-stream `land`, extra_data 927668178, not
the stone's -2144268767) with feet at y -16.0 near (-6.2, 4.9). Split the fill in two (y 0-9 and 10-20).

**The watcher stands where the actor does and its server body blocks charges.** Shots 1 and 3 hit the watcher
(no damage, no impact knock, it is probably not in survival), and every burst pushes both bodies.

## Actor position (server-simulated, from the burst pushes)

The burst push formula below inverts exactly to feet y -19.000 on every ground case (8 actor, 5 watcher),
so the inverted positions are reliable. Actor feet: (0.5, 0.5) at burst 1 (known start), (0.499, 1.501) at
t307, (0.521, -2.134) t340, (0.526, -2.789) t395, (0.02, 0.025) t445, (-1.333, 1.903) t498, (-1.822, 4.678)
t540, (-4.758, 5.408) t583, (-6.23, **-16.0**, 4.92) t637 (on the leftover block; it fell back to the floor
at 30943, `fall` at (-4.54, -18.9, 0.97)). Ticks t### are the actor's packet `tick` fields.

## Target and states

- AddEntity 9043: health 30, movement 0.4, box 0.6 x 1.77, Flags bits 19, 22, 48, 49 (+35 breathing next
  tick). `spawn` sound at breeze + (0, 1.239, 0). No entity properties ever (ints/floats empty).
- Target 9343 (6 ticks after spawn): TargetEid = actor, and FlagsExtended bit 24 (flag 88,
  FACING_TARGET_TO_RANGE_ATTACK) set in the same SetEntityData. `mob.warning` 1 tick later at breeze +
  (0, 1.239, 0).
- FlagsExtended is `bit63 | bit25` at rest (bit 25 = flag 89, always set). The only two that change:
  - **bit 24 (88) = fire_at_target running**: on 16 ticks before every shot, off 4 ticks after it.
  - **bit 39 (103, JUMP_GOAL_JUMP) = jump_around_target running**: on at the same packet that clears bit 24
    (shot + 4 ticks) when a jump is chosen, off 1 tick after touchdown.
- Flags change only bit 34 (moving). No charging/inhaling/sliding flag, no EntityEvent apart from
  DustParticles (rt 3) on a high landing and DeathAnimation at the /kill.
- AmbientSoundEventName: "ambient.in.air" at 21140 (last airborne tick of jump 4), "ambient" at 29238.
- Target lost 33739: TargetEid 0, 6 ticks after shot 11 burst on the block between them (breeze x -9.5,
  actor ~x -4.5, 7 apart): must_see lost. It then strolls at ~0.344/tick (37540-39641).

## Shot cycle (exact)

`bit 24 on -> 16 ticks -> shot -> 4 ticks -> bit 24 off` (pre_shoot 0.75 s + 1 tick, post_shoot 0.2 s).
Then either a jump (bit 39 on, see Jumps) or a walk/wait; bit 24 comes back on 1 tick after bit 39 clears, or
after 10 ticks with no walk (attack_cooldown 0.5 s), or when the walk ends (22, 30, 15, 31 ticks).

| # | rt | spawn ms | gap t | breeze->actor | moves to contact | contact | burst | damage |
|---|----|----------|-------|---------------|------------------|---------|-------|--------|
| 1 | 4 | 10141 | - | 8.00 | 11 | watcher | 10941 (t269) | none |
| 2 | 5 | 12493 | 47 (jump) | 1.55 | 2 | actor | 12843 (t307) | 20 -> 19 |
| 3 | 9 | 13990 | 30 (wait) | 6.85 | 6 | watcher (0.485, 0.629) | 14538 (t340) | none |
| 4 | 10 | 16739 | 55 (jump) | 4.91 | 7 | actor | 17341 (t395) | 20 -> 19 |
| 5 | 11 | 19288 | 51 (jump) | 4.84 | 7 | actor | 19893 (t445) | 20 -> 19 |
| 6 | 12 | 22093 | 56 (jump) | 3.55 | 5 | actor | 22593 (t498) | 20 -> 19 |
| 7 | 13 | 24193 | 42 (walk) | 4.27 | 6 | actor | 24748 (t540) | 20 -> 19 |
| 8 | 14 | 26442 | 45 (jump) | 3.23 | 5 | actor | 26943 (t583) | 20 -> 19 |
| 9 | 15 | 28940 | 50 (walk) | 7.19 | 10 | actor (on the block) | 29691 (t637) | 20 -> 19 |
| 10 | 16 | 30691 | 35 (walk) | ? | never | flew off, gravity 0 | - | - |
| 11 | 17 | 33241 | 51 (walk) | ? | 4 | block face x = -7.0 | 33441 | - |

Gaps decompose exactly: with a jump `4 + 10 + flight + 2 + 16` (flights 15, 23, 19, 24, 13), without one
`4 + wait + 16`. The breeze always stands on the floor (y -19) when it shoots.

## Wind charge

- AddEntity at **breeze feet + (0, 1.416, 0)**, same x/z (residuals <= 0.006 = breeze motion that tick).
  Flags bits 5 (INVISIBLE), 48, 49; box 0.3125; OwnerEid = breeze; LimitedLife -1. Next tick: +14, +34, -5.
  AddEntity pitch/yaw are Java-projectile style (yaw = atan2(vx, vz), pitch = atan2(vy, h), down negative),
  truncated toward zero to 1.40625 deg steps.
- **Speed 0.694-0.706 a tick, constant: no gravity, no drag** (every step equals the AddEntity velocity to
  1e-5). The first MoveEntityDelta is in the AddEntity batch (moves on its spawn tick).
- Aim (against the inverted actor feet): yaw error <= 0.49 deg, line height at the actor = **feet + 0.86 to
  0.95** (shot 1 0.81; shot 3 1.44 with the actor airborne), i.e. about the box centre, not feet + 0.5. Spread
  and speed spread fit Java's `shoot(..., 0.7, uncertainty 1)` (0.0075 gaussian) on easy.
- Contact = first tick whose moved box overlaps the body (no ray cut: shot 1 stops at z 0.178 with the body
  at 0.2-0.8). It then stops (no more moves, bit 34 cleared 2 ticks later) and **the impact and the burst
  both come 6 ticks after the contact tick**, in one batch, at the stopped position. Same 6 for the watcher.
- Block hit (shot 11): no delay, burst on the hit tick. The centre ray meets the face x = -7.0 at 0.857 of
  the step; the burst is that point with x pulled back 0.25 (-7.25); `undefined` at the face point.
- Removal: RemoveEntity + `breeze_wind_charge.burst` (entity_type "", unique -1, at the burst point) 1 tick
  after the burst. Shot 10 (aimed up at 0.277/tick) flew straight on for 5.2 s, left view (RemoveEntity
  35941 at (28.2, 11.6, -48.4), updates every 2-4 ticks when far) and was still alive at the /kill
  ("Breeze, Ghast Tear, Gunpowder, Wind Charge").

## Hits and pushes

- **Damage 1** (easy), HurtAnimation data 3, health UpdateAttributes 20-40 ms after. Burst damage: none.
- Hit batch order: SetEntityMotion (impact), HurtAnimation, the watcher's push, SetEntityMotion (impact +
  burst), LevelEvent ParticleBreezeWindExplosion data 3 (at the charge).
- **Impact knock replaces the motion: y 0.4, xz = 0.4 * unit(actor - breeze)** horizontally, with the
  breeze's position that tick (all 7 hits match to 1e-4; it is not the charge's direction).
- **Burst push is added: `p = 0.6 * (1 - d / 6) * unit(feet + (0, 1.62, 0) - c)`**, d = |feet - c| (feet,
  not eye), radius 6 = 2 * power 3, knockback_scaling 0.6, exposure 1 here. Burst 1 with the known start
  (0.5, -19, 0.5): d 0.892, p = (-0.00028, 0.47269, 0.19348) = MOT (-0.00028, 0.39429, 0.19348) minus the
  resting -0.0784. The second MOT minus the first gives it exactly on a hit.

| burst | body | d | push | burst | body | d | push |
|-------|------|---|------|-------|------|---|------|
| t269 | both | 0.892 | 0.5108 | t445 | actor | 0.889 | 0.5111 |
| t307 | both | 1.005 | 0.4995 | t445 | watcher | 2.098 | 0.3902 |
| t340 | actor | 3.021 | 0.2979 | t498 | actor | 0.911 | 0.5089 |
| t340 | watcher | 1.437 | 0.4563 | t498 | watcher | 2.363 | 0.3637 |
| t395 | actor | 0.870 | 0.5130 | t540 | actor | 0.945 | 0.5055 |
| t395 | watcher | 4.619 | 0.1381 | t540 | watcher | 3.261 | 0.2739 |

- A hit's total vy is 0.83-0.91 (0.4 + 0.43-0.51). Actor Flags bit 34 1-2 ticks after a knock.
- The breeze is pushed by its own bursts too (e.g. t307, 1.6 away: +z 0.457).
- 17440: the actor's SetEntityData carried FlagsExtended bits 25, 45 and ArrowShooterId 0, once (2 ticks
  after hit 4).

## Movement

- Walk (move_around_target): ground recursion `v' = 0.546 v + 0.2257`, steady **0.497/tick** (31940-32397,
  straight); ~0.47-0.48 on curved paths. Starts with a slow ramp (0.002, 0.010, 0.026, 0.050, 0.083 ...).
  Walk ends 4.3, 7.4, 4.7 from the actor (inside 4-8). No walk after shot 2 (1.55 off, only its own burst moved it).
- Jump chosen after shots 1, 3, 4, 5, 7 (breeze->actor at takeoff 8.0, 7.2, 5.1, 5.0, 4.6); not after shots 2, 6, 8
  (1.55, 3.55, 3.23: under valid 4). Shots 9 and 10 walked at 7.2 and ~5 (the leftover block was between them, probably).
- **Prepare: bit 39 on -> exactly 10 ticks -> launch SetEntityMotion**; `jump` sound (at the takeoff point,
  y -19) and the first airborne move 1 tick after the launch.

| jump | launch ms | v (x, y, z) | h | vy | angle | flight t | takeoff -> land dist | land -> actor | apex |
|------|-----------|-------------|---|----|-------|----------|-------|------|------|
| 1 | 10838 | (0, 0.5013, 0.5975) | 0.5975 | 0.5013 | 40.00 | 15 | 8.96 | 0.96 | 2.09 |
| 2 | 14693 | (0.0356, 0.8410, -0.4843) | 0.4856 | 0.8410 | 60.00 | 23 | 11.17 | ~4.0 | 5.12 |
| 3 | 17440 | (0.0518, 0.6897, 0.1774) | 0.1848 | 0.6897 | 75.00 | 19 | 3.51 | ~2.5 | 3.59 |
| 4 | 19990 | (-0.1264, 0.8917, 0.0936) | 0.1573 | 0.8917 | 80.00 | 24 | 3.78 | ~2.1 | 5.69 |
| 5 | 24892 | (-0.3109, 0.4446, 0.0167) | 0.3114 | 0.4446 | 55.00 | 13 | 4.02 | ~1.4 | 1.73 |

- Angles are exactly the pack's 40/55/60/75/80; |v| 0.54-0.97 (<= 1.4).
- **Flight: horizontal constant (no drag), vy -= 0.08 a tick (no drag)**, plus **+0.2655 on the first
  airborne tick only** (first dy = vy + 0.2655 on all 5; then vy - 0.08, vy - 0.16 ...). Distance = ticks * h
  exactly. Jump 5 alone was deflected mid-air (MOT 25041, 25143: h 0.311 -> 0.262 -> 0.328, ~35 deg turn)
  passing 0.6 from the watcher: entity push.
- Landings are **0.96-4 from the actor, not 4-8**: jumps go at or over the target (jump 1 straight through
  it). The landing rule is not pinned down.
- Touchdown: y -19 and on_ground on the landing move, bit 39 off 1 tick later, bit 24 on the tick after.
  Jumps 2-4 (apex >= 3.59) also give DustParticles on the touchdown tick and a breeze `land` 1 tick after at
  the last airborne position; jumps 1 and 5 (apex 2.09, 1.73) give neither.
- Body/head yaw (MED rot_y/rot_z) turns 21-22 units (~30 deg) a tick toward the target while bit 24 is on.

## Sounds

- `shoot` (breeze) and `undefined` ("", -1) at breeze + (0, 0.885, 0) on the spawn tick, one each per shot.
- `disappeared` at breeze + (0, 0.9, 0) on spawn (bit 5 set) and `reappeared` 2 ticks later at the charge +
  (0, 0.15625, 0): all 11 pairs per stream are the charges' (as with shulker bullets).
- `undefined` ("", -1) with the burst sound on the 7 actor hits and the block hit, **not** on the 2 watcher
  hits.
- Player `land`/`fall`: each stream gets only the other player's (actor stream: the watcher's `land`; watcher
  stream: the actor's `fall` after hits and `land` on the block), extra_data = the landed-on block. Fall
  damage was off; health never dropped from them.

## Other (noise)

- Ghast rt 6 at (-56.2, 1.87, -16.4), 13142: added already dead (health 0, DeathAnimation) with its drops
  (rt 7, 8: ghast tear, gunpowder); RemoveEntity 14192. Left over from an earlier drill; the drops survived
  to the /kill.
- 61 `block.campfire.crackle` per stream.
