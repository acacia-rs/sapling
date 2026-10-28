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
