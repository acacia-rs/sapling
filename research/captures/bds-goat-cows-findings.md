# bds-goat-cows findings

Capture `research/captures/bds-goat-cows.txt` (drill `goat_cows`): noon, easy, actor **creative** at
(0.5, -19, 0.5), the same glass pen as run 3 in `bds-goat-findings.md` (read that first), 368 s.
Times are the capture's ms, ~50 a tick. Actor runtime 1 (unique -665719930879), watcher runtime 4
(-878).

- Goats (runtime/unique): **g5 baby** (flag 11, Scale 0.5) 10034 at (5.5, 0.5); g7 12037 (0.5, 5.5);
  g9 14033 (4.5, 4.5); g11 16034 (4.5, -3.5). Uniques -665719930877, -875, -873, -871.
- Cows: c6 11207 (-4.5, 0.5); c8 13033 (0.5, -4.5); c10 15034 (-3.5, 4.5); c12 17035 (-3.5, -3.5).
  Health 10, movement 0.25. EntitySpawn data 720898 = (11 << 16) | 2. `spawn` at feet +1.105, +0.765.
- Both pen runs' first summoned goat came out a baby.

## Targets

- **Goats target cows**: all 31 TargetEid takes are cows; none are players or goats. g5 took c6 1
  tick after c6 spawned (10.0 off), g7 on its spawn tick, g9 8 ticks, g11 6 ticks after spawn, each
  followed by `mob.warning` (baby `.baby`, feet +0.5525). Switches go cow to cow without a 0 (g5
  c6 -> c8 52633).
- Drops happen outside the pen (7-15 off) but also inside at 1.1-3.5 off (g11 31683, 37784, 42582).
  Unresolved. Cows never target.

## The four rams

| # | goat | ram start (TargetEid 0) | pre_ram, goat at | target, off | contact | hit |
|---|------|-------------------------|------------------|-------------|---------|-----|
| 1 | g5 baby | 108855, held c8 | 113553 (1.654, -1.715) | c8, 7.02 due E | 115205, +33 t, 12th step | **c8** |
| 2 | g9 | 180954 + warning | 183454 (-4.427, -5.276) | c12, ~5.2 due S | none | **miss** |
| 3 | g5 baby | 264154 + warning.baby | 268858 (4.666, 1.693) | c8, 6.86 due E | 270504, +33 t, 12th step | **c8** |
| 4 | g9 | 277453 + warning | 282952 (0.330, 5.329) | c10, 5.96 due N | 284503, +31 t, 10th step | **watcher** |

- **When no target is held, the ram takes the nearest cow**: c12 1.40 off (next nearest 3.66), c8
  9.2 (next 10.6), c10 2.02. The run-up spots line up with those cows. Run-up 94, 50, 94, 110 ticks.
  Stand 22 ticks in all four.
- The charge steps are the same as before, plus a **12th step 1.0825** (rams 1 and 3, 6.35 run). The
  baby charges exactly like an adult. After contact the goat coasts x0.546 (0.513, 0.265, 0.145).

## Hits on cows (rams 1 and 3, by the baby)

- Health **10 -> 9** (115254) and **9 -> 8** (270554): 1 each. No regen up to the kill.
- On the impact tick: ArmSwing on g5, **HurtAnimation 30** on c8, and **one** SetEntityMotion on
  c8. On the next tick: `attack` (feet +0.325) and `ram_impact` (feet) at the goat. No cow sound.

  | ram | goat contact | cow | motion | h |
  |-----|--------------|-----|--------|---|
  | 1 | (-4.696, -1.505) | (-5.366, -1.012) | (-0.8826, 0.2842, 0.1579) | 0.8967 |
  | 3 | (-1.685, 1.504) | (-2.192, 1.925) | (-0.8728, 0.2842, 0.1201) | 0.8811 |

  - It fits the player rule folded into one packet: (plain hit 0.4 toward the cow, 0.4 up) / 2 plus
    the push. vy 0.2842 = 0.2 + 0.0842.
  - The horizontal push along the charge is **0.719 / 0.721** (x fits in both rams; z fits ram 3
    0.128 vs 0.120, ram 1 is 0.04 high).
  - These equal the old baby-on-player hit (push 0.702 / 0.084 at s 0.9998), but here s = 1.0825.
    1.5 s^2 x 0.468 would give 0.823 / 0.099, so **the baby's push doesn't scale with s^2**.
    The adult's does (ram 4).
- Next tick: ram 1 pinned c8 on the west glass (x held at -5.55). Ram 3 carried it 3.36 in ~8
  ticks to that wall.
- **Panic**: two legs at **0.2108 a tick** = 2.1586 x (0.25 x 1.25)^2 (movement x 1.25), then a stop.
  It lasted 3.3 s (115.8-119.1 s) and 1.7 s (271.2-272.9 s). No flag change but MOVING.

## The miss and the no-HurtAnimation impact

- **Ram 2 (183454) missed. c12 walked off.** At pre_ram c12 was at (-2.07, -2.92).
  - The charge went there **diagonally** (40 deg, then 63 deg), not along an axis: 0.0282 ... 0.7097,
    then a short 0.8575 to (-2.31, -2.94). That is **0.24 from c12's pre_ram spot**; c12 itself
    was 2.8 away.
  - Then it coasted x0.546: 0.468, 0.256, 0.140, 0.076.
  - So the ram aims at the target's position at pre_ram and stops pushing within 0.25 (Java's
    closerThan 0.25). No wall was involved.
  - **A miss is silent**: no ArmSwing, `attack`, `ram_impact`, HurtAnimation or motion on anyone.
- **Ram 4's impact (284551) hit the watcher**, who stood between g9 and c10 at ~(0.5, 0.4).
  - ArmSwing, `attack` (feet +0.65) and `ram_impact`.
  - One SetEntityMotion on runtime 4: (0.0452, 0.1398, -1.4913), `tick: 5620`. No HurtAnimation,
    no damage.
  - That is the adult push-only rule exactly: s 0.9967 gives 1.4901 / 0.1788, and vy -0.0784 / 2
    makes 0.1396.
  - `land` for -878 at (0.61, -18.92, -3.22) came 5 ticks later. c10 behind was untouched.

## Cooldowns

- **First ram: g5 started at 98.8 s** after spawn (pre_ram at 113.5 s from its summon). It was in
  the pen with a target throughout, which fits 30-300 s.
- The later ram starts (g5 +155.3 s, g9 +96.5 s after the miss) each came 9-12 ticks after the
  goat landed back in the pen or on its wall. Their cooldowns ran out while it was outside, so
  those gaps are only upper bounds.
- **g7 never rammed although it was inside 242.6-321 s** holding c10 from 275 s. That is 9 s past
  spawn + 300 s (old g4: ~3 s past). Either the cooldown can run past 300 s, or the ram needs more
  than a target (g7 stood still at (-5.41, 6.03), ~300-318 s). g11 was outside from ~105 s.
- **Re-target after a ram start**: 38.0, 33.0, 35.5 s (30-40 s attack_cooldown; a miss starts it too).

## Other

- No goat hurt or targeted a goat (goat health 10 until the kill at 378729).
- 15 `jump_to_block` over the glass, each ending in DustParticles + `fall.small`/`fall.big` + `fall`.
- Sounds: spawn, mob.warning(.baby), jump_to_block, fall*, pre_ram, attack, ram_impact, land. No
  ambient, step or hurt sounds. Cow motions of 0.10-0.31 with no hurt are goat pushes.

## Sapling should (on top of bds-goat-findings.md)

- Let goats target cows and pick the nearest at ram start when they hold none; skip creative players.
- Aim the charge at the target's pre_ram position at any angle. Stop pushing within 0.25 and
  coast x0.546. Send nothing on a miss.
- On a mob hit, send one SetEntityMotion: (0.4 toward the target, 0.4 up) / 2 + push.
  - Baby push ~0.72 / 0.084 whatever s is. Baby damage 1; an adult hitting a cow wasn't seen.
  - The cow then panics at movement x 1.25.
