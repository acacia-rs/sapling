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

## Not drilled

The chances themselves (taken as the Java game's 8, 3 and 6 in 10), what a hit does to
absorption, suspicious stew, chorus fruit, what honey leaves in the hand (taken as a glass
bottle), the other potions, splash and lingering ones, what
a potion leaves in the hand (taken as a glass bottle), a stew's bowl.

## Sapling

`sapling-data/src/food.rs` holds each food's effects, leftover and whether it cures;
`vitals/eating.rs` applies them, and absorption is a ninth effect: 4 health a level on top of
the player's own, which hits take first (`creatures/health.rs` `cushion`). Stews leave a bowl as
in the Java game. The potions of the table are drunk to the same effect (`consumable`); any
other is water.
