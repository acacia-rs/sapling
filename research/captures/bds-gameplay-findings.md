# Gameplay leftovers: what BDS did

Drills in `crates/sapling/examples/shared/gameplay_drill.rs`; captures beside this file.

## Mace (bds-mace.txt, bds-mace-push.txt, bds-mace-breach.txt)

- A hit smashes when the hitter has fallen 1.5 or more (fall = peak minus height at the hit).
  Damage = (6 + bonus) x 1.5 critical, bonus 4 a block for the first 3, 2 for the next 5, 1
  after; Density adds 0.5 x level a block. Fits: 1.593 → 18.556, 3.346 → 28.039, 6.594 → 37.781,
  15.909 → 53.864 off a golem. Fall 0.46: plain critical 9.
- Packets, smash tick: target `HurtAnimation` data 34 (`MaceSmash`), `Animate CriticalHit` 55,
  hitter `SetEntityMotion` (0,0,0) to it and viewers, `LevelEvent
  AddParticleSmashAttackGroundDust` at the target's feet; the tick after, `mace.smash_ground`
  (fall ≤ 5) or `mace.heavy_smash_ground` at the hitter's eyes, entity_type "", unique -1.
  Landing afterwards is a `land`, not a fall: the fall is forgotten.
- Wind Burst I/II/III: `SetEntityMotion` y 1.1 / 1.63 / 2.06 after the (0,0,0),
  `ParticleWindExplosion` data 3 at the hitter's feet, `LevelEventGeneric` 4052 radius 3, size 0,
  `wind_charge.burst`.
- Mobs near the target are shoved: told y 0.43276805 (0.6 up from resting, after a tick) whatever
  the fall; across 0.6 x (2.5 − d) away from the target (inward past 2.5), told after ground drag
  0.546. Five cows at d 2.21–2.81 fit to 1e-3; two at 3.5 and 3.8 got ~0.39 not 0.6 a block (not
  fitted).
- Breach: armour's share less 0.15 a level. A mace's 6 on a zombie in a diamond chestplate (10
  points, 2 toughness): 4.176, 5.076, 5.976, 6, 6 at Breach 0 to 4.

## Powder snow (bds-freeze.txt)

- Freeze +1/140 a tick in powder snow, −1/70 out of it or wearing any leather piece.
  `FreezingEffectStrength` told each tick it changes. Full: `HurtAnimation` 27, Flags bit 40 and
  strength 1.0 together, 1 damage then every 40 ticks.
- Movement attribute: modifier "Freeze effect" −0.05 x strength, told a tick behind it.

## Soul Speed, Swift Sneak, Aqua Affinity (bds-soles.txt)

- Soul Speed III on soul sand: `UpdateAttributes` movement 0.1615 each tick (+0.03 x (1 + 0.35 x
  level) as "Sprinting speed boost"); 0.1 with no modifiers once, stepping off.
- Swift Sneak: nothing from the server beyond the sneak flags; the client moves itself.
- Aqua Affinity: dirt by hand with eyes in water `BlockStartBreak` 873 (65535/75), with Aqua
  Affinity 4369 (65535/15).

## Horse jump (bds-horse-jump.txt)

- BDS rolls each horse's health (whole, 15–30), movement (0.1125–0.3375) and
  `horse.jump_strength` (0.4–1.0) as its pack says, and announces them in `AddEntity`.
- The charged jump is the rider's client's: the horse's height comes in the rider's input. BDS
  sets Flags bit 39 (rearing) on the horse at take-off and clears it on landing, told to the rider
  and viewers.

## Frog on land (bds-amphibians.txt, frog 6)

- Leaps at 70° (told y/across 2.747): first one to the block 1 across and 4 along, level with its
  own, at 0.689; v² = g·e² / (e·sin 2θ − 2f·cos²θ) with e to the block's centre and f = −1 gives
  0.687. Told every 3 ticks: across constant, y 0.24 less (gravity 0.08, no drag). On landing
  (x, −0.08, z) a tenth of the speed across.
- `jump_to_block` sound at its feet a tick after the leap; FlagsExtended bit 39 (as a breeze's
  leap) from the leap to the landing. Waits from landing to the next leap 7.5–9.2 s (pack
  cooldown_range 5–7 s).

## Squid swimming (bds-swimmers.txt, squid 20, glow squid 21; bdsre on 1.26.52)

- `Squid::postAiStep` (0x1446f8c90, found by its 0.99/0.9/0.8/0.75 tentacle constants) in water,
  server side: speed += (way − speed) × 0.22 each axis, after the water's 0.8; steps close 0.624
  of the gap a tick, as recorded (0.00537, 0.00665, 0.00746 … 0.00879). Yaw += (−atan2(x, z)° −
  yaw) × 0.1, pitch += (−atan2(across, y)° − pitch) × 0.1, unwrapped. Out of water: x, z 0,
  y = (y − 0.08) × 0.98. Tentacle speed 0.2/(r + 1) scaled by air/max air (×1.75 a baby); the
  tentacles only animate.
- Goals: the way (Squid+0x494) is written by four goal functions, found as the only ones storing
  to 0x494, 0x498 and 0x49c; their vtables at 0x14a994700:
  - `squid_idle` (tick 0x149ff7580, canUse: in water; start sets its count to 200): each tick the
    count rises; at 200 or more, or with the squid above its band and the way rising, a new way:
    x, z = rand − 0.5, y = ((high − low) × rand + low − y) × 0.1, scaled to 0.5 and / 20. The band
    (0x142f4ff40): from the squid's cell up to the first non-water block `top`; high = top − 1.5,
    low = top − 5, each raised to floor + 0.5 where under the column's floor.
  - `squid_flee` (tick 0x149ff70a0, canUse: hurt by a mob within 10 blocks, in water): away from
    it, × 3 (less (d − 5)/5 past 5), no rise above the band, / 20; `squid_flee_particle` every 10th
    tick from the 5th.
  - `squid_dive` (start 0x149ff6b50, out of water over a non-solid block): (rand − 0.5, −1,
    rand − 0.5) normalised × 0.5 / 20. `squid_out_of_water` (start 0x149ff7d50) and
    `squid_move_away_from_ground` (start 0x149ff7b80, on colliding) were not followed further.
- The steady drift 0.0146 = 0.22 × 0.025 / 0.376, the 0.025 way the idle goal picks.
- Spurts, not found: over ~5 s each squid got a push every ~12 ticks (glow squid 8, later 14-15):
  −0.0715 along z every time for both squids (whichever way they faced or drifted), across x
  sliding from +0.031 to −0.065 (the residual of v' = 0.624 v + 0.376 drift).
  No squid goal or postAiStep writes speed but the lerp; MOVING (flag 34) follows each push as the
  speed rule of docs/research/moving-flag.md would.
