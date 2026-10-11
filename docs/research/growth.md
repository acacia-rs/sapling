# Random ticks and crops growing

Measured 2026-10-11 with `bds_command_probe crops` and `ores`
(`crates/sapling/examples/shared/use_drill.rs`; `research/captures/bds-crops.txt`,
`bds-ores.txt`). Sapling's side is `blocks/growth.rs`.

## What BDS did

- 25 lit redstone ores, `randomtickspeed` 1: 16 were out after 90 seconds, at even odds through
  the time, which fits one random tick a block every 1760 ticks or so.
- 24 wheat at growth 0 on wet farmland around a water block, at noon, `randomtickspeed` 200: each
  went up one stage at a time, 20 of them through all seven; most were ripe 17 to 24 seconds in,
  the slowest at 45. That is a stage every 57 ticks or so.

The Java game grows a crop in a full wet field one random tick in six (`CropBlock`: one in
`25 / speed + 1`, where the speed is 1, 3 for its wet farmland, 3/4 for each wet farmland around
it, and half of the sum in a field of its own kind that is not in rows). At one in six, a stage
every 57 ticks is one random tick a block every 1900 ticks for each unit of the rule: the ores'
pace. So BDS grows wheat by the Java game's odds, and picks some 2.3 cells of a sixteen-cube a
tick for each unit of `randomtickspeed`.

## Sapling

Sapling keeps track of the crops, stems and berry bushes that are written or come in with a
column, and gives each the random tick's chance every tick, rather than picking cells: picking
them at `randomtickspeed` 200 cost more than a tick has. A random tick grows wheat, carrots,
potatoes, beetroot and melon and pumpkin stems a stage by the Java game's odds, and a sweet berry
bush one time in five.

In sapling's run of the drill (`sapling-crops.txt`) all 24 wheat were ripe between 7 and 35
seconds in, half of them by 19: BDS's pace.

## Left

- Light: the game grows nothing below light 9; sapling asks for none.
- A ripe stem bears no fruit.
- Everything else a random tick does: saplings, cane, cactus, bamboo, grass and mycelium
  spreading, farmland drying, leaves, ice and snow, copper, and fire and lit ore, which sapling
  times by themselves (`blocks/fire.rs`, `blocks/ores.rs`).
- Beetroot's and the other crops' own odds, which were not measured.
