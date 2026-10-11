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

## The rest of what random ticks do

Measured with `bds_command_probe grows` (`bds-grows.txt`, `sapling-grows.txt`), again at
`randomtickspeed` 200 and noon:

| What | On BDS |
| --- | --- |
| sugar cane by water, one high | a second cane after 9 seconds and a third after 14; then no more |
| cactus on sand | a second after 6 seconds, a third after 13 |
| ripe melon stem on farmland, dirt beside it | a melon on the dirt after 4 seconds, and the stem turned to it (`facing_direction` 5, east) |
| bamboo on dirt | a segment more every 2 seconds or so, thirteen high after 28 and growing; thin with small leaves to three high, then thick, the top two with large leaves, the third with small, none below |
| oak sapling on dirt | a tree after 42 seconds |
| dirt beside a grass block | grass after 2 seconds; the dirt beyond it after 8 |
| dry farmland with nothing on it, no water near | dirt in half a second |
| wet farmland, no water near | a step drier every 3 ticks or so, dirt after 1.3 seconds |

Cane and cactus fit the Java game's sixteen random ticks a segment at the pace above; neither
sent its age. Farmland dried three times as fast as that pace allows, so it is ticked more often.


Sapling keeps track of the crops, stems, berry bushes, cane, cactus, bamboo and farmland that
are written or come in with a column, and gives each the random tick's chance every tick,
rather than picking cells: picking them at `randomtickspeed` 200 cost more than a tick has
(`blocks/growth.rs`; what a tick does is in `growth/rules.rs`). A random tick grows wheat,
carrots, potatoes, beetroot and melon and pumpkin stems a stage by the Java game's odds, and a
sweet berry bush one time in five. A ripe stem puts its fruit on free dirt, grass or farmland
beside it and turns to it. Cane and cactus age sixteen ticks a segment up to three high, bamboo
grows one tick in five up to fourteen high with BDS's leaves, and farmland, ticked three times
as often, is wet by water within four, else dries a step and, bare and dry, turns to dirt.

In sapling's run of the drill (`sapling-crops.txt`) all 24 wheat were ripe between 7 and 35
seconds in, half of them by 19: BDS's pace.

## Left

- Light: the game grows nothing below light 9; sapling asks for none.
- Trees from saplings, and grass spreading to dirt: measured above, not built.
- Farmland getting wet was not measured; a pumpkin's facing, and what ground a fruit takes, are
  the Java game's.
- Everything else a random tick does: mycelium, leaves, ice and snow, copper, vines, kelp; fire
  and lit ore sapling times by themselves (`blocks/fire.rs`, `blocks/ores.rs`).
- Beetroot's and the other crops' own odds, which were not measured.
