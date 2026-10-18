# bds-witch findings

Capture: `research/captures/bds-witch.txt` (drill `witch`). Times are the capture's first column (ms);
ticks come from the actor's SetEntityMotion/UpdateAttributes/MobEffect `tick` fields (~51 ms a
tick in this run). Actor runtime 1, unique -562640715775. Easy, midnight, stone floor. The
`instant_health` heals (effect 6 Add/Remove, health back to 20 each second) are left out below.

Actor position: no packet carries it. Start (12.4334, -19, 20.5) (witch 1 was summoned at
(12.4334, -19, 14.5) = actor z-6). At witch 2's summon it was at (12.3441, -19, 26.7064), so the
three harming knockbacks pushed it 6.2 in +z. Positions at throws 3-5 below assume every break
sits 0.35 short of the actor's centre in z, as breaks 1-2 do exactly (gives 1.92, 1.97, 2.32 per
knockback; not checked independently).

## Witch 1 (rt 3), 6 off

- AddEntity 9127 at (12.4334, -19, 14.5), health 26, movement 0.25. `spawn` sound 9127.
- TargetEid = actor at 9138, the spawn tick (11 ms later). Same packet sets FlagsExtended bit 24
  (FACING_TARGET_TO_RANGE_ATTACK). `mob.warning` 9194 (next tick).
- Walked straight at the actor from 9292 (3 ticks after spawn) to 10088: steps 0.0104, 0.0136,
  0.0227, 0.0344, 0.0487, 0.0657, 0.0854, 0.1077, 0.1199, 0.1267, 0.1304, 0.1325, 0.1336, 0.1342,
  0.1345, 0.1346, 0.1348 (= polar bear's 0.1349, speed multiplier 1.0), then input stops and it
  slides 0.0736, 0.0402, 0.0219, 0.0120 ... (x0.546 ground friction). Input stopped ~19 ticks
  after taking the target (Java RangedAttackGoal stops at seeTime >= 20 inside the radius).
  Stopped at (12.4947, -19, 16.2277): 4.27 from the actor (horizontal), 1.73 walked.
  Flag bit 34 (MOVING) on 9491-10294.
- Stood there for all 5 throws. Lost the target at 25194 (TargetEid 0, bit 24 cleared), 6 ticks
  after the 5th harming knockback took the actor to z 26.7 (10.47 off, past attack_radius 10).
  From 28594 it random-strolled away (-z) at 0.1349 a tick.
- No MobEquipment until death (empty mainhand/offhand at 30053), no drinking, no USINGITEM.
  EntityEvent WitchSpellParticles at 14589 (no other event near it).
- Splashed by its own potions (break 3.92 off horizontally, 4.13 from its feet):
  - Poison from throw 1: witch VisibleMobEffects 38 (poison) 12439-17294. Poison damage 0.15
    each (26 -> 25.85 -> 25.7 -> 25.55 at 13544, 14787, 16038, 25 ticks apart; 85% magic
    reduction). Effect lasted ~97-98 ticks (last damage 25 ticks before removal), i.e. 0.108 of
    900. Java's 1 - d/4 needs d = 3.57; distance to its feet is 4.13 (Java: no effect), to its box
    3.62 (would be 86 ticks). Scaling for non-hit entities not pinned by this.
  - Harming from throw 2 (15444): HurtAnimation, SetEntityMotion (0, 0.4, 0) (vertical only), no
    health change. It hopped (y -18.6 ... -17.85 ... -19 at 15995).

## Witch 2 (rt 10), 12 off

- AddEntity 34093 at (12.3441, -19, 14.7064); actor 12.0 off. Never took a target (no TargetEid,
  no bit 24, no `mob.warning`), no throws. 12 is beyond its 10-block targeting.
- Head turns only, then random strolls at 43644 (to (8.93, 11.59), peak step 0.1348) and 54243.
  WitchSpellParticles at 44437 while strolling. Killed 55081 (empty equipment).

## Throws (all witch 1, from (12.4947, -19, 16.2277))

| # | ms | tick | after target | after prev | Aux | potion | actor z | flight | break |
|---|----|------|--------------|------------|-----|--------|---------|--------|-------|
| 1 | 12244 | ~295 | 3106 ms (~61 t) | - | 25 | poison | 20.50 | 3 t | 12392 (12.4386, -17.6837, 20.1500) |
| 2 | 15294 | 354 | | 3050 ms (59-60 t) | 23 | harming | 20.50 | 3 t | 15444 same point |
| 3 | 18338 | 414 | | 3044 ms (60 t) | 23 | harming | ~22.42 | 6 t | 18644 (12.4112, -17.5364, 22.0688) |
| 4 | 21386 | 473 | | 3048 ms (59 t) | 17 | slowness | ~24.39 | 9 t | 21837 (12.3831, -17.3749, 24.0376) |
| 5 | 24439 | 533 | | 3053 ms (60 t) | 23 | harming | ~24.39 | 9 t | 24895 same point as 4 |

- Choices fit the pack rule: 1 poison (health 20, not poisoned, 4.27 < 8); 2-3 harming (poisoned,
  > 3 so no weakness roll); 4 slowness (8.16 >= 8, not slowed); 5 harming (slowed and poisoned).
- Interval 3 s (attack_interval). First throw ~3.1 s after taking the target: the timer starts
  full.
- Spawn (AddEntity position) is always (12.4782, -17.585, 17.3819): 1.415 above the witch's feet,
  1.15 horizontally along the aim (dx -0.0165, dz +1.1499). Sound positions add 0.125 (box centre):
  the `disappeared` sound at (12.4925, -17.46, 16.3820) is 0.15 along the aim, 1.54 up, i.e. the
  spawn point before a further 1.0 forward.
- Velocity |v| = 0.75000 every time (shooter power 0.75). Yaw 0.818 deg west of +z (aims at the
  actor's x). Elevation (AddEntity pitch byte is quantised: 2.8125, 11.25, 18.28):
  - 1, 2: (-0.010695, 0.043837, 0.748641), 3.351 deg, horizontal witch-actor 4.27
  - 3: (-0.010499, 0.149279, 0.734919), 11.481 deg, ~6.19
  - 4, 5: (-0.010114, 0.247312, 0.707979), 19.254 deg, ~8.16
  Not Java's witch formula (eye-1.1, +0.2 d gives -0.6/3.2/5.2 deg). A continuous ballistic
  low-arc solve with v 0.75, g 0.05 fits to ~0.1 deg (aim 0.79 below the spawn, d +0.66), so the
  angle grows with distance; exact formula not found. angle_offset -20 not visible.
- Motion: first MED (same tick as AddEntity) = spawn + v0. Then each tick v = v x 0.99, vy -= 0.05,
  move by the new v (throw 1: v1 = (-0.010588, -0.006601, 0.741155) in the SetEntityMotion sent the
  spawn tick; steps dz 0.74116, 0.73374; dy -0.0066, -0.0565). Throw 3 SetEntityMotion at t4 =
  v0 x 0.99^5 in xz.
- Every throw broke on the actor (direct hit): break z = actor z - 0.35 (0.05 outside the 0.6 box),
  height above feet 1.316 (1-2), 1.464 (3), 1.625 (4-5); distance to feet 1.36 / 1.50 / 1.66.
  Breaks are interpolated mid-step (4: between z 23.505 and the next step).
- Break packets, same tick: LevelEvent ParticleSplash with the potion colour (data ARGB: poison
  -7888029 = 0xFF87A363, harming -5675670 = 0xFFA9656A, slowness -7622688 = 0xFF8BAFE0), then 5x
  ParticleProjectileHit data 37421070 (0x023B000E) at the same point. Next tick: RemoveEntity and
  LevelSoundEvent `glass` at the break point.
- Splash potion flags: 844424930132000 on AddEntity, 844442110017536 from the next tick.

## Effects on the actor

- 1 poison: MobEffect Add id 19 amp 0 duration 900 (full drinkable) at the break tick (12392).
  First poison damage on the next tick (HurtAnimation 12423, 20 -> 19 at tick 298), then 1 every
  25 ticks (323, 348, 373, 398, ...): damage when remaining % 25 == 0, 900 included, as Java. At
  /effect clear (tick 663) remaining 534 = 900 - 366, applied tick 297. No SetEntityMotion. The
  splash hit itself does no damage (no extra drop at break 4, tick 482).
- 2, 3, 5 harming: health 20 -> 14 (6 = full Harming I), HurtAnimation, SetEntityMotion
  (-0.00575, 0.4, 0.39996) at the break tick: 0.4 horizontal along the potion's flight/witch->actor
  direction, 0.4 up. Health drops land 21-28 ms after the break, before the next heal.
- 4 slowness: MobEffect Add id 2 amp 0 duration 1800 (full) tick 482; movement 0.1 -> 0.085
  (MovementSlowdown -0.15, operation 2). Remaining 1619 at the clear (1800 - 181). No motion.
- All durations/amounts are full, so a direct hit scales 1.0 (as Java's hit-entity rule). Java's
  1 - d/4 from the feet would have given 0.66 / 0.62 / 0.58. Off-hit scaling: only the witch's own
  ~97-tick poison above.
- VisibleMobEffects packs (id << 1 | ambient) 7 bits each: 38 poison, 550 slowness+poison, 12
  instant health.

## Witch sounds

- `spawn` 9127 (witch 1), 34112 (witch 2). `mob.warning` 9194, one tick after taking the target.
- Each throw (same tick as AddEntity): `throw` from the witch at its feet +0.95, an entity-less
  `undefined` at the same point, `disappeared` (entity_type splash_potion) at the pre-offset spawn,
  and 2 ticks later `reappeared` from the potion at its position +0.125. The `undefined` /
  `disappeared` / `reappeared` names are likely an off-by-something sound-id table (cf. the flags
  table), not real events.
- No ambient, hurt or death sounds from either witch in the capture.
