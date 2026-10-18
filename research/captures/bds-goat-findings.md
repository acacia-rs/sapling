# bds-goat findings

Captures (drill `goat`): run 1 `research/captures/bds-goat-wet.txt` (ground at y -60, wet, 70 s),
run 2 `research/captures/bds-goat-one.txt` (platform at y -20, dry, ~300 s), run 3
`research/captures/bds-goat.txt` (eight goats in a glass pen, 360 s). Times are the capture's first
column (ms), ~50 ms a tick. Actor runtime 1, watcher runtime 2. Easy, noon, actor in survival. The
`instant_health` heals and the campfire/sculk noise are left out below.

Flag bits used: 11 BABY, 19 CAN_CLIMB, 22 CAN_WALK, 25 ANGRY, 34 MOVING, 35 BREATHING,
48 HAS_COLLISION, 49 HAS_GRAVITY.

- **Runs 1-2, no ram.** The goat took the actor as target ~8-9 ticks after spawn both times and held
  it 14.8 s (run 1) and 82 s (run 2), mostly 1.4-8 off on open floor. It never ran at the actor,
  rammed or hurt it.
- **Run 3, six rams** (12 pre_ram / 12 ram_impact counted over both connections). Each ram went the
  same way:
  1. The goat walks to a spot due N/S/E/W of the target, 7 off or as far as a wall allows.
  2. `pre_ram`, then 22 ticks standing still.
  3. A straight 10-11 tick charge up to 1.052 a tick.
  4. On contact: ArmSwing, HurtAnimation 30, 2 damage (1 for the baby), knockback ~1.66-1.86
     horizontal and ~0.38-0.40 up, then `attack` + `ram_impact`.

## Run 1 (wet, 70 s, `bds-goat-wet.txt`)

Actor at (6.4334, -60, 0.5), never hurt (health 20 in all 71 UpdateAttributes) and never moved by the
server (no SetEntityMotion on it). Goat unique id -579820584957.

### Spawn

- AddEntity 9043 at (6.4334, -60, -7.5), 8 off the actor. Health 10, movement 0.4. Box 0.9 x 1.3.
  Flags 19, 22, 48, 49, plus 35 on the next packet (9044). Variant 0, MarkVariant 0, no baby flag
  (bit 11 clear). FlagsExtended -9223372036821221376 (bits 25 and 63), never changed.
- `spawn` sound 9043 at (x, feet +1.105, z +0.765): the head.
- The summon's EntityEvent EntitySpawn on the actor has data 8388610 = 0x0080_0002. The enderman's
  2490370 = 0x0026_0002. So the data is (entity type id << 16) | 2 (goat 128, enderman 38).

### Target, then nothing

- 9502 (~9 ticks after spawn): **TargetEid = -579820584959**, the actor. The id is the one handed out
  just before the goat's -957, the same pattern as the ender run, where the actor was -663 and the
  enderman -661. 9533 (31 ms later): `mob.warning` at the head. No ANGRY flag at any point.
- **The floor was not dry.** `splash` (feet +0.65, i.e. mid-body) came at 9533, 9783, 10735 and 11832.
  From 9533 to 12030 the goat floated with on_ground false, y bobbing between -59.98 and -59.32,
  drifting +x+z at up to 0.0697 a tick. Water SetEntityMotion samples: (0.0293, -0.2020, 0.0293)
  10681, (0.0374, 0.0051, 0.0374) 11031, (0.0297, -0.2030, 0.0297) 11778. Later splashes at 18231
  (13.6, -0.7) and 19125 (15.3, -0.6) each came with a short float (y -59.98 to -59.94, step down
  to ~0.07). The stone fill reported 0 changed (the floor was already stone) and the air fill 898,
  so water got back in after 7025. The capture doesn't show where it came from.
- 12182-15180 (~60 ticks): it stood at (8.54, -5.40), 6.26 off. Head yaw bytes (MED rot_z) 0-27
  around the actor's bearing (byte 14), pitch byte 254. Looking at it, not moving toward it.
- 15180 on: a **random stroll**, not a run. It went +x+z, away from the actor, with the closest
  point 5.70 at 16484. Then more strolls and stops: 21479-25529 out to 18.3, 27731-31679 out to
  26.6, 38182-43232 out to 32.8. After that it stood at (39.21, 2.12) until the kill (80102).
- **Target dropped at 24282**, horizontal 15.92 off. Goat feet to actor eyes (dy 1.62) was 16.005
  there and 15.884 on the packet before. It fits "let go when over 16, feet to eyes" (one sample).
  It never took a target again; the goat was 16-33 off for the rest of the run.

### Movement

- Steady stroll step **0.1243** a tick = 2.1586 x (0.4 x 0.6)^2 = 0.12434, exact.
- From rest: 0.0006, 0.0026, 0.0065, 0.0126, 0.0210, 0.0318, 0.0450, 0.0607, 0.0789, 0.0995, 0.1108,
  0.1169, 0.1203, 0.1221, 0.1231, 0.1237, 0.1240, 0.1241 ... 0.1243. The same ramp each time
  (15180, 21479, 27731, 38182). Corners dip to 0.1201-0.1216.
- Stop: 0.1046, 0.0795, 0.0581, 0.0458, 0.0391, 0.0355, 0.0335, 0.0183, then ~0.010 creeps.
- MOVING set at the first step >= ~0.045, cleared ~9 ticks into the stop.
- **No long jumps**: y stayed at -60 except for the water floats. No run_speed step (0.7 -> 0.1692 a tick)
  or ram_speed step (1.8 -> ~1.12) anywhere.
- Sounds from the goat: `spawn`, `mob.warning`, 6x `splash`. Nothing else: no ambient, step,
  pre-ram or impact sounds.
- Kill 80102: DeathAnimation, empty MobEquipment (main hand and offhand), RemoveEntity 81181 (~21
  ticks).

## Run 2 (dry, five minutes, `bds-goat-one.txt`)

Stone platform at y -20 (stand -19), no water (no `splash`, y never off -19 for the goat). Goat
unique id -584115552253, AddEntity 8032 at (6.4334, -19, -7.5), same stats and metadata as run 1.
Every 10 s, `execute as @e[type=goat,c=1] at @s run tp @p ^ ^ ^5 facing @s` put the nearest player
5.0 in front of the goat (by its body yaw), facing it. 30 tps, 9024-299412.

### Target

- **Taken 8434** (~8 ticks after spawn): TargetEid = -584115552255, the actor. `mob.warning` at the
  head 8482. No ANGRY flag, ever.
- **Dropped 90354**, and never taken again. The tp at 89130 put the actor at (30.19, -19, -2.86),
  past the platform's edge (the goat stood at (26.12, -5.77)). The actor fell 41 blocks. The drop
  came ~24 ticks into the fall (~20 down; 5.0 horizontal), and `fall` + `fall.big` at
  (30.19, -58.83, -2.86) came at 91052 (~38 ticks, which matches a 41-block fall). Health 20 -> 0,
  DeathAnimation 91044, `disappeared` 92102 and `reappeared` 93210 at the death spot (-59.9),
  Respawn 93154. Over 16 feet-to-eyes would be ~22 ticks into the fall, so the drop fits run 1's
  16 rule to ~2 ticks.
- Target held **82 s (~1640 ticks)**: 9 tps put the actor 5.0 off, and the goat strolled to within
  1.43 (33334) and 2.53 (51582) of it. No ram, no run, no hurt.
- After the respawn, `@p` was the watcher (`CmdWatcher` in all 21 later tp outputs). The goat never
  targeted it, although it walked within 0.90 of it three times. The watcher's game mode isn't in the
  capture (probably not survival, so it isn't a valid target).

### The falls

- Actor: `fall` + `fall.big` 91052, the death above. The only health change in the run besides the
  heals.
- Watcher: `fall` 231151 and 241252, both at (0.30, -58.83, -27.70). Tps 229285 and 239303 put it
  there off the platform's edge (the goat stood still at (3.83, -19, -24.17) in between). It took no
  harm and was tp'd back up each cycle. Three `land` sounds (159454, 199552, 219604) are the watcher
  dropping ~0.4 after tps to y -18.5..-18.6.
- The platform is wider than the 41x41 fill on some sides: the goat stood at z -24 and x 26.1 at
  y -19.

### Each 10 s cycle

| tp ms | who | goat at tp | goat did | closest | end off |
|-------|-----|------------|----------|---------|---------|
| 9024 | actor | (6.43, -7.50) | stood 57 t, strolled away | 5.00 | 14.82 |
| 19039 | actor | (3.58, -17.04) | stood 157 t, strolled | 5.00 | 7.96 |
| 29043 | actor | (7.66, -15.64) | strolled past it | 1.43 | 9.30 |
| 39074 | actor | (6.57, -22.03) | strolled away | 5.00 | 11.86 |
| 49080 | actor | (10.41, -15.99) | strolled past it | 2.53 | 5.91 |
| 59092, 69101 | actor | (16.96, -9.37) | stood the whole 20 s | 5.00 | 5.00 |
| 79119 | actor | (16.96, -9.37) | strolled away | 5.00 | 14.83 |
| 89130 | actor | (26.12, -5.77) | (actor fell, died) | 5.00 | 15.67 |
| 99146-299412 | watcher | | 21 cycles: 3 stood throughout, the rest strolled, 8 passing within 0.9-3.4 | | |

- **No step toward a ram anywhere**: 2475 goat moves. The fastest steps (0.13-0.21) are all 1.0-1.4
  off the watcher (107457-108504, 150905-153303, 211601-211653), i.e. entity push. Everything
  else is the stroll (0.1243 with the run-1 ramp) or standing. No run_speed (0.1692) or ram_speed
  (~1.12) step.
- No SetEntityMotion on anyone, no HurtAnimation, no EntityEvent on the goat until
  `/kill` (DeathAnimation 309425).
- The goat's packets to the actor stopped while it was dead or respawned (RemoveEntity 162851) and
  came back with a fresh AddEntity at 165452. That is ordinary view-range churn.

## Why runs 1-2 had no ram

- Not targeting: target held 296 ticks (run 1) and ~1640 ticks (run 2).
- Not range or sight: 4-7 off for long stretches, on open floor.
- Run 3 shows a goat can hold the target over 100 s inside the pen before its first ram (below).
  That fits a ram cooldown from spawn of up to 300 s. Run 2 lost the target at 82 s; with a uniform
  30-300 s, a first cooldown over 82 s has a chance of 218/270 = 81%.

## Run 3 (eight goats, rams)

Stone at y -20 (stand -19). A glass ring 3 high at +-7 around the actor's block leaves air for
x and z in [-6, 7); the actor started at (0.5, -19, 0.5) and was never tp'd. Eight goats were
summoned one a second from 10233 (runtime 3-10, unique ids -588410519549 ... -542; the actor is
-588410519551):

| goat | at | spawned |
|------|----|---------|
| g3 | (+5, 0) | 10233, **baby** (flag 11, Scale 0.5) |
| g4 | (-5, 0) | 11233 |
| g5 | (0, +5) | 12234 |
| g6 | (0, -5) | 13232 |
| g7 | (+4, +4) | 14234 |
| g8 | (-4, +4) | 15232 |
| g9 | (+4, -4) | 16238 |
| g10 | (-4, -4) | 17233 |

The baby's `mob.warning.baby` comes at feet +0.552 (adult `mob.warning` at +1.105).

- All eight took the actor 0-9 ticks after spawn.
- **They got out of the pen.** Every goat long-jumped (`jump_to_block`) onto the 3-high glass
  (y -16) and most dropped outside, first between 52 and 94 s. g6 and g7 never came back after
  ~70 s. The jumps:
  - SetEntityMotion vy 1.221-1.278, horizontal 0.215-0.226, re-sent every 3 ticks with vy 0.24
    lower each time.
  - `jump_to_block` sound at the take-off.
  - EntityEvent DustParticles on landing, ~1.3-1.5 s later.
- The actor's position isn't in the capture. I inferred it from the contacts: the knockbacks pushed
  it to the east glass (~(6.6, 1.3)) at 140 s and into the SE corner (~(6.6, -5.6)) at 217 s.

### The six rams

| # | goat | ram start (TargetEid -> 0) | pre_ram | goat at pre_ram | off target | ArmSwing | charge | hit |
|---|------|------------------------|---------|-----------------|------------|----------|--------|-----|
| 1 | g5 | 89132 (had target) | 91182 | (-5.334, 0.705) | 5.84 | 92784 (+32 t) | +x, 5.26 | **watcher** |
| 2 | g5 | 137034 (had target) | 138481 | (-5.502, 0.270) | 6.01 | 140082 (+32 t) | +x, 5.25 | actor |
| 3 | g5 | 213382 (+ `mob.warning`, no target before) | 215482 | (6.456, 6.269) | ~5.0 | 217032 (+31 t) | -z, 4.23 | actor |
| 4 | g8 | 272734 (+ `mob.warning`, no target before) | 273886 | (-0.480, -5.233) | ~7.1 | 275432 (+31 t) | +x, 4.19 | **g4** (in the way) |
| 5 | g9 | 274681 (+ `mob.warning`, no target before) | 279683 | (-0.431, -5.241) | ~7.0 | 281282 (+32 t) | +x, 5.25 | **g4** again |
| 6 | g3 (baby) | 302783 (had target) | 304731 | (-0.250, -5.460) | ~6.8 | 306384 (+33 t) | +x, 6.28 | actor |

"Off target" is from the actor (0.5, 0.5) for rams 1-2, and from the inferred actor position for
rams 3-6.

- **The run-up.** At ram start TargetEid is sent as 0. When the goat had no target, `mob.warning`
  plays as well (rams 3-5: those goats had been without a target since 70-213 s, outside the pen
  or on the wall). Then it walks to a spot **due W/E/N/S of the target** (every charge is
  axis-aligned, within 0.25), the same as Java's four-direction ram start. Details:
  - The spot is 7 off when there is room (rams 4-6, ~6.8-7.1), and against the glass when not
    (rams 1-2 at 5.84 and 6.01, the west wall; ram 3 at ~5.0).
  - Ram 3 started on the wall top and jumped down. Ram 5 started on the wall and spent 100 ticks
    getting down.
  - The walk is at **0.1690 a tick** (run_speed: 2.1586 x (0.4 x 0.7)^2 = 0.1692), ramping 0.138,
    0.152, 0.160, 0.164, 0.167 ... from a stroll. The arrival creeps at ~0.043 for 5-10 ticks
    and then stops. Ram start to pre_ram: 41, 29, 42, 23, 100, 39 ticks.
- **pre_ram** fires 1 tick after the goat stops (step < 0.003), at its feet (y -19). It stands
  **22 ticks** (1100-1103 ms in all six), then charges.
- **The charge** is identical in all six, per tick: 0.0282, 0.0583, 0.1131, 0.1888, 0.2859,
  0.4043, 0.5458, 0.7090, 0.8950, 0.9965, 1.0521. It never reaches ram_speed's steady 1.12
  (2.1586 x 0.72^2). Contact comes on the 10th or 11th step (step 0.997 or 1.052) after a straight
  4.2-6.3 run: **31-33 ticks after pre_ram**.
  - Mid-charge the goat gets two SetEntityMotion: ~0.22-0.30 at step ~0.40-0.55, and ~0.49-0.54
    at step ~0.9-1.0.
- **Impact tick**:
  - ArmSwing on the goat.
  - On the target: HurtAnimation **data 30** (the ram damage cause), plus the knockback below.
  - Health ~35-50 ms later: actor 20 -> 18 (adult, **2 on easy**), 20 -> 19 (baby, 1); g4
    10 -> 8 -> 6.
  - Next tick: `attack` at the goat's contact position, feet +0.65 (baby +0.325), and
    `ram_impact` at the same x/z at feet height.
  - The watcher (ram 1) took no damage and no HurtAnimation. Probably not survival, the same as
    run 2.
- **The goat after impact** keeps going through: steps 0.46-0.57, then 0.25-0.31, 0.05-0.16 (x
  0.546 on the ground). SetEntityMotion on the goat 0.26-0.31 at impact +1 tick, then 0.02-0.09.
  It doesn't bounce back.
- **Knockback.** The actor gets two SetEntityMotion on the impact tick:
  - First, a plain hit's: 0.400 horizontal along the charge, vy 0.400.
  - Then the ram's: (1.859, 0.399, 0.079) for ram 2, (0.048, 0.379, -1.690) for ram 3,
    (0.899, 0.284, -0.039) for the baby.
  - All of them fit **v = v_prev / 2 + push** (Java `knockback`), with push = **1.5 x s^2**
    horizontal along the charge and **0.18 x s^2** up, where s is the goat's step at contact.
    Ram 2: s 1.0521 gives push 1.6604 / 0.1992, result 1.8604 / 0.3992 (seen 1.8606 / 0.399).
    Ram 3: s 0.9970 gives 1.4911 / 0.1789, result 1.69 / 0.379 (seen 1.6903 / 0.379).
  - Watcher (no plain hit, prev vy -0.0784): push 1.6604 / 0.1992 gives 1.6604 / 0.1600 (seen
    (1.659, 0.160, -0.070), 1.6608).
  - g4 (one packet each, the plain hit folded in): 0.2 + 1.5 s^2 = 1.689 / 1.860 (seen 1.6784 /
    1.8489), vy 0.379 / 0.399 exact.
  - Baby: push 0.702 / 0.084 at s 0.9998, i.e. 0.468 of the adult's.
  - The pack's knockback_force 2.5 / knockback_height 0.04 aren't used as they stand: 1.5 = 2.5 x
    0.6 and 0.18 = 0.04 x 4.5. The mapping is open; build the measured constants.
- **No miss seen**: every ram hit something (the watcher, the actor 3x, a goat 2x), so whether a
  miss sounds `ram_impact` is not measured. A goat standing in the charge line takes the hit for
  the target behind it (rams 4-5).

### Cooldowns

- **First ram after spawn**:
  - g5: ram start at 77 s.
  - g8 at 257 s, g9 at 258 s. Both had been outside the pen since ~67 / 96 s, and each started
    within a second or two of getting back onto the wall or into the pen. Their cooldowns could
    have run out earlier.
  - g3: 292 s, after 117 s inside the pen holding the target.
  - g4: none in 367 s. It was inside with a target from 167 to 314 s, which is past spawn + 300 s
    by ~3 s.
  - g6, g7, g10: none, mostly outside.
  - So a uniform 30-300 s cooldown from spawn fits the samples, but both informative ones (g3, g4)
    sit at the top of the range. **cooldown_range is seconds, not ticks**: 30-300 ticks
    (1.5-15 s) can't explain g3's 117 s in reach with a target.
- **One goat's next ram**: g5's ram start came 44.2 s after ram 1's impact (92.8 -> 137.0). After
  ram 2 it was out of the pen 152.8-210.4, so 73 s to ram 3 is an upper bound. One clean gap,
  inside 30-300 s.
- **Re-targeting after a ram** (`mob.warning`, TargetEid = actor) came **33.0-38.7 s after the ram
  start**: g5 33.8, 34.0; g8 33.0; g9 33.5; g3 38.7. From the impact it is 26.9-35.1 s. So
  attack_cooldown (30-40 s) runs from the ram start and blocks nearest_attackable_target, while
  the ram itself doesn't need the target goal: rams 3-5 started with no target held.
- TargetEid otherwise changes as before: dropped out of range or sight (goats outside the glass),
  re-taken with `mob.warning` when back.

## Sapling should

- Goat: health 10, movement 0.4, box 0.9 x 1.3, `spawn` sound at the head (feet +1.105, +0.765
  forward). EntitySpawn data = (type id << 16) | 2.
- Take the target (any valid non-goat within 16, must_see) ~8-9 ticks after spawn, with no ram
  cooldown check. Send `mob.warning` at the head right after. No ANGRY flag.
- Let it go when the target is over 16 off, feet to eyes (fits both drops: one walking away, one
  falling). Don't re-target non-survival players.
- **Ram cooldown**: start one at spawn, sampled uniform 30-300 **s** (cooldown_range in seconds),
  and again after each ram. While it waits, the idle goals run (stand and look, stroll at 0.1243,
  float, long-jump) even with a target 1-5 off. The goat doesn't approach.
- **Ram start**, when the cooldown is out and a player is in reach (the ram picks its own target
  and doesn't need TargetEid):
  - Send TargetEid 0, plus `mob.warning` at the head if the goat held no target.
  - Start attack_cooldown 30-40 s from this point. It blocks nearest_attackable_target; re-target
    with `mob.warning` when it ends (seen 33.0-38.7 s).
- **Run-up**: pick the spot due W/E/N/S of the target at 7 (nearer if a wall is in the way; seen
  5.0-7.1). Path there at run_speed 0.7 (step 0.1692), arriving with a ~0.043 creep.
- **pre_ram** at the goat's feet 1 tick after it stops. Stand 22 ticks.
- **Charge** straight along the axis with the steps 0.0282, 0.0583, 0.1131, 0.1888, 0.2859, 0.4043,
  0.5458, 0.7090, 0.8950, 0.9965, 1.0521 (then toward 1.12). Hit the first entity the box meets,
  goats included.
- **Impact**, on the contact tick:
  - ArmSwing on the goat. On the target, HurtAnimation 30.
  - Damage 2 on easy (baby 1).
  - Survival target: first the plain hit's knockback (0.4 along the charge, vy 0.4) as its own
    SetEntityMotion. Then v = v/2 + push, push = 1.5 s^2 horizontal (along the charge) and
    0.18 s^2 up, s = the goat's step at contact. Baby: push x 0.468.
  - Non-survival players: no damage, no plain hit, push only.
  - Next tick: `attack` at the goat's position, feet +0.65 (baby +0.325), and `ram_impact` at the
    same x/z, feet height.
- **After impact** the goat keeps its momentum (0.46-0.57, then x0.546 on the ground). No bounce.
- **Long jumps** (`jump_to_block`): vy ~1.22-1.28, horizontal ~0.22, take-off sound, then
  DustParticles on landing. Goats clear a 3-high wall with them.
- Stroll with the ramp above. Entity push gives up to ~0.21 a step.
- Water: float (y bobs -59.3 to -60) at ~0.07 a tick, `splash` at feet +0.65 on entering.
- Still open: whether a missed ram sounds `ram_impact`, what happens when the charge ends at glass,
  the formula behind 1.5 / 0.18, and the baby factor.
