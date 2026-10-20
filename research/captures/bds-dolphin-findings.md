# bds-dolphin findings

Capture: `research/captures/bds-dolphin.txt` (drill `dolphin`). Times are the capture's first column (ms),
~50 ms a tick. The dolphin's own packets carry `tick: 0`, so dolphin ticks are counted from its
MoveEntityDelta packets (one a tick; only gap: 3 ticks arrive together at 25186-25188). Bite intervals are
ms / 50: the actor's `tick` field runs ~2% slow (601 -> 1175 over 585 real ticks), so its 19/22 steps are
artifacts. Actor rt 1, dolphin rt 3 (unique -644245094397). The dolphin's packets are identical in both
streams. Water: blocks y -26..-20, floor top y -26, surface y -19, pool interior x/z -6..7. Fits below
were fitted to MoveEntityDelta deltas with scripts and hold to ~1e-4 unless marked.

## Spawn (11035)

- AddEntity at (0.5, -25, 3.5), velocity (0, -1e-5, 0). Attributes: health 10, movement 0.1,
  underwater_movement 0.15, lava_movement 0.02. Box 0.9 x 0.6. **Air 4800, MaxAir 4800**
  (total_supply 240 s x 20). Flags bits 19, 20, 48, 49; FlagsExtended bits 25, 63.
- `ambient.in.water` at spawn + (0, 0.51, 0) (eye height 0.85 x 0.6). AmbientSoundEventName
  "ambient.in.water" is set in metadata 3 s later (14043). There are no ambient sounds after that.
- **Air drops 1 a tick for the whole time underwater** (4800 -> 3981 by 51885), with no packets
  between the decrements. It goes back to 4800 in one SetEntityData on the breach tick (see Breach).
  `swim_up_for_breath` never fired: Air never got low.

## Movement model (water), exact for the calm swim, the chase and passive drift

- Per tick: `pos += v; v *= 0.8` (**drag 0.8 on all three axes**), then the next tick adds the swim
  push `b = 0.0220` along the heading. Terminal speed is 0.022 / 0.2 = **0.110 a tick**. Straight legs
  measure 0.1092-0.1102. Speed from rest: 0.031, 0.056, 0.076, 0.091, 0.103 ...
- SetEntityMotion carries v after the drag: knock tick delta (-0.4004, 0.3999, -0.2267) x 0.8 =
  sent (-0.32029, 0.31995, -0.18133). The next delta is that plus 0.022 x heading.
- Calm, sinking or rising: horizontal push 0.0220 along yaw **plus a separate vertical push**:
  -0.022 while it sank to the floor (total b = 0.0311 = 0.022 x sqrt 2), and +0.0140 -> +0.0177
  (growing) while it rose at 17283-18083. Chase: one push of 0.0220 along the 3D direction it faces
  (pitch included), e.g. u = (0.67, 0.74, 0.01).
- **No gravity or buoyancy in water.** On the floor y stays exactly -26.000 (on_ground true). A
  straight mid-water leg holds vy -0.0155 for 40+ ticks without drifting.
- **Surface:** whenever the feet reach about -20.0 (box top 0.4 above the surface) the next delta
  gets an extra ~-0.086 vy (32142, 38937 ...), so a chase at the surface bobs between feet -20.6 and
  -19.97. It never left the water this way, only by the breach.
- **Slowness 255 did nothing in water**: movement went to 0 (MovementSlowdown -38.4, op 2) but
  underwater_movement stayed 0.15, and the speed stayed 0.1092 through 22035-25186.
- Rotation (256 units a turn): rot_y steps **7 units (9.84 deg, a 10 deg cap) a tick** while turning
  (260 of 423 yaw steps). rot_z is rot_y one tick behind early on, then equal to it. Pitch rot_x
  stays 0 on the floor. Rising: 253 -> 239 (-4.2 to -19.7 deg) at 3-4 units a tick. Chase: 217-255.

## Calm swim (11035 -> 25186)

| span | what |
|------|------|
| 11036-11883 | sinks from -25 to the floor (-26) in 17 ticks while turning +7 a tick |
| 11883-17191 | 106 ticks on the floor (g=t), 0.087-0.107 a tick, wandering loops and S-turns around (-1.6..1.2, -1.5..4.2) |
| 17234-19300 | Flags +57 (SWIMMING), rises to y -23.74 (the calm max) heading -z to the wall (z -5.24) |
| 19300-23034 | turns back. Straight +z leg 20800-23034: d = (0.0065, -0.0155, 0.1090) every tick |
| 23100-25186 | turns 90 deg in ~12 ticks, then straight -x at 0.110, y -24.99 at the hit |

- The calm y range is -26.0 to -23.74: **it never came within 4.7 blocks of the surface and never
  breached in 14 s**. Course changes come every ~3-5 s (16283, 17234, 18338, 19793, 23294). With
  random_swim interval 0, the target points cannot be read off the packets.
- Flags bit 34 flickers (a clear and a set in the same tick, for 2-5 ticks) around each re-plan
  (16233, 19838, 23193) and often while stuck in the corner later. Otherwise it stays set.

## The hit (25186, the actor's fist)

- 25230 HurtAnimation data 2. 25233 health **10 -> 9**. 25234 TargetEid = actor and SetEntityMotion.
  25293 (+1 tick) **Flags bit 25 (ANGRY)** and `mob.warning` (dolphin) at the last pre-knock position
  + (0, 0.51, 0). Watcher-only `attack.strong` (player) at (-1.3129, -24.5107, 4.3811), the actor.
- Knock: the knock tick's delta is a normal swim step plus **(-0.291, +0.400, -0.214)**. The xz part
  (0.361) points from the actor's `attack.strong` x/z to the dolphin to 1e-3. Equivalently,
  y = 0.4 + old_y / 2 (0.39985). The xz rule is not pinned (0.41 after removing old/2 instead).
- After that the plain drag-0.8 model holds. It hit the -x wall (x -5.55 = -6 + 0.45; SetEntityMotion
  x 0.0 at 25483) and topped out at y -23.2.

## Bites (melee_box_attack)

| # | ms | +ticks | dolphin feet | actor knock (x, y, z) |
|---|----|--------|--------------|-----------------------|
| 1 | 27938 | - | (-2.047, -24.563, 2.844) | (0.1724, 0.4, 0.3610) |
| 2 | 28985 | 21 | (-1.060, -24.135, 4.669) | (0.1495, 0.4, 0.3725) |
| 3 | 29984 | 20 | (-0.551, -22.949, 6.154) | (0.3354, 0.4, 0.2196) |
| 4 | 30983 | 20 | (0.452, -21.590, 6.545) | (0.3998, 0.4, 0.0415) |
| 5 | 32187 | 24 | (2.404, -20.098, 6.509) | (0.3981, 0.4, 0.0499) |
| 6 | 33635 | 29 | (5.170, -20.216, 6.500) | (0.3966, 0.4, 0.0518) |
| 7 | 34633 | 20 | (5.882, -20.050, 6.341) | (0.3662, 0.4, 0.1609) |
| 8 | 36387 | 35 | (6.147, -20.611, 5.166) | (0.1356, 0.4, 0.3763) |
| 9-23 | 37385-51383 | 20 each | around (5.3-6.5, -20.3..-19.99, 5.2-6.5) | xz 0.4000, y 0.4 |
| 24 | 55183 | 76 | (6.516, -21.945, 5.194) | (0.0194, **0.32**, 0.5985) |
| 25 | 56193 | 20 | (5.738, -21.457, 5.197) | (0.3077, 0.32, 0.5149) |
| 26 | 57193 | 20 | (5.424, -20.527, 5.161) | (0.3427, 0.32, 0.4912) |

- First bite 55 ticks after the hit (2.75 s spent recovering from the knock and closing ~3 blocks).
- One batch per bite: dolphin ArmSwing, the actor's SetEntityMotion, the actor's HurtAnimation **data 2**.
  The actor's health UpdateAttributes follows 0-1 tick later. **Damage 2.5 on easy** (20 -> 17.5
  every time; attack 3 -> 3/2 + 1). The 1 s instant_health resets health to 20 between bites.
- `attack` sound (entity_type dolphin) 1 tick after the swing, at **the dolphin's swing-tick
  position + (0, 0.3, 0)** (half the height; checked on bites 1 and 23).
- **Cooldown 20 ticks.** The long gaps are out-of-reach stretches. From 32683 the actor is pinned in
  the (6.7, 6.7) corner (every splash/underwater sound is at x = z = 6.7). Against that corner, the
  **xz box gap** (max over axes of |d| - 0.45 - 0.3) is **<= 0.789 at all 21 corner bites**. Bites 6
  and 8 (gaps 0.780, 0.784) land on the first tick back in reach after stretches with gaps up to
  1.42 / 1.26 (33343-33583, 35583-36337). That fits horizontal_reach 0.8: dolphin box + 0.8 on x/z
  must touch the actor's box. The vertical rule is untested (no actor y in any packet).
- Actor knock: **xz = 0.4 x (dolphin -> actor)** plus old/2 (|xz| 0.4000-0.4019), **y = 0.4** even
  in water (the knock lifts it out: splash ~0.45-0.5 s after each bite, then underwater.enter/exit).
  Bites 24-26 are **xz ~0.599, y 0.32**. They come after the breach set dolphin Flags bit 3
  (SPRINTING) at 51885, which is never cleared. That fits a sprint-knockback path but is unproven.
- Chase speed is the calm speed (push 0.022, terminal 0.110). There is no attack-speed bonus.

## Breach (51885-52291, during the chase)

- 51885: Flags +3, SetEntityMotion (-0.0968, 0, -0.4422) (xz 0.453, heading away from the corner).
  51940: delta (-0.098, **+0.620**, -0.450), Flags -57. 51991: **Flags +35 (BREATHING), Air 4800**.
- Out of water 7 ticks. Peak feet -18.748 (0.25 above the surface). In air: xz x0.91 a tick, and
  vy fits `vy' = 0.98 vy - 0.176` over 4 ticks: much stronger than gravity, cause unknown.
- 52291: `splash` (dolphin, extra_data 1672429) at the previous tick's position + (0, 0.3, 0),
  Flags -34 -35. After that: vy x0.80 a tick, xz 0.8 v + 0.022 push. Flags +57 again at 52993 (14 ticks).
- Air was 3981 (not low), so this is random_breach and not swim_up_for_breath. It cost a 76-tick bite gap.

## Calming: none

- Bit 25 (ANGRY) and TargetEid stayed set from 25293 until the kill at 57237 (**32 s, past the
  25 s anger**). The bites never stopped. No packet shows a calm.
- Kill: DeathAnimation 57237 with an empty MobEquipment pair (main hand, offhand), then health 0,
  TargetEid 0 and Flags +34 (57254). RemoveEntity 58288 (21 ticks).

## Other

- No dolphin hurt, death, ambient (after spawn), flop or breathe sounds. Its only sounds are
  `ambient.in.water` x1, `mob.warning` x1, `attack` x26 and `splash` x1.
- Watcher-only: `attack.strong` at the hit, and 2 player `land` sounds at 59393/60040 (cleanup).
  Otherwise the watcher matches the actor stream for the dolphin.
- Actor HurtAnimation data 4 (-2, -2) at 59587/60084: the cleanup `/fill ... stone` refills the pool
  around the actor (suffocation). Not dolphin.
- Sculk sensor power on/off pairs and campfire crackles are noise.
