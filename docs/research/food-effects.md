# What food does besides feeding

From `bds_command_probe sick` (`research/captures/bds-sick.txt`): the bot, kept hungry by the
hunger effect, eating one of each from its hand.

- Spider eye: poison for 100 ticks, each of one time.
- Rotten flesh: hunger for 600 ticks, 3 times of 5.
- Raw chicken: nothing, 3 times of 3; in `more` (research/captures/bds-more.txt) hunger for 600
  ticks, 4 times of 10.
- Pufferfish (`more`): poison at the second level for 1200 ticks, nausea (effect 9) for 300 and
  hunger at the third level for 300.
- Enchanted golden apple (`more`): regeneration at the second level for 600 ticks, absorption at
  the fourth for 2400 (16 health), resistance (11) and fire resistance (12) for 6000.
- A bottle of honey (`more`): held 40 ticks, 6 hunger and 1.2 saturation, poison `Remove`d, use
  method `Eat` and a burp.
- Poisonous potato: poison for 100 ticks, 3 times of 3.
- Golden apple: regeneration at the second level for 100 ticks and absorption (effect 22) for
  2400, both `Add` beside `CompletedUsingItem`; `UpdateAttributes` `minecraft:absorption` 4 of 16
  at once, and 0 when the effect was cleared.
- Milk, held 32 ticks like food: every effect `Remove`d with the ticks it had left,
  `CompletedUsingItem` with use method -1, and a bucket in the bottle's slot. No burp.
- A stronger hunger given over the rotten flesh's replaced it (`Update`), and nothing was left
  when that ran out.

## Potions (`drink`)

A potion put in the hand by `/replaceitem ... potion 1 <data>` and drunk
(`research/captures/bds-drink.txt`): held 32 ticks like food, then the effect's `Add` beside
`CompletedUsingItem` with use method `Consume`.

| data | effect | level | ticks |
| --- | --- | --- | --- |
| 0, 4 | none (water, awkward) | | |
| 14, 15, 16 | speed | I, I, II | 3600, 9600, 1800 |
| 21, 22 | instant health | I, II | |
| 23, 24 | instant damage | I, II | |
| 25, 26, 27 | poison | I, I, II | 900, 2400, 450 |
| 28, 29, 30 | regeneration | I, I, II | 900, 2400, 450 |
| 36 | wither | II | 800 |

The rest (`drink2`, `research/captures/bds-drink2.txt`):

| data | effect | level | ticks |
| --- | --- | --- | --- |
| 5, 6 | night vision | I | 3600, 9600 |
| 7, 8 | invisibility | I | 3600, 9600 |
| 9, 10, 11 | jump boost | I, I, II | 3600, 9600, 1800 |
| 12, 13 | fire resistance | I | 3600, 9600 |
| 17, 18, 42 | slowness | I, I, IV | 1800, 4800, 400 |
| 19, 20 | water breathing | I | 3600, 9600 |
| 31, 32, 33 | strength | I, I, II | 3600, 9600, 1800 |
| 34, 35 | weakness | I | 1800, 4800 |
| 37, 38, 39 | slowness IV, IV, VI with resistance III, III, IV | | 400, 800, 400 |
| 40, 41 | slow falling | I | 1800, 4800 |
| 43, 44, 45, 46 | effects 32, 33, 34, 35 | I | 3600 |

## Stews, chorus fruit, leftovers, chances (`foods`, `foods2`, `foods3`, `food-odds`)

Drilled 2026-10-29 (`research/captures/bds-foods.txt`, `bds-foods2.txt`, `bds-foods3.txt`,
`bds-food-odds.txt`); the bot kept hungry and eating from its hand.

- Every food's hunger and saturation were the Java game's but sweet and glow berries: 2 and 1.2
  on BDS, as a melon slice (sapling had 2 and 0.4; fixed). Saturation is kept up to the hunger.
- A stew (mushroom, beetroot, rabbit, suspicious) and a potion leave a bowl (324) and a glass
  bottle (431) in the hand, sent in `InventoryContent`; honey a glass bottle.
- Suspicious stew by its data value: 0 and 10 night vision 100 ticks, 1 jump boost 100, 2 weakness
  140, 3 and 11 blindness 140, 4 poison 220, 5 and 6 saturation 6, 7 fire resistance 60, 8
  regeneration 140, 9 wither 140, 12 nausea 140; 13 and 14 nothing (sapling gave nothing; fixed).
- Chorus fruit: before `CompletedUsingItem`, `MovePlayer` of mode `Teleport`, cause `ChorusFruit`,
  source entity type 1, to the eater and its viewers, and `CorrectPlayerMovePrediction`; the next
  tick `teleport` heard at the eyes where it was and where it came, then the burp at the new place.
  A sample moved (3.78, 0, -0.65). Sapling did not move the eater; it now tries 16 spots in the
  box 8 about the feet as the Java game (`vitals/chorus.rs`).
- Chances over every capture so far (sick, more, food-odds): raw chicken 10 of 33, rotten flesh 16
  of 25, poisonous potato 13 of 23: the Java game's 3, 8 and 6 in 10 within the spread (rotten
  flesh two deviations low); kept.
- Ominous bottle (`fx3`, research/captures/bds-fx3.txt): see effects-raids.md.
- Raw beef, porkchop and rabbit told saturation 1.8000001, the f32 product 3 x 0.3 x 2 (sapling
  1.8; fixed). The bowl or bottle left in an empty hand is not heard (sapling popped it; fixed).
- A suspicious stew's effect comes as `MobEffect` `Add` and then `Update`, same values; saturation
  from one lasts its 6 ticks, feeding a hunger a tick and leaving saturation be (effects.md;
  sapling made it instant; fixed).
- Run against sapling (research/captures/sapling-foods.txt, sapling-foods2.txt, sapling-foods3.txt)
  after the fixes above every food's values, effects and leftovers match. Chorus fruit's
  `teleport` sounds come before `CompletedUsingItem` on sapling, after it on BDS, and BDS told the
  cooldown at the start only (bds-small.txt had it at the end too); left. Sapling also tells
  hunger and saturation after a potion, which BDS did not.

## Splash and lingering potions (`clouds`, `clouds2`)

Drilled 2026-10-29 (research/captures/bds-clouds.txt, bds-clouds2.txt): poison thrown straight
down on the thrower, the watcher standing off along x.

- The thrower, hit, got the whole 900 ticks. Another got `1 - d / 4` of them, `d` the gap from
  where it broke to the nearest point of the other's box: 744, 519, 294 and 69 ticks with the box
  0.7, 1.7, 2.7 and 3.7 off. Sapling measured to the feet; fixed (`creatures/splashes.rs`).
- The lingering cloud: items.md, "Area effect clouds". It is added in the middle of the block it
  broke in (0.5, -19.5, 0.5 for a break at (0.508, -20, 0.5)); sapling added it where it broke
  (fixed, `shots/clouds.rs`).
- Run against sapling (research/captures/sapling-clouds.txt, sapling-clouds2.txt): the splash data,
  the 225 ticks of a lingering pulse and the glass sound match.

## Not drilled

What a hit does to absorption.

## Sapling

`sapling-data/src/food.rs` holds each food's effects, leftover and whether it cures;
`vitals/eating.rs` applies them, and absorption is a ninth effect: 4 health a level on top of
the player's own, which hits take first (`creatures/health.rs` `cushion`). Stews leave a bowl as
in the Java game. The potions of the tables are drunk to the same effect (`consumable`) but the last four,
whose effects the shard lacks and which are water.
