# bds-shulker findings

Capture: `research/captures/bds-shulker.txt` (drill `shulker`). Times are the capture's first column (ms).
Ticks come from the actor's UpdateAttributes/SetEntityMotion/MobEffect `tick` fields. Bullet ticks are
counted in MoveEntityDelta packets (one a tick, none skipped) back from the hit tick, so they are exact.
Ms-only events are +-1. Actor runtime 1, unique -601295421439. Easy, midnight, stone floor top -19.
The `instant_health` heals (health back to 20 each second) are left out below.

**Only the 6-off run is usable.** Levitation lifted the actor ~23 blocks, `/effect clear` at t662
dropped it and it died of fall damage (t690, before the second summon). The 12-off shulker was summoned
next to the death spot and the actor then respawned out of its range, so it never targeted or shot
(see Phase 2). To capture it, prevent the fall death (`/gamerule falldamage false`, or tp back to the
floor) and tp before the second summon.

## Shulker 1 (rt 6, unique -601295421437)

- AddEntity 9064 at (6.5, -19, 14.5), velocity 0, yaw/pitch 0, health 30, movement 0. Metadata: Variant
  16, Color 0, ShulkerPeekId 0, ShulkerAttachFace 0 (down), ShulkerAttached 1, ShulkerAttachPos
  (6, -19, 14) (its own cell, sent in the next-tick SetEntityData), box 0.6 x 1.8 (never updated).
  Flags bits 22, 48, 49, 50, then bit 35 (breathing) is added 1 tick later. `spawn` sound at y +0.392.
- Target: 9504 (~t238), one SetEntityData: TargetEid = actor, **ShulkerPeekId 100**, FlagsExtended
  gains bit 24. Next tick (9560): `mob.warning` (shulker, y +0.392) and `shulker.open` (y +0).
- Peek stays 100 through all 4 shots. **No open/close per shot.** Target lost 17955 (t404-405):
  TargetEid 0, Peek 0, FlagsExtended bit 24 cleared, and `shulker.close` the next tick (18004). The
  reconstructed actor (below) was ~15.2 blocks away then, so it had just left the 15 attack_radius/
  target range. must_see was not the cause: the view was open air.
- Idle peeks after that: Peek 30 at 26570 (t574), Peek 0 at 29525 (59 ticks later), Peek 30 at
  30026, killed 30040. Each change gets `shulker.open`/`shulker.close` exactly 1 tick later. A
  `shulker.open` follows the death (30075).
- Pitch (MoveEntityDelta rot_x, 256 units) tracks the rising actor: 249 (-9.8 deg) at target, then 252,
  249, 246 ... 231 (-35.2 deg) at 16955. It goes back to 0 by 18255 after the loss, then idles head yaw
  (rot_z sweeps 53 -> 46 -> 32 -> 17 -> 0). **Never moves or teleports**: no x/y/z in any of its 27
  deltas, and none of the `disappeared`/`reappeared` sounds are its own (see Sounds).

## Bullets (4, rt 7-10, all OwnerEid = shulker, no TargetEid in metadata)

| # | rt | spawn | hit | flight | legs (axis, ticks) | hit point |
|---|----|-------|-----|--------|--------------------|-----------|
| 1 | 7 | 11406, t276 | 12660, t301 | 25 t | +z 25 | (6.5, -18.308, 19.9) |
| 2 | 8 | 13255, t312 | 16705, t381 | 69 t | +z 20, +y 26, +z 10, +z 12 | (6.5, -12.777, 22.725) |
| 3 | 9 | 15355, t353 | 21325, t471 | 118 t | +y 20, +z 10, +y 20, +z 30, +y 18, +z 19 | (6.5, -7.019, 27.169) |
| 4 | 10 | 17654, t398 | 24976, t543 | 145 t | +y 10, +z 20, +y 40, +z 40, +y 21, +z 13 | (6.5, -2.082, 31.609) |

- Spawn: AddEntity at **(6.5, -18.308, 15.5) for all 4** = shulker + (0, +0.692, +1.0), even when the
  first leg is +y. Velocity 0, box 0.3125 x 0.3125, LimitedLife -1, Flags bits 5 (INVISIBLE), 49. Next
  tick: Flags bits 14, 34, 49 (visible, moving; no collision bit 48). The first MoveEntityDelta is
  in the AddEntity's batch, so the bullet moves on its spawn tick.
- x stays exactly 6.5 throughout. The motion is **Java's ShulkerBullet, exactly** (fits every step to
  1e-4):
  - each tick `T *= 1.025`, then `v += (T - v) * 0.2`, then `pos += v`. No gravity, no drag.
  - on a re-pick, `T = 0.15 * unit axis direction` (the new T is 0.15375 on its first tick, every time).
    The old axis's speed then decays x0.8 a tick, which is the rounded turn.
- Speed along a fresh leg: 0.0307, 0.0561, 0.0772, 0.0949, 0.1098, 0.1227, 0.1338, 0.1436, 0.1523,
  0.1603 (tick 10), 0.2049 (tick 17), 0.2529 (tick 25). Fastest seen 0.2952 (the 40-tick legs).
- Bullet 1 (6 off, actor on the ground at z 20.5, same x, bullet already in the target's y cell): one
  straight +z leg of 4.4 blocks in 25 ticks.
- Re-pick triggers: leg lengths 10/20/30/40 = flightSteps `10 + 10 * nextInt(5)` running out. The
  others (26, 18, 21) end on the tick the bullet's cell on the leg's axis equals the target's. With
  the reconstructed actor y, that matches the target's **centre** cell (feet + 0.9), where Java uses the
  feet cell. The y model is unverified, so treat this as tentative.
- Differences from Java: bullets 3 and 4 start +y, so the first pick does **not** exclude the attach
  face's axis (Y), which Java's constructor does. Bullet 2's leg-3 expiry re-picked +z on the same
  axis, which fits Java's random fallback when no axis is left to close.
- Bullets keep homing after the shulker drops its target (bullet 4 flew 138 ticks past t405).
- Pitch (rot_x) follows the motion with lag: 6, 15, 24, 32 ... 63 (88.6 deg) climbing, back to 0 on
  +z legs. Occasional SetEntityMotion carries that tick's step exactly (e.g. (0, 0, 0.20487696)).
- Flight time at 6 off: 25 ticks to a standing actor. The later bullets' 69-145 ticks are chases of
  a levitating target, not a clean 12-off number.

## Hits on the actor

| hit | tick | health | HurtAnimation data | SetEntityMotion | levitation |
|-----|------|--------|--------------------|-----------------|------------|
| 1 | t301 | 20 -> 16 | 3 (projectile) | (0, 0.4, 0.4) | Add id 24 amp 0 dur 200 |
| 2 | t381 | 20 -> 16 | 3 | (0, 0.4, 0.4) | Update, dur 200 |
| 3 | t471 | 20 -> 16 | 3 | (0, 0.4, 0.4000412) | Update, dur 200 |
| 4 | t543 | 20 -> 16 | 3 | (0, 0.4, 0.40022495) | Update, dur 200 |

- **Damage 4 on easy** (unscaled). One batch holds the motion, the HurtAnimation and the MobEffect, and
  the health UpdateAttributes (same tick field) follows within ~25-50 ms. VisibleMobEffects 48 a tick later.
- Knockback: `vxz = old/2 + 0.4 * dir`, and **y = 0.4 even in mid-air** (Java sets y only on
  ground). 0.4000412 = 0.4 + (0.4 * 0.91^89)/2, the leftover of the previous knock. dir was +z
  every time: that is both the bullet's last leg and shooter -> actor, so which one sets it is not pinned.
- The levitation is refreshed with `Update` (not a new `Add`) at 200 ticks on each hit. `/effect clear`
  at 31045 removed it with duration 81 left (t662 = 543 + 200 - 81).
- Bullet removal: RemoveEntity 1 tick after the hurt, with `bullet.hit` (entity_type "", unique -1)
  at the hit point. No LevelEvent/particle at any hit (the only LevelEvent is unrelated, below). No
  MoveEntityDelta is sent for the hit tick. The hit point is the ray from the bullet's centre against
  the actor's box **inflated by 0.3** (face at actor z - 0.6): bullet 1 hits at 19.9 = 20.5 - 0.6.
- The actor's Flags gain bit 34 (moving) 1 tick after hit 1 and keep it while airborne.

## Actor position (server-simulated)

No packet carries it, but the server moves the probe's server-side body. The bullets chase it, and
this model reproduces every hit z to 1e-4 (23.3253, 27.7695, 32.2094) and the death z 36.65634:
- z: knock at t301 from the ground: first tick friction 0.546, then 0.91 a tick in the air, so the
  knock covers 0.4 + 0.4 * 0.546 / 0.09 = 2.827. Each mid-air knock covers 0.4 / 0.09 = 4.44.
- y (fits the hit heights, feet + 0.95 to 1.01): `vy = 0.98 * (vy + (0.05 - vy) * 0.2)`, terminal
  0.0454 a tick (~0.9 blocks/s). Feet ~-13.7 at t381, -8.0 at t471, -3.1 at t543, ~+3.9 at t662.
- After the clear at t662 it fell ~23 blocks: watcher-only `fall.big` + `fall` (extra_data
  -2144268767, a block id) at 32525. DeathAnimation + DeathInfo `death.fell.accident.generic` at 32471
  (t690), PlayerLastDeathPosition (6, -18, 36). Respawn 34585.

## Shots and timing

- First shot 37-38 ticks after taking the target (9504 -> 11406). Shot to shot (exact, from the bullet
  ticks): **36, 41, 45 ticks** (t276, t312, t353, t398), inside attack_interval 1-3 s (20-60).
- Each shot is one batch: AddEntity, `disappeared` (bullet), `undefined`, `shoot`.

## Sounds

- `shoot`: shulker's (entity_type shulker, its unique id) at shulker + (0, 0.99, 0), on the spawn tick.
- `undefined`: no entity (entity_type "", unique -1), at the same point as `shoot`, same tick. One per shot.
- `disappeared` / `reappeared` (5 each per stream; 10 = both streams): they follow **Flags bit 5
  (INVISIBLE)**. 4 pairs are the bullets': `disappeared` on spawn (bit 5 set, at shulker + (0, 0.9, 0)),
  `reappeared` 2 ticks later at the bullet + (0, 0.15625, 0), 1 tick after bit 5 clears. The 5th pair is
  the actor's: `disappeared` at 33518 (dead, bit 5 set) and `reappeared` at 34625 (respawned, bit 5 cleared).
- `shulker.open` / `shulker.close`: 1 tick after each Peek change (100 or 30 -> open, 0 -> close).
- No hurt, ambient or death sounds from the shulker. `bullet.hit` only at hits.

## Phase 2 (12 off): no data

- `/summon shulker ~ ~ ~-12` ran with the actor dead at z 36.656: rt 11 AddEntity at (6.5, -19,
  24.656342), snapped to z 24.5 the next tick (watcher MoveEntityDelta, Flags bit 34 for one tick). The
  actor respawned at 34585 and lost it (RemoveEntity 34676, along with the watcher, unique ...438).
- It never set TargetEid and never shot. Idle only (watcher view): Peek 30 for 19, 39, 19, 59 ticks
  (= 20 * (1..3) - 1), with head-yaw sweeps. The actor still got its open/close sounds at
  (6.5, -19, 24.5). Killed at 55071.

## Other (noise)

- 62 `block.campfire.crackle` per stream. Sculk sensor `power.on`/`power.off` at (4, -40, 4) 21920/22918.
- LevelEvent ParticleProjectileHit data 96 at (-1.5, -27.5, 0.5) at 21172 plus an `open` sound there
  at 21226: away from the drill, unrelated.
- The opening `/kill` reported "Totem of Undying, Experience Orb, Experience Orb": leftovers from an
  earlier drill.
