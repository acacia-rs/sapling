# What food does besides feeding

From `bds_command_probe sick` (`research/captures/bds-sick.txt`): the bot, kept hungry by the
hunger effect, eating one of each from its hand.

- Spider eye: poison for 100 ticks, each of one time.
- Rotten flesh: hunger for 600 ticks, 3 times of 5.
- Raw chicken: nothing, 3 times of 3.
- Poisonous potato: poison for 100 ticks, 3 times of 3.
- Golden apple: regeneration at the second level for 100 ticks and absorption (effect 22) for
  2400, both `Add` beside `CompletedUsingItem`; `UpdateAttributes` `minecraft:absorption` 4 of 16
  at once, and 0 when the effect was cleared.
- Milk, held 32 ticks like food: every effect `Remove`d with the ticks it had left,
  `CompletedUsingItem` with use method -1, and a bucket in the bottle's slot. No burp.
- A stronger hunger given over the rotten flesh's replaced it (`Update`), and nothing was left
  when that ran out.

## Not drilled

How often raw chicken does give hunger and for how long, the chances themselves (taken as the
Java game's 8, 3 and 6 in 10), what a hit does to absorption, a pufferfish, an enchanted golden
apple, suspicious stew, chorus fruit, honey, potions, a stew's bowl.

## Sapling

`sapling-data/src/food.rs` holds each food's effects, leftover and whether it cures;
`vitals/eating.rs` applies them, and absorption is a ninth effect: 4 health a level on top of
the player's own, which hits take first (`creatures/health.rs` `cushion`). Stews leave a bowl as
in the Java game.
