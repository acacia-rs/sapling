# bds-pandas findings

Capture: `research/captures/bds-pandas.txt` (drill `pandas`). Times are the capture's first column (ms),
~50 ms a tick. Actor rt 1 (unique -657129996287), fixed at the tp spot until a bite throws it. Panda rts:
3 bystander (`panda_aggressive` event), 4 aggressive, 5 lazy, 6 worried, 7 weak, 8 playful. Every
panda spawned at (0.5, -19, 3.5) (bystander 6.5, -19, 3.5); actor at z 2.0. The panda packets are the
same in both streams (1-2 ms jitter). Panda MoveEntityDelta come one a tick, sometimes one per 2 ticks;
speeds below are per tick. Flags: bit 34 MOVING, 35 BREATHING, 25 ANGRY; FlagsExtended bit 3 ROLLING,
22 IS_AVOIDING_MOBS, 25 set on every panda from spawn (as in provoke3). Scripts: scratchpad `pandas_*.py`.

## Spawn: Variant and health

| rt | event | AddEntity Variant | first SetEntityData (+1 ms) Variant | health | movement |
|----|-------|-------------------|-------------------------------------|--------|----------|
| 3 | panda_aggressive (bystander) | 6 | **3** | 20 | 0.15 |
| 4 | panda_aggressive | 6 | 6 | 20 | 0.15 |
| 5 | panda_lazy | 1 | 1 | 20 | **0.07** |
| 6 | panda_worried | 2 | **4** | 20 | 0.15 |
| 7 | panda_weak | 5 | 5 | **10** | 0.15 |
| 8 | panda_playful | 3 | 3 | 20 | 0.15 |

- AddEntity already carries the event's Variant (provoke3, summoned with no event, had Variant 0
  there). The full SetEntityData sent 1 ms later re-rolled it for **2 of 6** pandas. The cause is
  open (genetics birth event?). MarkVariant 0 and metadata Health 1 on every panda; no other gene field.
- Behaviour after the flip is mixed. rt 3 (now 3) **rolled** like a playful panda and never acted
  aggressive. rt 6 (now 4, brown) still **avoided like worried**: IS_AVOIDING_MOBS was set on its spawn tick.
- Spawn flags 19, 22, 48, 49, 54, then +35 in the same ms. Box 1.3 x 1.25.
- Weak: health attribute **10/10** (AddEntity and UpdateAttributes). Its hit took it to 9.

## The player's hit (all five, identical)

- HurtAnimation 2, then health -1 and **TargetEid = actor** in the same ms. `mob.warning` (panda)
  arrives +1 tick at the panda's pre-knock feet + (1.105 along its facing, 1.1875, 0): 0.85 x width,
  0.95 x height. **No ANGRY (bit 25) on any panda, ever**, unlike provoke3, where ANGRY came with
  `mob.warning`. No panda hurt, ambient, sneeze or eat sounds, and no Sitting/Laying/Eating metadata changes.
- Panda knock: delta (0, 0.400, 0.400) on the hit tick (away from the actor). SetEntityMotion
  (0, 0.3136, 0.2184) = (0.4 - 0.08) x 0.98 and 0.4 x 0.546. Then xz x0.91 a tick in air. It
  lands +10 ticks, 2.06 blocks back. The panda was slowed (movement 0) until `effect clear`, ~1 s after the hit.
- Every panda kept TargetEid = actor until the /kill (19-22 s). **Only the aggressive one ever
  attacked.** In provoke3 the playful, normal and lazy pandas each bit once; here none of them did
  (they panicked instead; see below). Differences: midnight vs noon, open floor vs pen, slowness through the knock.

## Speed law (fit)

Per-tick ground speed v = **2.16 x (movement x multiplier)^2** (0.0486 = 2.16 x 0.15^2):

| case | movement x mult | predicted | measured |
|------|-----------------|-----------|----------|
| walk / chase, 0.15 x 1 | 0.15 | 0.0486 | 0.0485-0.0486 |
| panic, 0.15 x 1.25 | 0.1875 | 0.0759 | 0.0742-0.0753 |
| worried sprint, 0.15 x 1.5 | 0.225 | 0.1094 | 0.1068-0.1073 |
| lazy panic, 0.07 x 2.5 | 0.175 | 0.0662 | 0.0654-0.0660 |
| lazy walk (provoke3), 0.07 x 1 | 0.07 | 0.0106 | 0.0105-0.0106 |

This explains provoke3's "unexplained" lazy step of 0.0106. Speed ramps up over ~8 ticks from rest.

## Aggressive (rt 4)

- Hit 15275. No panic (it kept TargetEid and walked at the actor). After the clear (16333) it
  ramped up from 16438 and chased at **0.0486 (walk speed, no attack bonus)**.
- **8 bites**, each ArmSwing + actor SetEntityMotion + actor HurtAnimation 2 in one tick, actor
  health **20 -> 16 (4 on easy = 6/2 + 1)** 0-2 ticks later, then `attack` (panda) +1 tick at **the
  panda's swing-tick feet + (0, 0.625, 0)** (half height). The actor heals to 20 each second.

| # | ms | +ticks | panda feet (x, z) | actor knock (x, y, z) | actor land (x, z) |
|---|----|--------|-------------------|-----------------------|-------------------|
| 1 | 18638 | 67 after hit | (0.915, 3.747) | (-0.0925, 0.4, -0.3892) | (0.087, 0.260) |
| 2 | 20733 | 42 | (0.990, 1.710) | (-0.1959, 0.4, -0.3487) | (-0.855, -1.573) |
| 3 | 26587 | **117** | (-0.554, -0.129) | (-0.1005, 0.4, -0.3872) | (-1.441, -3.548) |
| 4 | 28641 | 41 | (-0.933, -2.082) | (-0.1262, 0.4, -0.3796) | (-2.076, -5.517) |
| 5 | 30883 | 45 | (-1.787, -4.034) | (-0.0845, 0.4, -0.3910) | (-2.542, -7.531) |
| 6 | 32983 | 42 | (-1.973, -6.061) | (-0.1356, 0.4, -0.3763) | (-3.208, -9.487) |
| 7 | 35233 | 45 | (-2.783, -8.024) | (-0.1153, 0.4, -0.3830) | (-3.818, -11.464) |
| 8 | 37284 | 41 | (-2.972, -10.001) | (-0.1888, 0.4, -0.3526) | (-4.744, -13.310) |

- Actor knock: **xz 0.4000 along panda -> actor, y 0.4** (bite 1: the direction matches the
  actor at (0.5, 2.0) to 1e-3). "Land" is the watcher-only player `land` sound ~11 ticks after
  each bite, so each bite throws the actor ~1.8-2.0 blocks.
- The interval is set by the walk, not by a cooldown: 41-45 ticks = ~2 blocks at 0.0486.
  `melee_box_attack` not once: it never let go.
- Reach: bite 1 came the first tick the xz box gap (max axis |d| - 0.65 - 0.3) reached **0.797**.
  Bites 2-8 came 5-6 ticks after the gap to the **land point** passed 0.8, at gap 0.49-0.53. That fits
  horizontal reach 0.8 if the actor slides ~0.28 after landing (ground friction from ~0.16 left);
  no actor position packet confirms it.
- Bites 2 -> 3 (117 ticks): from 20890 to ~25000 the panda jittered within ~1.5 blocks at
  0.02-0.15 a tick, MOVING toggling every 2-6 ticks, with the actor ~2.5-3 blocks off. Then it walked
  again at 0.0486. Cause unknown (stuck path?).
- **No calming**: TargetEid stayed set from the hit (15334) to death (37314), 22 s. No ANGRY flag,
  so the 500 s anger is not visible on the wire.

## Bystander (rt 3, panda_aggressive event, Variant flipped to 3)

- Never took a target, never ANGRY, never attacked. It was 15.4 blocks from rt 4 at the hit
  (inside the broadcast 41). **But the broadcast test is void**: it behaved playful.
- Rolls: ROLLING (FX 3) set with SetEntityMotion (h, vy 0.27), h = 0.2000 (once 0.2178, once
  0.2005), held constant along its facing. vy goes back to 0.27 on each landing (a hop every 8-15
  ticks). **Every roll lasts 32 ticks** (FX 3 set -> cleared: 10684, 12432, 14483, 17733, 27433;
  the playful rt 8 the same). **Slowness 255 did not stop it** (movement 0 from 12033): three
  back-to-back rolls took it from z 3.5 to 22.1. Later rolls went to z 28.2 and (9.6, 33.4).

## Lazy (rt 5)

- Hit 44459. Health 20 -> 19, TargetEid. Clear at 45474. **Panic 46034-48233 (~45 ticks) at
  0.0654-0.0660** away (+z), 2.7 blocks (dist 3.8 -> 6.5). Then near-still (<= 0.014) until the kill.
  Never bit. No TEMPTED (provoke3's lazy had it at noon).

## Worried (rt 6, Variant 4 after the flip)

- **Before the hit**: IS_AVOIDING_MOBS from its spawn tick (69539). It ramped up and fled at
  **0.1073** (sprint x1.5) from 69984 until slowness (70541), (0.5, 3.5) -> (-0.84, 4.03).
- Hit 73754: TargetEid set **and IS_AVOIDING_MOBS cleared** in one packet. Panic at 0.0742 (x1.25)
  for 42 ticks after the clear (75334-77382), 3.7 -> 5.8 blocks off.
- 77488: IS_AVOIDING_MOBS set again. Sprint 0.1068 for 6 ticks (dist 6.4 -> 6.9), then from dist
  7.15 a walk at 0.0485 (x1.0) for ~17 s, out to 15.9 blocks. So it sprints below ~7 blocks and
  walks beyond. FX resent unchanged at 81383, 86033 and 93433. Never bit.

## Weak (rt 7)

- Health 10 -> 9. Panic 104639-105784 (~24 ticks) at 0.0751, 3.66 -> 4.42 blocks (mostly -x).
  Idle 13 s, then a stroll at 0.0307 (fits mult 0.8 under the law). Never bit.

## Playful (rt 8)

- Panic 133887-138634 (~95 ticks) at 0.0753 (0.0726 at the end), 3.5 -> 9.6 blocks.
- Roll at 147333-148932 (+15 s after the hit): motion (-0.0316, 0.27, 0.1975) = 0.2 along its
  facing, 0.2000 a tick for 32 ticks (4 hops). Never bit.

## Death and open questions

- Death, the same for every panda: DeathAnimation and an empty MobEquipment pair, then TargetEid 0
  and health 0 about 1 tick later, then RemoveEntity 21 ticks later.
- Still open: why rt 3 and rt 6 re-rolled their Variant (and which groups they then had); why no
  panda got ANGRY or bit once here; and the bite-3 stall. Broadcast anger needs a bystander that keeps Variant 6.
