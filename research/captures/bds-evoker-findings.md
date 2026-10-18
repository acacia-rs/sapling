# bds-evoker findings

Capture: `research/captures/bds-evoker.txt` (drill `evoker`). Times are the capture's first column (ms).
Ticks come from the actor's UpdateAttributes/SetEntityMotion `tick` fields, interpolated (+-1);
deltas below are counted in ~50 ms send batches and are exact. Actor runtime 1, unique -597000454143.
Easy, midnight, stone floor y -20 (top -19) over x -3..12, z -12..40, air to y -13. The
`instant_health` heals (health back to 20 each second) are left out below.

Actor position: no packet carries it. Start (6.5, -19, 20.5) (tp output). At evoker 2's summon it was
at (5.6715, -19, 31.7899) (evoker 2 = actor z-2). Vex knockbacks moved it in +z in between.

## Casts

| # | evoker | flag on (ms, tick) | start sound | spell | fangs / vexes | flag off |
|---|--------|--------------------|-------------|-------|---------------|----------|
| 1 | E1 rt 3 | 9186, t234 | `cast.spell` 9237 | line 16 | 10233-10990, t255-270 | 11132 |
| 2 | E1 | 16188, t372 | `prepare.summon` 16237 | 3 vexes | 21235-21237, t470 | 21138 |
| 3 | E1 | 38236, t803 | `cast.spell` 38284 | line, cut at 5 | 39283-39486, t824-828 | 39487 |
| 4 | E2 rt 31 | 44285, t923 | `cast.spell` + `prepare.attack` 44335 | circles 5 + 8 | 45334 / 45485, t943 / t946 | 46236 |
| 5 | E2 | 51285, t1059 | `cast.spell` 51331 | line 16 | 52341-53086, t1080-1095 | 53235 |
| 6 | E2 | 58282, t1197 | `cast.spell` 58334 | line 16 | 59339-60084, t1218-1232 | 60235 |

- Casting = Flags bit 42 (EVOKER_SPELL): 844459294588928 -> 848857341100032. Same packet sets
  `EvokerSpellCastingColor`: fangs -10072743 = 0xFF664D59 (0.4, 0.3, 0.35); vex -5000244 =
  0xFFB3B3CC (0.7, 0.7, 0.8), Java's colours. The colour is never cleared (only rewritten).
- The flag is on for 39 ticks (1946-1953 ms), cast_duration 2 s, and off 2 ticks before the vexes
  appear for the 5 s vex cast (99 ticks on).
- The first fang comes 20 ticks after the start sound (996-1010 ms) = base_delay 1 s. Vexes come
  100 ticks after `prepare.summon` (5000 ms) = base_delay 5 s.
- Gap from flag off to the next flag on: 101 ticks after a fang spell (5056, 5049, 5047 ms),
  342 ticks after the vex spell (17098 ms). The cooldown that applies is the cooldown of the spell
  just cast (5 s, 17 s), measured from the end of the cast. Start to start: 140 ticks (fang) and
  441 ticks (vex) = duration + cooldown.
- Picks: E1 at 10 off chose line, vex, line. E2 at 2 off chose circles (target < 3), then two lines
  (the circle knockback put the actor past 3). E2 never chose vexes in 15 s.
- Sounds: `cast.spell` starts every fang cast (1 tick after the flag). `prepare.attack` plays at the
  circle cast's start and again when circle 1 spawns (45336); line casts have no second sound.
  `prepare.summon` plays at the vex cast's start and again when the vexes spawn (21237). No sound at
  circle 2 or the line's fangs.
- Cast 3 was cut: E1 lost its target (TargetEid 0, flag off, same packet 39487, t828) after 5 of
  16 fangs and no more fangs came. The actor was then ~19.6 off (estimate from the vex hit at 39437).

## Fangs (66, all with OwnerEid = the evoker)

- AddEntity: yaw/pitch/head/body yaw 0 for all 66, velocity 0, Flags 0, FlagsExtended 0, box 1.0 x
  0.8, `LimitedLife` 22. The position is integer x/z (floored) and y = the ground top.
- Life, the same for every fang: spawn s: SetEntityData LimitedLife 20. s+1: EntityEvent ArmSwing
  (event 4, data 0), LimitedLife 18, and the damage (below). s+2: LevelSoundEvent `throw` at
  (x, y+0.4, z) and `fang` at (x, y, z), both entity_type evocation_fang. Then LimitedLife -2 a tick
  (16 ... 2), -1 at s+10, RemoveEntity at s+11. The 66 `throw` sounds are those, one per fang, so
  probably an off-by-one sound-id name (as with the witch's `undefined`/`disappeared`).
- Line: fang i (0..15) at floor(evoker + 1.25 (i+1) dir), spacing 1.25 = size 20 / 16, first fang
  1.25 out. One fang a tick (delay_per_summon 0.05 s).
  - E1 cast 1 from (6.5, 10.5): z 11, 13, 14, 15, 16, 18, 19, 20, 21, 23, 24, 25, 26, 28, 29, 30, x 6:
    exactly floor(10.5 + 1.25 (i+1)), straight +z at the actor.
  - E1 cast 3 from (6.7655, 10.2804): z 11, 12, 14, 15, 16, x 6 (= floor(10.28 + 1.25 (i+1))).
  - E2 casts 5 and 6 from (5.7676, 29.6803), identical: z 30, 32, 33, 34, 35, 37, 38, 39, 40, 42, 43,
    44, 45, 47, 48, 49. x 5 for i 0-12, x 4 for i 13-15, so dir x is -0.0439 to -0.0472. The
    evoker -> actor direction at the summon is -0.0455. The actor only moved along that ray, so
    whether the direction is taken at the cast or at each fang is not pinned.
  - y: -19 on the floor. z 42 (past the stone, z > 40) is at y -20, z 43-49 at -19 again, so each
    fang snaps to its own column's ground.
- Circles (E2 at (5.7676, 29.6803), base angle f = atan2(dz, dx) to the actor = 92.6 deg):
  - Circle 1, 5 fangs, r 1.5, all at 45334-45336 (t943): in creation order (5,31), (4,30), (4,28),
    (6,28), (7,30) = floor(E + 1.5 (cos, sin)(f + i 72 deg)) for f 84.4-95.2 deg.
  - Circle 2, 8 fangs, r 2.5, 3 ticks later at 45485-45488 (t946): (5,32), (3,31), (3,29), (4,27),
    (5,27), (7,28), (8,29), (7,31) = f + i 45 deg for f 92.8-95.3 deg. There is no offset (Java adds
    72 deg); f is 0.2 deg off the summon-time direction (the actor moved after the circle 1 hit).
  - All fangs in a circle swing, sound and are removed together (s+1, s+2, s+11).

## Damage to the actor

| ms | tick | by | health | HurtAnimation data | SetEntityMotion |
|----|------|----|--------|--------------------|-----------------|
| 10636 | 263 | fang rt 11 (6, 20), line 1 | 20 -> 14 | 14 (magic) | (0, 0.4, 0.4) |
| 23385 | 512 | vex rt 22 | 20 -> 14.5 | 2 (entity attack) | (-0.2185, 0.4, 0.3351) |
| 29140 | 625 | vex rt 22 | 20 -> 14.5 | 2 | (0.3692, 0.4, 0.1538) |
| 32785 | 697 | vex rt 21 | 20 -> 14.5 | 2 | (-0.3592, 0.4, 0.1761) |
| 33734 | 715 | vex rt 21 | 20 -> 14.5 | 2 | (-0.1753, 0.4, 0.3601) |
| 38186 | 803 | vex rt 21 | 20 -> 14.5 | 2 | (0.0511, 0.4, 0.3967) |
| 39437 | 827 | vex rt 20 | 20 -> 14.5 | 2 | (0.1720, 0.4, 0.3611) |
| 45384 | 944 | circle 1 | 15.5 -> 9.5 | 14 | (-0.0182, 0.4, 0.3996) |
| 52487 | 1083 | fang rt 47 (5, 33), line 5 | 20 -> 14 | 14 | (-0.0182, 0.4, 0.3996) |
| 59591 | 1223 | fang rt 65 (5, 35), line 6 | 20 -> 14 | 14 | (-0.0182, 0.4, 0.3996) |

- Fang damage is 6 on easy (unscaled, or a base of 10 under Java's d/2+1). It lands on the fang's
  ArmSwing tick (s+1), in the same batch as the HurtAnimation and SetEntityMotion, not on the sound
  tick.
- Fang knockback is 0.4 horizontal + 0.4 up, along the **owner -> actor** direction, not the
  fang's: circle 1 and both E2 lines give the same (-0.0182, 0.3996) = 0.4 x the E2 -> actor unit
  vector (-0.0455, 0.9990).
- One hurt per cast. In line 1 the next fangs (z 21, 23, ... at t264-271) fall inside the 10-tick
  invulnerability. Circle 2 (t946) is also inside it. On line 5, fang z 32 (t1082) missed and z 33 hit,
  so the actor's z was ~32.8-33.8. On line 6, z 34 missed and z 35 hit (~34.8-35.8), so ~2 blocks per
  fang knockback.
- Vex hits: 5.5 each on easy (fits (3 + iron sword 6)/2 + 1; not checked). The knockback is 0.4
  horizontal along the vex's flight, 0.4 up.
- The actor's Flags gain bit 34 (MOVING) 1-2 ticks after each knockback (844459290394624 ->
  844476470263808) and lose it ~13 ticks later. Natural regen 14.5 -> 15.5 at 43485 after the
  `/effect clear`.

## Vexes (cast 2)

- 3 at 21235-21237 (t470): rt 22 (16, -19, 8), rt 21 (14, -19, 7), rt 20 (14, -19, 9). E1 stood at
  (15.1641, 8.5321) (block (15, 8)), so the offsets are (+1, 0), (-1, -1), (-1, +1) in blocks, with
  integer x/z like fangs and y at the evoker's feet. Same tick: MobEquipment mainhand network_id 309
  (iron sword presumably). Health 14, movement 1.0, box 0.4 x 0.8, OwnerEid = E1, LimitedLife -1.
- Flags 1125899911561216 = bits 19, 22, 50. The evoker has 19, 22, 35, 48, 49, so the vex has no
  HAS_COLLISION (48), no AFFECTED_BY_GRAVITY (49) and no BREATHING (35), and has FIRE_IMMUNE (50).
  Moving adds bit 34 and charging adds bit 43 (1134696004583424).
- TargetEid = actor 2 ticks after spawn (21239). Each vex then re-sends the same TargetEid **every
  tick**, each with a `mob.warning` sound (410 in all: rt 20 217 to 33037, rt 21 183 to 30336,
  rt 22 10 to 21682). E1 itself warns once.
- Charge: bit 43 set, `charge` sound the next tick (8 charges). Charge to hit: rt 22 21339 -> 23384
  (41 t), 28032 -> 29140 (22 t), 32785 -> no hit. rt 21 28934 -> 32785 (77 t), 33184 -> 33734
  (11 t), 37084 -> 38186 (22 t). rt 20 30934 -> no hit, 38036 -> 39437 (28 t). Hit = ArmSwing +
  damage; the `attack` sound comes 1 tick later at the vex (6 sounds, y ~-17.1 to -17.6, i.e. it
  hits from about the actor's eye height). Bit 43 clears on the hit.
- Charge motion (rt 22, t481-512): v' = 0.91 v + 0.05 along the aim. Steps 0.3166, 0.3379, 0.3573 ...
  0.5419 horizontal + 0.047 up (terminal 0.556 in 3D). After the hit: no thrust, x0.91 a tick
  (0.2465, 0.2244, 0.2042 ...). Wander between charges ~0.135 a tick (median of all 3).
- Flight: y -19 ... -10.67 (rt 20), above the cleared volume's top (-13), and x up to 16 (past the
  floor's 12.5). Whether those cells are solid is not in the capture. Vex SetEntityMotion packets
  are sent every ~5-10 ticks while it flies.
- Lifetime: no health change (ATTR) from spawn to `/kill` at 40049 (t839, 369 ticks = 18.5 s), so
  the 30-119 s damage timer was not reached. Deaths: DeathAnimation, mainhand + offhand
  MobEquipment, TargetEid 0, health 0.

## Evoker movement and target

- E1 spawn 9035 (t231), TargetEid = actor at 9186 (3 ticks later, with the cast-1 flag),
  `mob.warning` 9237 (next tick, with `cast.spell`). E2: spawn t919, target t923, `mob.warning` t924.
- Neither evoker moved while casting. E1 walks, all between casts and > 12 from the actor (so not
  avoid_mob_type's 8):
  - t297-360, 5 s after cast 1's fangs: (6.50, 10.50) -> (15.16, 8.53), straight +x then curving,
    peak 0.1943 a tick, Flags bit 34 12685-15388. Stopped 12 ticks before cast 2.
  - t476-488: a 0.3-block shuffle (peak 0.155) as the vexes appeared on it.
  - t625-679 (vex cooldown): back to (6.77, 10.28). Peak **0.3877** (t633-655, 2.0x the first walk),
    then 0.1943 (t656-666), then x0.546 decay. It started on the same tick as a vex hit (29140).
    Which goal drives it is not identified.
- E2 (2 off) moved 4 ticks *away* from the actor (+x -z, steps 0.0024, 0.0105, 0.0251, 0.0527:
  avoid_mob_type) and stopped at t923 when cast 4 began, before the slowness (MobEffect id 2 amp
  255 at 45055, movement 0). It never moved again.
- The held E2 still turns its head (MoveEntityDelta rot_z) and pitch (rot_x) during casts; its body
  yaw (rot_y) changes only twice (t992).

## Other

- No land, step, hurt, ambient or death sounds from any mob. 82 `block.campfire.crackle` (noise).
- Each evoker death drops 2 xp orbs (values 3 and 7) at the evoker's position, plus an entity that
  has no AddEntity in the capture (rt 28, rt 77; it falls from y -18.875, has flags like an item). It
  is probably the totem item, sent as AddItemActor, which the capture does not log. rt 28 is
  removed by the second `/kill` (61086).
- E2 AddEntity FlagsExtended has bit 22 set (-9223372036817027072), cleared with the target packet.
  E1's FlagsExtended stays -9223372036821221376 (bits 25, 63).
