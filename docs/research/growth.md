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
these fit, and a pitcher crop by a crop's odds, two high from its third stage. Ice is left: it
melts by light, which sapling does not reckon. Copper is left too: a stage every random tick or
two would weather a block in minutes at the rule's 1, which is not what the game is known for.

`bds_command_probe copper` (`bds-copper.txt`) bore that out: of 25 blocks of copper none
changed in a minute at the rule's 1, nor in half a minute more at 20, where the pace seen at 200
would have turned each several times. So copper does not weather in step with the rule, and
what it did at 200 is no guide. The same run's wet farmland, a torchflower crop on it and no
water near, dried a step after 40 seconds at 1 and a step every 5 seconds or so at 20: slower
at 20 than a tenth of its pace at 200 would be, so farmland is not in step with the rule either,
and sapling's three random ticks to a crop's one is right only near 200 and near 1. The
torchflower crop did not grow in the minute and a half.

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
on average, which was not looked into, and BDS's held leaf stayed marked where sapling clears it.

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

Read from the bds-blocks build with bdsre (labels in `bdsre/names.user.json`); not drilled.
"Light" is `BlockSource::getRawBrightness` (1415f7870): sky light less `skyDarken` where the
night counts ("darkened"), or block light if brighter. [D] decompiled, [I] inferred.

| What | BDS | Sapling |
| --- | --- | --- |
| crops (`CropBlock::randomTick` 1471459b0) | grow in darkened light >= a field (9 [I]) in their own cell [D] | 9 in the cell |
| crops (`CropBlock::canSurvive` 147146150) | break when light, night not counted, < a field (8 [I]) and the cell is under the heightmap [D] | on a neighbour's change (`blocks/ground.rs`), not on placing [I] |
| stems (146616700) | darkened light >= 9 in their cell [D] | as BDS |
| bamboo (1458106a0, 145812a70) | one tick in 3, darkened light >= 9 over it; a stalk up to 12 + a hash of its column % 5 [D] | the light; its odds and height stay as measured above |
| berry bush (1471efab0), cocoa (148b2b230), vines (1425e0490), cave vines (148b208d0) | no light read [D] | none |
| grass (`GrassBlockBase::randomTick` 147165820) | dirt if `_canBeGrass` (147165b50) fails: liquid or a block that keeps light out over it; with darkened light >= 9 over it, 4 tries 1 across, 3 under to 1 over, at dirt that could be grass [D] | dies under a block of filter 15 or liquid; the dirt must be open to the air |
| mycelium (1465eb280) | light, night not counted: dies with < 4 over it under a solid block; with >= 9 over it, 4 tries 1 across, 2 either way, at dirt with >= 4 over it and nothing solid or liquid [D] | as BDS, the dirt open to the air |
| ice (`IceBlock::randomTick` 142fd3470) | block light in its cell > 11 - its light block (0): water, air in an ultrawarm dimension [D] | block light in its cell, or one less beside it, >= 12: sapling's light table gives ice filter 2, where BDS's constructor gives none |
| snow layer (`TopSnowBlock::randomTick` 1471fb850) | block light b >= 12 in its cell melts b - 11 layers, up to 8 [D]; also one layer now and then in a biome over 0.15 when not snowing | the light; not the warm biome |
| freezing (`LevelChunk::tick` 1409f98a0) | `randomTickSpeed` picks a chunk a tick at the rain top; one in 16 freezes still water that is not water on all four sides, biome <= 0.15, block light < 10 (`Weather::shouldFreeze` 14119e860), whatever the weather [D] | as BDS, up to 16 picks a column a tick, chunks within 4 of a player (`growth/snowfall.rs`) |
| snow lying | while raining, `doWeatherCycle` on, one in 256 - min(512 x rain x downfall, 240); block light < 12, cold, ground that holds it; layers up to the biome's most (`Weather::canPlaceTopSnow` 14119d880) [D] | one in 16 (full rain, downfall >= 0.47); a layer more on a snow layer under the biome's most ("Snow layers" below) |
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

## Snow layers

`Weather::canPlaceTopSnow` (14119d880) [D]: on a snow layer of height h (0 for one layer) a layer
more, unless h is 7 or the snow there is already as deep as the biome's most, `snow_accumulation`'s
top times eight in BDS's vanilla biome definitions (`behavior_packs/vanilla/__brarchive/biomes.brarchive`):
one layer for most cold biomes (`[0.0, 0.125]`), two for frozen oceans and rivers, cold beaches,
stone beaches and the taigas, four for the cold taigas, eight for ice plains, twelve for ice
mountains and ice spikes. Sapling counts the layers in the cell and a full layer under it
(`growth/snowfall.rs`); BDS's smoothing, which may put the layer on a lower neighbour instead, is
not ported [I].

- Light: none of "Light" above was drilled against BDS; the crops' 9 and 8, the soils mushrooms
  keep to and snow's odds are inferred. Daylight detectors read the sky light in their cell by the
  reckoning in redstone.md.
- Trees from saplings: built from the decompile (saplings.md), not checked against the timing
  measured above.
- Farmland getting wet was not measured; a pumpkin's facing, and what ground a fruit takes, are
  the Java game's.
- Copper is not in step with the rule; fire and lit ore sapling times by themselves
  (`blocks/fire.rs`, `blocks/ores.rs`). Sapling's wild plants were not drilled against BDS's
  timing; a vine's sideways growth onto the top of a block (Java's last case) has no Bedrock state
  and is left out.
- Beetroot's and the other crops' own odds, which were not measured.
