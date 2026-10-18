# bds-ender findings

Captures (drill `ender`): run 3 `research/captures/bds-ender.txt` (dry floor), run 2
`research/captures/bds-ender-wet.txt` (wet ground); run 1's capture was overwritten. Times are the
capture's first column (ms); ticks are the actor's UpdateAttributes/SetEntityMotion `tick` (~51 ms a
tick of capture time, so 1000 ms reads as 19-20 ticks). Actor runtime 1. Easy, midnight. The
`instant_health` heals (health back to 20 each second) are left out below.

Flag bits used: 19 CAN_CLIMB, 22 CAN_WALK, 25 ANGRY, 34 MOVING, 35 BREATHING, 48 HAS_COLLISION,
49 HAS_GRAVITY. HurtAnimation data is the damage cause: 2 entity attack, 9 drowning (water).

## Run 3 (dry floor)

Actor at (6.43, -60, 0.50) on a dry 41x41 stone floor (y -61) with air 6 high. Enderman (rt 6)
summoned 2 off at 9040; it teleported on its first tick (9041) 11.35 to (5.3296, -60, 13.7979) (no
water this time, so the spawn teleport is its own rule). Slowness 255 at 10032. The actor was tp'd to
1.5 +z of it, (5.33, -60, 15.30), and `!hit` reported Ok at 13193. No water, no splash anywhere.

### Hit -> target -> anger -> teleport

- 13214: HurtAnimation data 2 on the enderman. 13235: health 40 -> 39, **TargetEid = actor**, and
  the knockback. MED step (0, +0.4, -0.4), then SetEntityMotion (0, 0.3136, -0.2184), the next tick of
  that flight: 0.4 / 0.4 straight away from the actor, worked out from where it stood (no teleport
  yet).
- 13280 (next tick): ANGRY (bit 25), `mob.warning`, movement re-sent 0.0.
- 13331 (2 ticks after the damage): teleport **mid-knockback** from (5.3296, -59.0575, 12.9808)
  (airborne) to (18.8406, -60, -2.5379), 20.6 away, from 2.3 to 22.1 off the actor.
- Run 2's hit-teleport came on the damage tick, before the knockback. So the hit-teleport lands 0-2
  ticks after the damage (two samples), not at a fixed point.
- Slowness cleared 14235 -> movement 0.45. TargetEid and ANGRY then held to the kill (45281), 32 s
  (~640 ticks), with no release.

### Teleports while angry

Actor positions are inferred: the actor stays put until the first hit, then each 0.4 knockback
carries it ~2.5 along the knockback direction. So "from actor" is good to ~1.

| ms | tick | from | to | dist | from actor | toward? |
|----|------|------|----|------|------------|---------|
| 13331 | ~343 | (5.33, -59.06, 12.98) | (18.84, -60, -2.54) | 20.6 | 2.3 -> 22.1 | no (the hit) |
| 14832 | ~372 | (17.24, -60, -1.24) | (3.91, -60, 13.75) | 20.06 | 20.4 -> 2.1 | **yes** |
| 21278 | ~496 | (1.57, -60, 22.99) | (-13.71, -60, 28.33) | 16.19 | ~2.5 -> ~13.7 | no |
| 39032 | ~829 | (24.16, -60, 7.50) | (38.83, -60, 16.34) | 17.12 | ~1.5 -> ~15.8 | no |

- 14832 came **1501 ms (30 ticks)** after the hit-teleport left it 22.1 off, while it was still
  ~20 off (it had only started walking when slowness ended). It landed 2.1 from the actor. Java's
  `teleportTowards` fits: a point 16 along the line to the target ((7.9, 11.7) from (17.24, -1.24))
  plus up to +-4 jitter per axis (actual offset (-4.0, +2.0)).
- So the rule is Java's: **target over 16 off -> after 30 ticks, teleport toward it**. It fits both
  runs. Run 3's other landings (13.7, 15.8 off) were under 16 and it ran in instead (38 and 51 ticks
  to the next hit). Run 2's landings over 16 (20.1, 23.0) each ran back under 16 within ~20 ticks (estimated from its 0.437 step).
- 21278 and 39032 jumped from right next to the actor, ~14-16 away, with no hurt and no cause in the
  capture. Java's other branch (target looking at it from under 4 blocks -> random teleport) would
  give exactly this if the bot was looking at its attacker at those moments; the actor's rotation
  isn't captured. Rate: 2 in ~640 angry ticks here (run 2 had 3 in ~430, some from 8-11 off, which
  Java's under-4 branch doesn't allow).
- Packets as before: forced MoveEntityDelta, LevelEvent SoundEndermanTeleport at origin and
  destination, SetEntityMotion (0, 0, 0) when moving.

### Hits on the actor (25)

- Damage **4.5** every hit. Knockback SetEntityMotion horizontal 0.4000-0.4003 away from the
  enderman, vy 0.4. HurtAnimation data 2 on the actor and ArmSwing on the enderman the same tick;
  `attack` sound (feet +1.45) the next tick.
- **Cooldown 20 ticks**: consecutive hits with no teleport between are 1000 ms apart (952-1046 ms;
  the tick field reads 19 or 20), 19 times. 20 hits ran back to back (16079-38175, broken only by the
  21278 teleport), the enderman following the actor as each knockback pushed it ~2.5 away. One gap of
  1350 ms (25 t) is a catch-up after a long knockback.
- First hit after arrival:
  - 21278 -> 23177 (2745 ms after the previous hit): landed 13.7 off, ran 38 ticks, hit on contact
    while still at full step (0.42).
  - 39032 -> 41576: 15.8 off, 51 ticks, hit on contact at 0.4371.
  - Both immediate: the cooldown had run out on the way.
  - After the toward-teleport (14832, 2.1 off) it circled for 25 ticks before the first hit at
    16079 (56 ticks after taking the target).
- No hit missed; health back to 20 from the heal before each one.

## Run 2 (wet ground, `bds-ender-wet.txt`)

Actor tp'd to the flat ground (6.43, -60, 0.50). **The ground has water cells**: both endermen
splashed and took 1 drowning damage on spawn, the actor splashed at its first stare tp and at every
knockback landing, and enderman 2 was water-hurt 7 times. Enderman 1 walked ~26 s without touching
water (its stroll avoids it). The actor stood in the water around (-6, -12) for all of phase 2.

### Enderman 1 (rt 4) and the stare

- AddEntity 8023 at (6.4334, -60, -7.5), health 40, movement 0.3. Same tick: `splash` (feet +1.45),
  `spawn` (feet +2.465), HurtAnimation data 9, health 39, and a teleport 16.99 to
  (-6.0815, -60, -18.9837). No other teleport for the 26 s it lived.
- Six stares, `tp @p ~ ~ ~6 facing ~ ~2.55 ~`, each putting the actor 6.0 due +z:

  | ms | tick | actor at | enderman |
  |----|------|----------|----------|
  | 12020 | ~291 | (-7.79, -60, -7.57) | walking, 0.19/t |
  | 14027 | ~330 | (-6.52, -60, -5.89) | starting a walk |
  | 16024 | ~369 | (-2.86, -60, -9.50) | standing at (-2.8595, -15.4952) |
  | 18031, 20030, 22030 | ~408, 448, 487 | (-2.86, -60, -9.50) | same spot, still |

- **No reaction to any stare**: no TargetEid, no ANGRY, no `mob.warning`/stare/scream sound, no
  teleport, through the six stares and the 12 s after them (to the kill at 34050, tick ~765). It
  strolled away at 22526 and 31525.
- From 18077 to 22081 its head (MED rot_z) stayed at 253/0/253/0 (yaw 0 = due +z, at the actor),
  pitch byte 7 (9.8 deg down; the actor's eyes are 8.8 deg down at 6 off). So it was looking at the
  actor (look-at-player) and still didn't anger.
- Whether the bot held the look: no packet carries the actor's rotation. After each tp the actor's
  MOVING flag went on 2-6 ticks later and off ~6 ticks after that, and its exhaustion rose every tick
  from 12125 to ~25000 (it was drifting in the water). Nothing shows the look was held.

### Enderman 2 (rt 5): the hit

- AddEntity 37080 at (-2.3596, -60, -7.4915), 2 off. Same tick: splash, HurtAnimation 9, health 39,
  teleport 16.37 to (4.2112, -22.4677); 7 ticks later (37431) a second one, 12.99 to
  (-5.5201, -13.8598). Slowness 255 at 38077 (movement 0.0).
- Actor tp'd to (-5.52, -60, -12.36) (1.5 off) at 40061; `!hit` reported Ok at 41487. Then:
  - 41553 (next tick): teleport from (-5.5201, -13.8598) to (5.9252, -18.2198), 12.25 away (13.3
    from the actor), with HurtAnimation data 2.
  - 41574 (same tick): health 39 -> 38 (a 1-damage punch), **TargetEid = actor**, SetEntityMotion
    (0.35605, 0.4, -0.18230): 0.4 horizontal, 0.4 up. The direction is from the actor to the
    **post-teleport** position ((11.45, -5.86) normalised), so the knockback is worked out after the
    teleport.
  - 41632 (tick after): ANGRY (bit 25) set, `mob.warning` at feet +2.465, movement re-sent as 0.0.
  - The knockback flight: steps 0.4, 0.2184 (x0.546, first tick on the ground), then x0.91 in the
    air; vy 0.4, 0.3136, 0.2289, ... ((vy - 0.08) x 0.98); landed at 42130.
  - Slowness cleared at 42531: movement **0.45**. Chase began 42680.
- No stare/scream sound at any point. `attack.strong` (the actor's punch) only reached the watcher,
  at (-5.52, -59.1, -12.36).

### Angry movement

- Steady chase step **0.4371** a tick (= 2.1586 x 0.45^2 = 0.43712), e.g. 43523-43831, 49031-50480,
  57330-58124. Corners dip to 0.41-0.42.
- From standing (42680, 62082): 0.0249, 0.0441, 0.0737, 0.1117, 0.1582, 0.2134, 0.2773, 0.3455,
  0.3855, 0.4081, 0.4201, 0.4272, 0.4315, 0.4340 ... 0.4371 (same 9-10 tick ramp as the stroll).
  After a teleport made while running (47431, 51723, 56028) it skipped the ramp: 0.1984, 0.3068,
  0.3660, 0.3983 ... (x0.546 approach from the first tick).
- Every return from the west took the same route, via (-12.7, -16.3), (-11.1, -16.65),
  (-9.4, -15.6), (-7.7, -13.7), into the actor from the south-west (the path goes round water).

### Hits on the actor

| # | ms | tick | gap | enderman at | SetEntityMotion (actor) | health |
|---|----|------|-----|-------------|-------------------------|--------|
| 1 | 44925 | 936 | - | (-6.40, -60, -13.35) | (0.26626, 0.4, 0.29850) | 20 -> 15.5 |
| 2 | 51229 | 1059 | 123 t | (-7.66, -60, -13.66) | (0.27808, 0.4, 0.28753) | 20 -> 15.5 |
| 3 | 54175 | 1117 | 58 t | (-7.14, -60, -13.12) | (0.19281, 0.4, 0.35047) | 20 -> 15.5 |
| 4 | 61730 | 1265 | 148 t | (-6.76, -60, -12.75) | (0.08075, 0.4, 0.39176) | 20 -> 15.5 |

- Damage **4.5** each (7 on normal -> easy 7/2 + 1).
- Same tick: EntityEvent ArmSwing on the enderman, SetEntityMotion on the actor (horizontal exactly
  0.400 away from the enderman, vy 0.4), HurtAnimation data 2 on the actor; health packet 16-21 ms
  later. Next tick: LevelSoundEvent `attack` from the enderman at feet +1.45, and the actor's
  `splash` as it lands back in the water.
- Every hit was the first one after the enderman came back from a teleport; there were never two hits
  without a teleport between them, so the gaps (58-148 ticks) are trip times and the attack cooldown
  is not measured (it is <= 58 ticks). After hit 1 it stood next to the actor ((-5.70, -12.36),
  45.1-47.0 s, ~38 ticks) without hitting again, head pitched down to byte 44 (62 deg).

### Teleports while angry

| ms | tick | from | to | dist | from actor | cause |
|----|------|------|----|------|------------|-------|
| 41553 | 869 | (-5.52, -13.86) | (5.93, -18.22) | 12.25 | 1.9 -> 13.3 | hit by the actor |
| 43926 | 916 | (-1.00, -18.20) | (-9.33, -19.53) | 8.43 | 7.8 -> 8.3 | none seen (running) |
| 47380 | 985 | (-5.53, -12.03) | (-2.12, -16.95) | 6.00 | 0.3 -> 6.2 | water, 2455 ms after hit 1 |
| 47931 | 996 | (-4.82, -14.21) | (-16.63, -0.49) | 18.11 | 2.4 -> 15.8 | none seen, 11 t after the last |
| 51626 | 1070 | (-5.65, -12.29) | (-21.32, -9.69) | 15.88 | 0.3 -> 15.7 | water, 397 ms after hit 2 |
| 55975 | 1157 | (-6.21, -12.24) | (-16.11, -20.23) | 12.72 | 0.5 -> 13.2 | water (hurt at 55775, 4 t earlier) |
| 56381 | 1165 | (-14.58, -18.74) | (-18.57, -4.53) | 14.77 | 11.1 -> 14.8 | none seen, 8 t after the last |
| 56626 | 1170 | (-17.40, -4.82) | (-28.91, -5.95), then (-24.00, -20.62) | 11.56 + 15.47 | | water, two jumps in one tick |
| 58180 | 1201 | (-13.24, -15.11) | (-19.36, -13.75), then (-24.97, 0.62) | 6.27 + 15.43 | | water, two jumps |
| 62023 | 1278 | (-6.29, -11.62) | (2.74, -18.54) | 11.38 | 0.6 -> 10.8 | water, 293 ms after hit 4 |

- Water: touching it gives HurtAnimation 9, 1 damage (39 ... 32), a `splash` at feet +1.45, and a
  teleport the same tick (once 4 ticks later). If it lands in water it hurts and jumps again in the
  same tick. Four of the seven water hits were in the actor's own cell right after a hit: the
  enderman follows through into the actor's water and gets thrown out. That is what kept the hit
  rate down.
- Hit by the actor: teleport the next tick, 12.25 away, even under slowness 255 (one sample).
- Three angry teleports with no hurt (43926, 47931, 56381) don't fit a "teleport toward a target over
  16 away" rule: the actor was 7.8, 2.4 and 11.1 off, and none of the jumps went toward it.
- All offsets within +-16 in x and z (largest 15.67), fractional, landing standing at y -60. Packets
  as in run 1: forced MoveEntityDelta, LevelEvent SoundEndermanTeleport at origin and destination,
  plus SetEntityMotion (0, 0, 0) when it was moving.

### Letting go

- Never. TargetEid stayed on the actor and ANGRY stayed set from 41574 to the kill at 63522 (21.9 s,
  ~430 ticks), through teleports up to 24 off. The kill cleared TargetEid (63530).

## Run 1 (capture overwritten)

Stone floor at y -20 (top -19) over the flat ground (stand y -60); endermen 8 and 1.5 off. Both
teleported on their first tick and then down off the floor; the stare (once, at 6 off, held 45
ticks) got no reaction, and the `!hit` failed at 3.86 off. No target, no anger, no damage either way.

### Spawn metadata (both runs)

- AddEntity health 40, movement 0.3; Flags 19, 22, 48, 49, plus 35 on the next tick; FlagsExtended
  bits 25, 55 (36028797052518400), never changed. Box 0.6 x 2.9. EndermanHeldRuntimeId -604749536 =
  FNV-1a of `{name: "minecraft:air", states: {}}` (computed), never changed (no pickup seen in
  either run). `spawn` sound at feet +2.465. The summon also sends EntityEvent EntitySpawn on the
  actor (data 2490370).

### Calm teleports (run 1)

| ent | ms | gap | from | to | horiz | dy |
|-----|----|-----|------|----|-------|----|
| 6 | 9317 | spawn +2 ms | (12.4334, -19, 12.5000) | (27.4833, -19, 12.8278) | 15.05 | 0 |
| 6 | 17591 | 162 t | (28.2464, -19, 9.7734) | (40.1006, -60, -4.3398) | 18.43 | -41 |
| 6 | 19490 | 37 t | (40.1006, -60, -4.3398) | (32.6097, -60, -11.8626) | 10.62 | 0 |
| 6 | 24037 | 89 t | (32.6097, -60, -11.8626) | (16.7011, -56, 0.8885) | 20.39 | +4 |
| 6 | 31035 | 137 t | (20.9190, -59, -9.5286) | (36.4786, -60, -15.6299) | 16.71 | -1 |
| 8 | 34346 | spawn +1 ms | (28.2464, -19, 17.2734) | (30.3767, -60, 26.5482) | 9.52 | -41 |
| 8 | 34484 | 2.7 t | (30.3767, -60, 26.5482) | (18.1475, -60, 35.5372) | 15.18 | 0 |
| 8 | 43234 | 171.5 t | (12.7207, -60, 27.7484) | (25.4921, -60, 37.6855) | 16.18 | 0 |
| 8 | 49484 | 122.5 t | (14.3815, -60, 29.3401) | (15.6530, -60, 36.4951) | 7.27 | 0 |

- First-tick teleport in 4 of 4 spawns across both runs (run 2's were water-hurt ones, so they don't
  separate a spawn rule from water).
- Run 1 calm gaps: mean 103 ticks (5.2 s), max 8.6 s. **Run 2's enderman 1 made none in ~520 calm
  ticks**, which a flat 1%-a-tick chance would give 0.5% of the time. So the calm rule is not a
  per-tick constant. It depends on something the two runs differ in (run 1 near a floating stone
  floor at y -19 and off it; run 2 on the ground among water). Open.
- Offsets within +-16 in x and z, fractional; y = the ground below the picked point (41 down off the
  floor twice in three from it). Teleports both standing and mid-walk; it keeps walking afterwards.
- Packets: MoveEntityDelta `force_move: true` (no MoveEntityAbsolute); LevelEvent
  `SoundEndermanTeleport` data 0 at the origin and at the destination; no `mob.endermen.portal`
  LevelSoundEvent; SetEntityMotion (0, 0, 0) if it was moving; MOVING cleared the next tick.

### Stroll (both runs)

- Steady 0.1943 a tick (= 2.1586 x 0.3^2). From rest: 0.0009, 0.0040, 0.0101, 0.0196, 0.0328, 0.0496,
  0.0703, 0.0948, 0.1232, 0.1555, then 0.1731, 0.1827, 0.1880, 0.1907 ... 0.1943. Arrival 0.156,
  0.108, 0.081, 0.066, 0.058, then x0.546 slide. Corners dip to ~0.188.
- MOVING set at the first step >= ~0.05, cleared ~5 ticks into the slide.
- Falls send SetEntityMotion with vy -0.0784, -0.2305, -0.3043, -0.4475 (fall ticks 1, 3, 4, 6).
- No ambient (`mob.endermen.idle`), hurt or death sounds in either run. Death: DeathAnimation, empty
  MobEquipment, RemoveEntity ~21 ticks later; run 1's enderman 1 dropped an ender pearl (killed by
  the next `/kill`; run 2's opening kill also removed an "Ender Pearl").

## Sapling should

- Stare: no measured delay. 7 stares at 6 off across two runs never angered an enderman, though in
  both runs its head turned to the actor. Either BDS needs a look the bot doesn't hold, or a stare
  from a bot never counts. Until it is pinned, don't anger on stare.
- On a hit from the actor: take the damage (1 for a bare hand) and set the target, with the
  0.4 / 0.4 knockback away from the actor from wherever the enderman is at that moment. Set ANGRY
  and send `mob.warning` (feet +2.465) the next tick. Random-teleport (+-16 x/z, fractional, onto
  the ground) 0-2 ticks after the damage, even mid-knockback and under slowness (12.25 and 20.6
  seen).
- Angry: movement 0.45 (step 0.4371), with the ~10-tick start ramp from standing.
- Teleport toward the target: when it is over 16 off, after 30 ticks (Java `teleportTowards`: 16
  along the line to the target plus +-4 jitter per axis). Seen once, landing 2.1 off.
- Other angry teleports: ~14-16 long, away, while next to the target. 2 in 640 ticks dry, 3 in 430
  wet. Java's "looked at from under 4 blocks -> random teleport" fits the dry ones if the bot was
  looking; needs the actor's rotation to confirm. Until then, a small per-tick chance (~0.3%).
- Attack: 4.5 on easy (7 base), **cooldown 20 ticks**, hit on contact if the cooldown has run out.
  Knockback 0.4 horizontal / 0.4 up, ArmSwing + HurtAnimation 2 the same tick, `attack` sound at
  feet +1.45 the next tick.
- Water: each touch gives 1 drowning damage (HurtAnimation 9), a `splash`, and a teleport the same
  tick, repeated within the tick if it lands in water. Paths avoid water; the angry chase does not
  always.
- Teleport on spawn (first tick; 5 of 5, dry and wet). Calm random teleports: rate not settled (5 s
  mean in run 1, none in 26 s in run 2).
- Keep the target until death (nothing released it in 22 s wet / 32 s dry, out to 24 off).
- Next drill: a stare with the actor's rotation captured; the same for the angry no-cause teleports.
