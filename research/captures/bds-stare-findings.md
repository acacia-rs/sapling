# bds-stare findings

Capture `research/captures/bds-stare.txt` (drill `stare`). Times are the capture's first column (ms,
~50 ms a tick). The actor and watcher streams carry the same enderman packets within 1 ms, so only the
actor's are quoted; the watcher adds nothing. Easy, midnight, survival. Actor feet (0.5, -19, 0.5)
(every `summon ~ ~ ~6` lands at (0.5, -19, 6.5)), eyes (0.5, -17.38, 0.5), stone floor at y -20.
Each `!look` sets the bot's yaw/pitch at once (`Controls::look_at`), so it takes effect on the next
client tick; `##` lines carry no time, so "look at" below is the window between the neighbouring packets.

## Result: one trigger in six, and it pins the rule

Angles are between the look ray (eyes to feet + (side, 2.6, 6)) and the enderman's head at
feet + 2.465 (where BDS plays its sounds; 0.85 x 2.9). Distance eyes -> head 6.06.

| # | side | look at | enderman during the look | off head | ray vs box | result |
|---|------|---------|--------------------------|----------|------------|--------|
| 1 | 0   | 13986-14032 | at (0.5, 6.5) 3 ticks, then teleported (14184) to (13.98, -19, -9.31) | 1.26, then 125 | hit | none |
| 2 | 0.5 | 26082-26132 | at (0.5, 6.5) | **4.87** | misses by 0.17 | **target 26333** |
| 3 | 1   | 38032-38582 | at (0.5, 6.5) the whole 3+ s (no teleport for 8 s) | **9.42** | misses by 0.64 | none |
| 4 | 1.5 | ~50240-50333 | teleported 50232, just before the look, to (16.33, -19, 5.25) | 59 (13.9 planned) | miss | none |
| 5 | 2   | 61482-62883 | at (0.5, 6.5) ~10 ticks, then teleported (62883) to (-5.01, -27, 2.17), under the floor | 18.25, then 98 | miss | none |
| 6 | 3   | after 73633 | teleported 73537, before the look, to (-11.74, -19, 7.33) | 87 (26.3 planned) | miss | none |

- **Threshold between 4.87 and 9.42 deg** at 6.06 (to Java's eye height 2.55: 4.72 / 9.35). The two
  clean samples are trials 2 and 3 (enderman in place for the whole 0.25 s and longer).
- Java's `isLookingAtMe` (dot > 1 - 0.025 / d) gives **5.21 deg** at 6.06: fits. A ray-vs-box rule
  does not: trial 2's ray passed 0.17 outside the 0.6 x 2.9 box (1.8 deg off its nearest point) and
  still triggered. (Bedrock's `looked_at` documents `field_of_view` 26 with `scale_fov_by_distance`;
  not checked against this pack. 26 deg unscaled is ruled out by trial 3.)
- The enderman's own facing doesn't decide it: in trial 2 its head was still 38 deg off the actor
  (head yaw byte 155; 128 = facing the actor), in trial 3 it faced the actor exactly (128 from 39883)
  and nothing happened.
- **Trial 1 failed because the enderman left**: looked at dead centre (1.26 deg, ray in the box), but
  it teleported 3 ticks after the look line, under the 0.25 s (5 tick) `min_looked_at_duration`, with
  no TargetEid. Whether the look caused that teleport can't be told; calm teleports are this frequent
  anyway (below). Trials 4 and 6 left before the look, trial 5 was never within 18 deg.

## Trial 2: stare -> target -> anger -> teleport away

- Look line 26082-26132; effective by ~26180. **26333: TargetEid = actor** (-627065225215: two below
  the first mob's id, the same offset as the actor in bds-ender). 4-5 ticks after the line, 3-4 after
  the look took effect: the 0.25 s duration, give or take the tick the look landed on.
- **26388 (next tick)**: `mob.warning` and **`stare`** LevelSoundEvents at the enderman's feet + 2.465;
  Flags + bit 25 ANGRY (844459294588928 -> 844459328143360); movement default 0.3 -> 0.45 (current
  still 0 under slowness 255). Body yaw starts turning, bytes 235, 214, 192 (~30 deg a tick).
- **26486 (3 ticks after the target, 2 after ANGRY)**: teleport 16.82 to (-9.08, -19, -7.32),
  6.00 -> 12.37 from the actor: away, not toward. No attack (12 off, slowness 255).
- The actor kept looking at the empty spot to ~30.3 s. Teleports while angry:

  | ms | from actor (horiz / 3D) | jump | fits teleportTowards? |
  |----|-------------------------|------|------------------------|
  | 31186 (94 t after) | 12.37 -> 28.64 / 50.01 | 16.55 horiz, down 41 to y -60 | no (random, target under 16) |
  | 31337 (3 t after) | 28.64 -> 24.30 / 47.66 | 4.47 | yes: 16 along the 3D line, offsets x -3.92, z -2.95 |
  | 32034 (14 t after) | 24.30 -> 16.70 / 44.27 | 9.40 | yes: offsets x +3.33, z -3.08 |

  So toward-teleports fire 3 and 14 ticks after the target got over 16 (3D) away, not after a fixed
  30 ticks as bds-ender's single sample suggested. Distance is 3D: 16.7 horizontal but 44 in 3D still
  counted as far.
- Killed at 32302; TargetEid cleared to 0 at 32333 (same as bds-ender). The 25 s anger wasn't reached.

## Other notes

- **Spawn teleport is not every time**: trials 3-6 teleported in the AddEntity tick (+1-2 ms);
  trial 1 at +6 ticks (10332); trial 2 never did (still at (0.5, 6.5) at the tp). Revises bds-ender's
  "5 of 5".
- Calm teleports (all under slowness 255, so slowness doesn't stop them): 8 in 42 s of trial windows
  (tp-back to kill), mean ~5.3 s, like bds-ender run 1 (5.2 s). Trial 3's enderman made none in 8 s.
  Trial jumps 10.6-20.8, per-axis within +-16, onto the ground below (y -60 or -27).
- A dying enderman still teleports: trial 4's at +10 ticks after DeathAnimation (56833), trial 5's
  twice (68782, 68942) before RemoveEntity.
- Head turning (look-at-player, at the actor): after each tp back, head yaw byte 249 stepping -7 a
  tick (9.8 deg) in bursts, then -4/-5, settling at 128 (facing the actor) after ~3.5 s (trial 3).
  Head pitch byte 7 (9.8 deg down), as in bds-ender. Trial 5's head instead turned to +z (247 -> 17)
  and the body slid 0.0008: something else held its look (the watcher?).
- No `mob.endermen.portal`, idle, scream or hurt sounds; teleports send LevelEvent
  SoundEndermanTeleport at origin and destination, as before.

## Sapling should

- Stare trigger: a player within 64, not wearing a carved pumpkin, whose look direction is within
  Java's `1 - 0.025 / d` dot of the enderman's head (feet + 2.465; 5.2 deg at 6) for 5 ticks
  (0.25 s). Measured bracket 4.87-9.42 deg at 6.06; no box test.
- Then: TargetEid that tick; next tick ANGRY, movement 0.45, `mob.warning` + `stare` at feet + 2.465;
  random teleport 2 ticks later (seen once, 16.8, away).
- Teleport toward the target when it is over 16 off in 3D (16 along the line, +-4 per axis, onto
  the ground); delay 3-30 ticks seen, not fixed.
- Spawn teleport: usually the first tick, not always. Dying endermen keep teleporting.
- Next drill: more angles between 5 and 9 deg (e.g. sides 0.6-0.9) and other distances to separate
  Java's 0.025 / d from a scaled-FOV rule; a tp-and-hold that re-tps the enderman just before the look.
