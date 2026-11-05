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

## The random tick's pace

Read 2026-11-04 from `LevelChunk::tickImpl` (0x1409f98a0; the 2.5 at 1409fbe49, the loop at
1409fbfef). Each ticking chunk takes `n`, its highest section not air in both layers plus one, and
makes `floor(2.5 n) * randomTickSpeed` picks a tick. A pick draws a global LCG (0x14caa09f0,
`s = 3s + 0x3c6ef35f`, `r = |s >> 2|`): x = `r & 15`, z = `(r >> 8) & 15`, y = `(r >> 16) % 16n`
up from the chunk's bottom, and ticks the block in both layers. The LCG is even over a long run
(simulated). A cell's odds a tick are `floor(2.5 n) * rule / 4096 n`: one random tick every 2048
ticks per unit at n = 1, 1638 at 2, 1755 at 3, 1653 at 23. The old flat 1800 was the 20% gap below:
BDS's flat drill chunks are n = 1.

`g3rate` (bds-g3rate.txt against sapling-g3rate.txt; 256 lit ores at y -60 in four chunks made
n = 1, 2, 3 and 23 by a stone set high, three rounds of 12 s at 50, censored exponential fit) gave
ticks per random tick at rule 1 of 2147, 1781, 1830 and 1719 on BDS and 1999, 1549, 1734 and 1613
on sapling, against 2048, 1638, 1755 and 1653: each side two or three standard errors off the
formula, in opposite directions, and both following its shape. Sapling (`blocks/growth/pace.rs`) gives
each chunk its own odds from `World::filled_sections`; test
`a_chunk_with_blocks_above_its_lowest_section_ticks_a_cell_a_quarter_more_often_as_on_bds`.

The picks are drawn with replacement (no check in the loop at 1409fbfef), so past a rule of about 1600 a
cell comes up more than once a tick: `n` = 3 at 4096 is 2.33 random ticks a tick, Poisson. Sapling ticked
a cell at most once a tick (one roll a cell, and the drawn cells deduplicated), so dripstone grew some
four times slower than BDS's at 4096 (bubbles-and-dripstone.md, i2grow); no later merge had touched the
pace. `l2pace` (thirteenth2l_drill.rs; 91 stalactites under a pool in an `n` = 3 chunk, three rounds of
6 s at 4096 and one of 20 s at 256, each root's first growth at 0.0114 of its random ticks, censored
exponential fit): BDS 2.23 random ticks a tick at 4096 and 0.170 at 256 (bds-l2pace.txt), against
2.33 and 0.146; sapling before 1.01 and 0.172 (sapling-l2pace-before.txt), and at `TICK_SCALE` 2
only 0.27, the 4096 snowfall picks a column putting the server behind. Fixed: a rolled cell takes a
Poisson count of ticks by one roll (`pace::count`), the drawn cells keep their repeats
(`growth/sampled.rs`), and snowfall skips a dry pick that does not freeze before reading the column.
Sapling after: 2.36 and 0.132 (sapling-l2pace.txt); test
`a_cell_takes_its_chance_in_ticks_on_average_more_than_one_a_tick_past_one`. Re-run on the fix at
`TICK_SCALE` 1: `g3rate` 1672, 1553, 2013, 1659 (sapling-g3rate2.txt; timed from 0.6 s into each
round, so the lag between the rule's answer and its first tick drops out: BDS's capture so gives 1773,
1714, 2049 and 1727), `firepace` fire out after 20.6, 5.6 and 1.0 s, 7, 7 and 9 ores of 12
(sapling-firepace2.txt; BDS 20.0, 5.9, 1.0 s and 6, 8, 8).

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
sent its age. Farmland dried three times as fast as that pace allows here, but not in `l2farm` below.

## A second row

Measured with `bds_command_probe grows2` (`bds-grows2.txt`), as `grows`. The ice by the torch
was water half a second in and ran over the floor, so the snow layer, the kelp and the
torchflower crop say nothing.

| What | On BDS |
| --- | --- |
| cocoa on a jungle log | age 1 after 2.3 seconds, 2, its last, after 4.7 |
| nether wart on soul sand | ages 1 and 2 after 5 seconds, 3, its last, after 15 |
| pitcher crop on wet farmland | growth 1 after 1.5 seconds, 2 after 8, 3 after 10 with an upper half over it, 4 for both a moment on |
| vine on a stone wall | more vines on the wall's other faces |
| ice by a torch | water at its first random tick |
| block of copper | exposed after 0.9 seconds, weathered after 1.5, oxidized after 2 |
| cactus | grew as before; stone set beside it did not break it within a second |

Sapling grows cocoa one random tick in five and wart one in ten, the Java game's odds, which
these fit, and a pitcher crop by a crop's odds, two high from its third stage. Ice melts by a lamp
(`growth/cover.rs`, "Light" below). Frosted ice set by `/fill` ages a step at each look 20 to 40 ticks
apart by day or under glowstone and melts in about 5 s, and never in 20 s at midnight unlit
(bds-u1frost.txt, 2026-11-04); a corner two under glowstone, lit 9 itself and 12 over it, melted, so
sapling ages it where the brightest of it and its neighbours is over 11 less its age (`blocks/frost.rs`,
test `frosted_ice_melts_by_day_and_by_a_light_but_not_in_the_dark`). Copper is left: a stage every random tick or
two would weather a block in minutes at the rule's 1, which is not what the game is known for.

`bds_command_probe copper` (`bds-copper.txt`) bore that out: of 25 blocks of copper none
changed in a minute at the rule's 1, nor in half a minute more at 20, where the pace seen at 200
would have turned each several times. So copper does not weather in step with the rule, and
what it did at 200 is no guide. The same run's wet farmland, a torchflower crop on it and no
water near, dried a step after 40 seconds at 1 and a step every 5 seconds or so at 20: slower
at 20 than a tenth of its pace at 200 would be, so farmland is not in step with the rule either,
and sapling's three random ticks to a crop's one is right only near 200 and near 1. The
torchflower crop did not grow in the minute and a half.
`FarmBlock`'s random tick (147151550, its `BlockRandomTickEvent` lambda; 2026-11-05) does one step
a tick and nothing else paces it: water within 4 across (the rows up not read), or rain over it (> 0.15), wets it
to 7, else it dries a step, else with nothing growing on it it turns to dirt. So its gap to the rule
is the random tick's own pace, not farmland's; sapling's 3 to a crop's 1 stood in for that.
`l2farm` (thirteenth2l_drill.rs, bds-l2farm.txt, 2026-11-05: 64 dry cells and 64 wet ones under
wheat, no water near, `n` = 3 chunks, 40 s at 20 then 10 s at 200) settles it: BDS's dry farmland
went to dirt at 0.0124 a tick at 20 and 0.095 at 200, the wet dried 0.0117 and 0.099 steps a tick
(one step each but where two random ticks fell between sends), against the pace's 0.0114 and 0.114.
Farmland is ticked as any block; the "every 3 ticks" at 200 above was not seen again. Sapling's 3 to
1 removed (sapling-l2farm.txt; test `farmland_dries_a_step_at_each_random_tick_and_no_faster`).

`bds_command_probe t3copper` (2026-11-04, bds-t3copper.txt) set 25 lone blocks of copper at the rule's
100 for a minute: the median block was exposed after 19.9 s, weathered 30.8, oxidized 45.8 on BDS
(23, 22, 20 of 25 got there), 12.7, 26.1 and 35.0 on sapling (24, 23, 22), whose odds are the
binary's (copper.md O2). One sample a side; left as is. A second sample (`e3pace`, bds-e3pace.txt
against sapling-e3pace.txt, 28 blocks at 100 for a minute, means of those that got there): exposed
13.0 s vs 10.5, weathered 24.1 vs 23.5, oxidized 38.8 vs 32.0 (27/24/21 of 28 vs 28/27/22), within a
standard error of each other but for oxidized, cut at 60 s; the same odds, nothing changed. Lit
redstone ore in the same run (200 cells) went out after 795 vs 815 ticks at rule 1 (124 vs 126 out in
90 s), 75 vs 60 at 20 and 20.3 vs 16.7 at 100: sapling's random ticks some 20% quicker at the high rules,
two to three standard errors. Settled 2026-11-04: sapling ticked every chunk as BDS's tallest
("The random tick's pace").

## Leaves

Measured with `bds_command_probe leaves` (`bds-leaves.txt`, `sapling-leaves.txt`), at
`randomtickspeed` 200:

- A cube of oak leaves by no log, which nothing touched, did not decay in a minute.
- A cube of 26 around a log: nothing for five seconds. When the log was set to air all 26 were
  sent again in that tick with `update_bit` set, and each was air after its next random tick,
  the last 2.1 seconds on. Four oak saplings were left.
- The same cube with `persistent_bit` set, as a player's leaves have it: nothing changed.

`bds_command_probe leafreach` (`bds-leafreach.txt`): a log with a row of ten leaves run out from
it, and a leaf set and taken away at the far end. The decay came down the row a leaf every
quarter second or so and stopped at the fourth leaf from the log, which was sent marked and
stood for the twenty seconds left: wood holds leaves four away through leaves, not the Java
game's six.

Sapling marks the leaves in the 26 cells around a log or a leaf that goes, and a marked leaf's
random tick drops it, with what a leaf leaves, unless wood is within four of it through leaves,
which clears the mark (`blocks/leaves.rs`). Its capture has the same 26 marked in the tick and
all of them gone 1.2 seconds on. BDS's four saplings of 26 are more than sapling's table gives
on average, which was not looked into.

The held leaf's mark (2026-11-04, `bds_command_probe n2leafmark`, research/captures/bds-n2leafmark.txt: the
same row, then `/testforblock` on leaves one to five): the client was never sent the mark cleared, yet the
four held leaves, the fourth sent marked, tested `update_bit` false and none true (the fifth was gone). BDS clears it unseen; so does sapling now (it sent the cleared
leaf until then, `blocks/growth/rules.rs`).

## Farmland trodden down

Measured with `bds_command_probe trample` (`bds-trample.txt`): the actor, in survival, was put
over wet farmland and left to drop. Three drops of 0.4 and three of 0.8 left the farmland; a
drop of 1.5 and three of 3 made it dirt in the tick of landing. The drill is rough: standing on
farmland, which is lower than a block, `~-1` named the cell under it, so the later 1.5 drops and
the two onto wheat laid their farmland a level down and say nothing.

Sapling treads farmland to dirt under a survival player that lands on it, with the Java game's
chance, the drop less a half (`trample` in `blocks/growth.rs`); what grew on it breaks. Mobs and
creative players tread none, and the chance between 0.8 and 1.5 was not measured.

## Sapling

Sapling keeps track of the crops, stems, berry bushes, cane, cactus, bamboo and farmland that
are written or come in with a column, and gives each the random tick's chance every tick,
rather than picking cells: picking them at `randomtickspeed` 200 cost more than a tick has
(`blocks/growth.rs`; what a tick does is in `growth/rules.rs`). A random tick grows wheat,
carrots, potatoes, beetroot and melon and pumpkin stems a stage by the Java game's odds, and a
sweet berry bush one time in five. A ripe stem puts its fruit on free dirt, grass or farmland
beside it and turns to it. Cane and cactus age sixteen ticks a segment up to three high, bamboo
grows one tick in five up to fourteen high with BDS's leaves, and farmland, ticked three times
as often, is wet by water within four, else dries a step and, bare and dry, turns to dirt.
Dirt open to the air becomes grass with the chance the grass blocks near it have of picking it
(four tries in 45 cells each, the Java game's, which BDS's two and six seconds fit); the dirt is
what sapling keeps track of, not the grass. Bamboo's odds were changed from one tick in three
after sapling's capture, which shows it three times too fast.

In sapling's run of the drill (`sapling-crops.txt`) all 24 wheat were ripe between 7 and 35
seconds in, half of them by 19: BDS's pace.

## Light

Read from the bds-blocks build with bdsre (labels in `bdsre/names.user.json`); drilled 2026-10-29 as the list at
the end says.
"Light" is `BlockSource::getRawBrightness` (1415f7870): sky light less `skyDarken` where the
night counts ("darkened"), or block light if brighter. [D] decompiled, [I] inferred.

| What | BDS | Sapling |
| --- | --- | --- |
| crops (`CropBlock::randomTick` 1471459b0) | grow in darkened light >= a field (9 [I]) in their own cell [D] | 9 in the cell |
| crops (`CropBlock::canSurvive` 147146150) | break when light, night not counted, < a field (8 [I]) and the cell is under the heightmap [D]; but `neighborChanged` asks only the ground, and drilled wheat in the dark stayed through a change beside and below it | never break by light |
| stems (146616700) | darkened light >= 9 in their cell [D] | as BDS |
| bamboo (1458106a0, 145812a70) | one tick in 3, darkened light >= 9 over it; a stalk up to 12 + a hash of its column % 5 [D] | the light; its odds and height stay as measured above |
| berry bush (1471efab0), cocoa (148b2b230), vines (1425e0490), cave vines (148b208d0) | no light read [D] | none |
| grass (`GrassBlockBase::randomTick` 147165820) | dirt if `_canBeGrass` (147165b50) fails: liquid or a block that keeps light out over it; with darkened light >= 9 over it, 4 tries 1 across, 3 under to 1 over, at dirt that could be grass [D] | dies under a block of filter 15 or liquid; the dirt must be open to the air |
| mycelium (1465eb280) | light, night not counted: dies with < 4 over it under a solid block unless that has a tag (0x14cd22f48, unread); with >= 9 over it, 4 tries 1 across, 2 either way, at dirt with >= 4 over it and nothing solid or liquid [D]. Drilled: under stone in the dark it lived 60 s at 200 | spreads as BDS, the dirt open to the air; never dies (TODO in `growth/cover.rs`) |
| ice (`IceBlock::randomTick` 142fd3470) | block light in its cell > 11 - its light block (0): water, air in an ultrawarm dimension [D] | block light in its cell, or one less beside it, >= 12: sapling's light table gives ice filter 2, where BDS's constructor gives none |
| snow layer (`TopSnowBlock::randomTick` 1471fb850) | block light b >= 12 in its cell melts b - 11 layers, up to 8 [D]; also one layer now and then in a biome over 0.15 when not snowing: drilled at light 11 in plains, 8 layers in about 32 s at 200 | the light; in a warm biome one layer a random tick in 16 |
| freezing (`LevelChunk::tick` 1409f98a0) | `randomTickSpeed` picks a chunk a tick at the rain top; one in 16 freezes still water that is not water on all four sides, biome <= 0.15, block light < 10 (`Weather::shouldFreeze` 14119e860), whatever the weather [D] | as BDS, chunks that tick ("Where random ticks run"; `growth/snowfall.rs`); no cap on the picks since 2026-11-04 (mob-damage.md "What the rain's level drives") |
| snow lying | while raining, `doWeatherCycle` on, one in 256 - min(512 x rain x downfall, 240); block light < 12, cold, ground that holds it; layers up to the biome's most (`Weather::canPlaceTopSnow` 14119d880) [D] | as BDS, the biome's downfall from `BiomeDefinitionList` (`sapling_data::biome_downfall`); a layer more on a snow layer under the biome's most ("Snow layers" below) |
| mushrooms (1465ea600, `canSurvive` 1465eac10) | spread one tick in 25, at most 5 in 9x3x9; survive on mycelium and podzol [I], else light, night not counted, < 13 [D] | as BDS ("Wild plants" below) |
| sun (`BurnsInDaylightSystem` 148485460, `Actor::isInDaylight` 140d38060) | `skyDarken` <= 3, sky over the head by the heightmap, brightness over 0.5; not in water, nor rain in a biome over 0.15 (snow does not save it), no item in the head slot, not in powder snow [I]; fire for 8 s, no random roll [D] | sky light 15 at the head; mobs wear nothing here, powder snow not asked |

Kelp takes no random tick on BDS (no subscription found).

## Wild plants

Read from the bds-molang build with bdsre (2026-10-27) and measured with `bds_command_probe plants`
and `plantmeal` (`bds-plants.txt`, `bds-plantmeal.txt`), at `randomtickspeed` 200 and noon.

| What | BDS |
| --- | --- |
| mushrooms (`MushroomBlock::randomTick` 1465ea600) | one in 25; returns at the fifth of its kind in x and z 4, y 1 about it; then four steps of `(nextInt(3)-1, nextInt(2)-nextInt(2), nextInt(3)-1)`, each taken into air it could stand in, and a fifth that puts one there [D]. The drill's brown mushroom in a dark room put three more down in 60 seconds |
| mushroom bone meal (1465ead90) | the huge mushroom feature (143b7ef50) four times in ten, every time in creative; takes the meal either way [D] |
| vines (`VineBlock::randomTick` 1425e0490) | the Java 1.12 rule: one in four, a face of six; up into air with some faces (one draw of 16, the faces it can hold to), capped at five vines in 9x3x9; across to a missing face, capped, holding to the block or round a corner; else down into air or onto the vine under, no cap [D] |
| cave vines (148b208d0) | a berry vine fixes itself to head or body by what is under it; else two draws, one in ten grows a vine under a tip of age under 25, a berry one in nine of those [D]. The drill's vine set at age 25 did not grow |
| cave vine bone meal (148b219b0) | berries, the age kept, when it has none: shown as one UpdateBlock with `ParticleCropGrowth` at the cell's corner; a second click picked the berries (age 13 after, a glow berry, `pick_berries.cave_vines`) |
| kelp (142fdbf50, 142fdc460) | no random tick: each kelp schedules its growth on the random-tick queue, `floor(ln u / ln(1 - 0.14 x 3 / 4096)) + 1` steps on, and grows a cell into a source over it, a year older, up to 25 [D]. That queue runs `randomTickSpeed` steps a game tick: the drill's kelp at 200 put on a cell every 48 ticks or so, 9750 steps over 200 |
| kelp bone meal (142fdcbe0) | the top of the stalk, if it waits, grows the next tick; nothing if it does not wait [D] |

Sapling keeps these, the stalactites and lava with air over them in a list per world and draws
`randomTickSpeed / 1800` of the list a tick at random (`growth/sampled.rs`), so a cave full of them
costs a tick what it changes (`growth/wild.rs`, `vines.rs`, `kelp.rs`; `blocks/drips.rs`). Kelp in a
column that comes in waits to grow from the top of each stalk; BDS keeps the wait with the column.

## Where random ticks run

Drilled 2026-11-04 with `bds_command_probe tickrandom` (`examples/shared/sixth2_drill.rs`;
`research/captures/bds-tickrandom.txt`, `sapling-tickrandom.txt`) at `tick-distance` 4. Per chunk offset a wheat on
dry farmland, bare dry farmland, a copper block and oak planks with fire on them were set with the rule at 0, the bots
moved to chunk 0,0, the rule was 200 for 20 s, then every cell was read with `/testforblock`.

- In the tick diamond (`ticking::in_tick_region`, mob-damage.md): offsets 0,0, 0,1, 4,0, 4,1 (past the circle
  dx² + dz² <= 16), 3,2, 2,3 and 2,-3: farmland dry to dirt, copper weathered, the planks burnt away; the wheat grew
  at all but 0,0 (dry farmland, one random tick in 26).
- Outside it: 5,0, 5,1, 4,2 (inside the square), 3,3 and 3,-3: every cell as set, the fire still burning.
- 6,0, in a ticking area: as in the diamond.

So random ticks, and the random-tick queue fire and kelp wait on, run in a chunk only while it ticks: a player's
diamond or a ticking area. The queue's clock is the chunk's own: `LevelChunk::tick` (0x1409f98a0) ticks
`mRandomTickQueue` (+0x138) up to its own current tick (+8) plus `randomTickSpeed`, so a plant or fire out of range
waits as long again; the block queue (+0x130) is ticked up to the level's tick instead. Sapling: `ticking::Region`
gates the tracked cells (`blocks/growth.rs`), the drawn cells (`growth/sampled.rs`), snowfall's picks (kept columns
now included, `ticking::ticked_columns`), and holds kelp, fire and lit ore out of range (`Region::hold`); test
`random_ticks_run_only_in_the_tick_region_and_kept_columns_as_on_bds` (tick_circle.rs). A world nobody is in still
ticks everywhere, for headless shards. Drilled on sapling after the fix (`sapling-tickrandom.txt`): every
farmland, copper and fire answer as BDS's, in, out and kept. Two cells differ for other reasons: the wheat at 0,1, 4,0,
4,1 and 3,2 did not grow, being under the drill world's ground at y 60 to 63 (the wheat past z ±48 grew), and the
planks at 5,0 burnt 6 s after their fire was set, before the bots moved (fire's pace, since fixed: fire.md "Pace and rule").

The block queue, drilled 2026-11-04 with `bds_command_probe tickqueue` (`examples/shared/seventh2_drill.rs`,
`bds-tickqueue.txt`; `bds-tickqueue1.txt` is a first try whose commands past the diamond all answered
`testforblock.out`). From chunk 3,0 each offset got six repeaters at delay 4 to a lamp, a lava source and sand, each held
by a plug in one layer; one `/fill` took the plugs out and the bots went to chunk 0,0 a second later, for 5 s. At 4,0,
4,1 and the ticking area 6,0 the lava ran and the lines went off while they were away; at 4,2, 5,0 and 5,1 both
stopped mid-way and ran on once the bots were back (the lamps out some 0.25 s after), so the block queue and the
circuit run only in a chunk that ticks, but up to the level's tick, so what fell due meanwhile runs at once. The sand had
become falling blocks before the bots left; those out of range hung at y -35.9 for the 5 s and fell on on their return
(116183), the rest landed. Sapling: `Region::runs_due` gates the queued blocks, liquids, falls, buttons, plates,
scaffolding and lamps (`blocks/queued.rs`, `liquid.rs`, `falling.rs`, `doors.rs`, `plates.rs`, `scaffolds.rs`,
`redstone/apply.rs`), `redstone.rs` holds a circuit's sources and writes out of range, and `falling/flight.rs` moves only
falling blocks whose cell ticks; test `block_ticks_and_circuits_wait_out_of_the_tick_region_and_catch_up_as_on_bds`.
Sapling's drill (`sapling-tickqueue.txt`) answered every one of the 36 reads before and after as BDS's.
`tools/bench.sh` times worldgen (sapling-world's `fill_views`), which this does
not touch; the gate costs a tracked cell a map lookup and skips the block read for cells out of range.

## Snow layers

`Weather::canPlaceTopSnow` (14119d880) [D]: on a snow layer of height h (0 for one layer) a layer
more, unless h is 7 or the snow there is already as deep as the biome's most, `snow_accumulation`'s
top times eight in BDS's vanilla biome definitions (`behavior_packs/vanilla/__brarchive/biomes.brarchive`):
one layer for most cold biomes (`[0.0, 0.125]`), two for frozen oceans and rivers, cold beaches,
stone beaches and the taigas, four for the cold taigas, eight for ice plains, twelve for ice
mountains and ice spikes. Sapling counts the layers in the cell and a full layer under it
(`growth/snowfall.rs`).

Smoothing (14119dec0, called from `LevelChunk::tick` with it on) [D]: a layer meant for a cell
that would then hold h layers goes instead to the cell north (z - 1) or south (z + 1) of it at
the same y, whichever would hold fewer than h and fewest (north on a tie), and across a chunk's
edge only if fewer than h / 2; never east or west. A fresh layer (h 1) never moves. Checked on
bds-u1cold.txt's final field (2026-11-05, thirteenth2l): of z-neighbours in one chunk 1.6% were
over a layer apart, of x-neighbours 23%, of z-neighbours across a chunk's edge 46%. Sapling
since (`snowfall.rs` `smoothed`, assert in `water_freezes_and_snow_lies_where_it_is_cold_while_it_snows`),
sapling-u1cold.txt: 2.6%, 30% and 51% (its snow stopped 7 s sooner, the drill's waits being wall time). The same cell rule also lays a layer on air over a whole
layer where the biome piles past 8 (ice spikes, ice mountains), which sapling did not.
A small plant at the cell (`canPlaceTopSnow` takes air or a block whose snowlogging value
144c4e400 is 3) goes into the second layer under the snow, as BDS's freeze pass leaves short grass,
ferns and flowers (sapling-worldgen `block/layers.rs`); clearing the snow gives it back. Sapling
since 2026-11-05 (`blocks/waterlog.rs` `snow_keeps`, `snowfall.rs` `room`; unit test
`snow_keeps_a_small_plant_under_it_and_gives_it_back_when_cleared`). Not drilled live: the BDS drill
world is plains, too warm for snow.

- Light, drilled 2026-10-29 with `lightgrow`, `lightpoke` and `croprates` (`examples/shared/growth_drill.rs`;
  `research/captures/bds-lightgrow.txt`, `bds-lightpoke.txt`, `bds-croprates.txt`). The scenes use sealed rooms lit
  by light blocks, at `randomtickspeed` 200 for 60 s. Tests: `tests/integration/growth_drilled.rs`, `lit_rules.rs`.
  - Drilled same:
    - Wheat, carrots and melon stems grow at light 9 and not at 8.
    - Bamboo grows at 9 and not at 8.
    - Mycelium spreads at 9.
    - Covered grass turns to dirt at once.
    - Ice melts at block light 12 and not at 11.
    - Snow melts several layers a tick at 13 and one at 12.
    - A mushroom in daylight on stone breaks when a neighbour changes; one on mycelium stays.
    - Wheat, beetroot, carrots and potatoes ripen at one pace (median ripe about 20 to 26 s); beetroot is not slower.
  - Fixed:
    - Crops never break by light. They had broken in sapling as a neighbour changed in the dark, a fill's crops among them.
    - Mycelium under stone does not die.
    - Snow in a warm biome loses a layer one random tick in 16.
  - Left:
    - Grass spread at light 9 over its dirt, but within 60 s not at 10; sapling's did the reverse. Read as chance.
      Re-drilled 2026-11-04 (bds-e3light.txt against sapling-e3light.txt): grass at 9 and 10 and the mushroom at
      13 went the same way on both.
    - BDS did not break a mushroom at light 13 when a light block was set beside it (same re-drill: alike).
    - Freezing and snow lying, drilled in an ice plains flat world (`u1cold`, bds-u1cold.txt, 2026-11-04: two
      5x5 pools, a torch at 14 -60 7 by the second, 70 s of snow at random tick speed 30). Both froze the open pool
      whole. BDS's snow piled to 8 layers; sapling's lay one and stopped, its column top passing over a thin layer,
      fixed (`growth/snowfall.rs`, test `water_freezes_and_snow_lies_where_it_is_cold_while_it_snows`). By the
      torch BDS left 9 cells water, sapling 4: they fit a water cell lit one under the air above it on BDS and two
      under on sapling (its water's light filter of 2), which is the light engine's, not freezing's, and left.
      Looked again 2026-11-05 (thirteenth2e): `Weather::shouldFreeze` (14119e860) reads the water cell's own
      block light (1415f7530, byte 1) under 10, so the gap is the block light reaching the water. The filter
      can't just drop to 1: data3d heightmaps count `light_filter > 0` and other readers share it. BDS's
      relighter (SubChunkRelighter) has no label to break on; left until the light engine is ported.
      The sun melts nothing in ice plains; a warm biome's melt is above.
  - Sapling's drill world has ground at y 60 to 63 over the drills (`drill_server.rs`), so a scene that needs daylight
    at y -26 clears it first.
  - Daylight detectors read the sky light in their cell by the reckoning in redstone.md.
- Trees from saplings: built from the decompile (saplings.md), not checked against the timing
  measured above.
- Farmland getting wet was not measured; a pumpkin's facing, and what ground a fruit takes, are
  the Java game's.
- Copper is not in step with the rule; fire and lit ore sapling times by themselves
  (`blocks/fire.rs`, `blocks/ores.rs`). Sapling's wild plants were not drilled against BDS's
  timing; a vine's sideways growth onto the top of a block (Java's last case) has no Bedrock state
  and is left out.
- Each crop's own odds are drilled: all four ripen at one pace ("Light" above, `croprates`).
- Wild plants' timing, compared 2026-11-05 (thirteenth2l; `sapling-plants.txt`, and `l2shroom`:
  twelve dark rooms, a brown mushroom each, 60 s at 200, `bds-l2shroom.txt`, `sapling-l2shroom.txt`):
  same. Kelp grew its 9 cells at 2.5 s a cell on BDS, 2.2 s on sapling; mushrooms spread 24 times in 9
  rooms on BDS, 21 in 8 on sapling, first spreads alike (median 22 s and 26 s). The cave vine's set age
  is random on both (BDS's came up 25 and never grew). Vines were not in the drill.
