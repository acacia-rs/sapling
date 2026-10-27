# bds-wander findings

Captures: `research/captures/bds-swimmers.txt`, `bds-flyers.txt`, `bds-grazers.txt`,
`bds-amphibians.txt` (drills of the same names, `crates/sapling/examples/shared/wander_drill.rs`),
BDS 1.26 (`bds-molang` build) on devbox. Speeds below are per tick, fitted from MoveEntityDelta
steps as `v' = k·v + a` over each mob's track (one delta a tick for these near watchers). Pack values
are the mobs' entity files in the vanilla behaviour pack (`mob-traits.py`).

## Fish (cod, salmon, pufferfish, tropical fish)

- Drag **0.8 on all three axes** in water, no gravity (as the dolphin, bds-dolphin-findings.md).
- Push along the body's yaw = `0.98·s²` with `s` = `underwater_movement`: cod 0.1 → 0.0098,
  salmon and tropical fish 0.12 → 0.0141, pufferfish 0.13 → 0.0166; exact to 1e-5. No ramp: the
  first step after a turn is already full strength. Terminal 5×push (cod 0.049).
- Vertical push ±1.25× the horizontal one while far from the target height (cod ±0.01225,
  salmon ±0.0176, pufferfish ±0.0207); smaller near it (not fitted).
- Yaw turns at most 30° a tick toward the target. They never stop (`random_swim` interval 0,
  xz 16, y 4) and never left the pool.

## Squid, glow squid

- Constant 3D drift: push 0.00293 along a unit direction, drag 0.8 → 0.0146 a tick. The
  direction changed about every 200 ticks (two samples; y shares +0.336 and +0.219).
- Spurts (+~0.08 along −z every 12 ticks, decaying ×~0.62) during part of the run: not copied.

## Amphibians (turtle, frog, axolotl)

- Turtle in water: drag **0.9** (its `water_movement` drag_factor), push 0.0141 = 0.98·0.12²,
  ramping as legs do (pace +0.1 a tick), yaw ≤5° a tick (`movement.amphibious` max_turn 5).
  Turtles summoned on land walked to the water (`move_to_water`).
- Frog in water: push 0.0221 = 0.98·0.15², drag 0.8. Axolotl: push 0.0392 = 0.98·0.2², max_turn 15.
- Vertical pushes small (~−0.0005, drag 0.9): not fitted. The frog on land stayed there and jumped
  (`jump_to_block`): not copied.

## Flyers

- Bat: push **0.1** along the 3D unit direction to its spot; drag 0.91 across, **0.6** up and down,
  no gravity. Re-targets every 2–7 ticks (`float_wander` float_duration 0.1–0.35 s), xz 10, y 7,
  y_offset −2. It never hung in a 12-high dark room in 40 s.
- Allay: drag ≈0.8645 every way (best cluster 0.85–0.8645), push 0.0098 = 0.98·0.1² ramping
  +0.00098 a tick (pace +0.1); terminal 0.0723. `random_hover` xz 8, y 8, offset −1, hover height
  1–4. Its vertical controller is not fitted.
- Parrot: falls at spawn as `v' = (v − 0.08)·0.98·0.6` (terminal −0.1142); takes off climbing
  +0.02 a tick; flies with ≈0.0133 per axis (0.0188 in all) and drag ≈0.92; hovered bobbing ~8 over
  the floor and landed once.

## Grazers (horse, donkey, mule, llama)

- Stroll terminal speeds are the legs rule (`0.98·(0.7·movement)²/0.454`): horse 0.10375 with its
  rolled movement 0.3132 (the pack rolls 0.1125–0.3375; sapling takes the middle), donkey and mule
  0.0324, llama 0.0661. Legs last 100–400 ticks; none gave up within ~300.

## Hoglin, zoglin, warden (`bds-beasts.txt`; the actor in survival on easy, healed every second)

- Each took the actor as its target (TargetEid) and walked at it by the legs rule: hoglin steps
  0.19427 (movement 0.3), zoglin 0.1784 (0.25 × melee 1.15), warden up to 0.271 (0.3 × 1.2).
  Hoglin and zoglin: `mob.warning` a tick after the target, `angry` a tick after that, both at
  0.85 of their height.
- Hits: ArmSwing, the actor thrown (±0.4 across, 0.4 up) as by any hunter, `attack` a tick later at
  half its height. Hoglin every 40 ticks (`cooldown_time` 2), zoglin and warden every 20. On easy
  the hoglin's hits cost 2.58–5.04 and the zoglin's 3.10–4.56, fractions: `[3, 9]` and `[3, 8]` are
  drawn as floats (`min(h, h/2 + 1)` on easy). The warden's cost 16 (30 on normal).
- The warden took no target until hit (51377: TargetEid, `mob.warning` the same tick), and was not
  moved by the hit (`knockback_resistance` 1; BDS's iron golem in `bds-golem.txt` kept its y the same way).
  No sonic boom: the actor never left its reach.

## Creaking (`bds-beasts.txt` at noon, `bds-creaking.txt` at midnight)

- Unwatched it strolls at 0.0311 (movement 0.4 × 0.3), day or night; it took no target in 10 s
  behind the actor's back.
- Looked at by the survival actor: TargetEid that tick, then `mob.warning`, `stare` (at 0.85 of its
  height) and `activate` (at its feet) a tick later; it stops and is heard `stare` every 3 ticks.
- Looked away from: `unfreeze` (feet), then it charges at 0.345 (0.4 by the legs rule) and hits
  every 40 ticks for 3 (2.5 on easy), throwing as any hunter. Looked at again: `freeze`.

## Crowds (`bds-crowd.txt`)

- Four cows summoned on one spot, and four pigs in a 2×2 pen, stayed exactly together: no push
  under 0.01 apart.
- Once a cow strolled off the spot its steps grew past its walk by exactly
  `2 × 0.05 × sqrt(d)` for each cow it overlapped (`d` the larger of |dx|, |dz| at the tick's
  start; Java's `Entity.push` with `min(1, 1/sqrt d)`, felt from both sides): 0.0707 at 0.125 from
  two cows, 0.107 at 0.285, 0.151 at 0.573, 0.1995 at 0.995. The cows it left were pushed the
  other way, 0.0354 each. They still pushed 0.995 apart, past their 0.9-wide boxes, and no more at
  1.561: sapling counts boxes 0.2 wider (an untested pick between the two).

## Sapling's run of the same drills (2026-10-27, tools/remote-drills.sh)

Path over the run, BDS → sapling: cod 43 → 44, salmon 57 → 61, pufferfish 69 → 71, tropical
fish 57 → 59, squid 13 → 11, allay 48 → 50 (both max step 0.0723), bat 192 → 294 (sapling's flies
faster: its spot changes less), parrot 47 → 156 (BDS's sat and landed), land turtle 126 → 139,
water turtle 150 → 159, axolotls 88/156 → 166/217, land frog 34 → 18 (no jumping). Donkey and mule
0.0324 and llama 0.0661 steps match; the horse is slower (middle of its range).
