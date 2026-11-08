# Bone meal

Measured 2026-10-11 with `bds_command_probe bonemeal`
(`crates/sapling/examples/shared/use_drill.rs`): the actor, in creative on stone and looking east,
clicks the top of a plant a block east of it with bone meal, several times. Captures:
`research/captures/bds-bonemeal.txt` and `sapling-bonemeal.txt`. Sapling's side is
`blocks/bonemeal.rs`.

A click that takes sends `LevelEvent ParticleCropGrowth`, data 0, from the corner of the clicked
cell, before the blocks change.

## What BDS did, in creative

| Plant | A click |
| --- | --- |
| wheat, carrots, beetroot at growth 0 | growth 7 at once; each later click still sent the particles |
| melon stem at growth 0 | growth 3, 6, 7; the fourth click sent nothing |
| sweet berry bush at growth 0 | growth 3; the next click picked it, as a bare hand does |
| oak sapling | a tree at once, its logs where the sapling was; later clicks sent nothing |
| short grass | tall grass, both halves |
| poppy | two more poppies on the grass beside it |
| grass block with air over it | short grass over it and on eight cells within two of it, a fern and an azure bluet among them; a second click, the grass now covered, sent nothing |
| sugar cane one high | three high (on sand without water, where it then broke) |
| bamboo sapling | bamboo with a small-leaved bamboo over it; a click more, one more bamboo on top |
| stone | nothing |

## Sapling

The same, for all but the tree: a sapling takes the bone meal and stays a sapling.

## Grass and flowers (decompiled 2026-11-05, thirteenth2p)

`GrassBlockBase::onFertilized` (147165f00; GrassBlock vtable 14a8e9550, slots 61, 132, 133): 48 random
walks from the cell over the grass, walk j (16 to 63) taking j / 16 steps, each step x and z by
nextInt(3) - 1 and y down one in nine, else up one in eight (`oneIn`, the level's random). A walk ends
where the cell under it is no grass block or the cell is solid (`_shouldStopRandomWalk` 147164d70).
At a free end `_plantGrass` (147164e10) plants short grass, a flower one in eight, else a fern one in
seven; on short grass with air over it, it grows it tall one in ten. Planting nothing takes no meal.
The flower (1438ee940) is the legacy flower features' by VanillaBiomeTags: 9 flower forest (noise
x/48 into the list at 14ccdae18), 1 plains (noise x/200 under -0.8 a tulip, else a dandelion one in
four or poppy, azure bluet, oxeye daisy, cornflower), 8 swamp (blue orchid), and for most others a
dandelion one in three, else a poppy. Named 2026-11-08 (fourteenth N; tags off 14393d890's HashedStrings,
0x30 apart, read at registry +0x98 + 0x18 x tag; blocks by the VanillaBlockTypeIds initialiser's `lea`s):
23 `pale_garden` a closed eyeblossom; 24 `meadow` or 11 `birch`, unless 12 `hills`, wildflowers with
`growth` `nextInt(3)`; 10 `forest` or 22 `forest_generation` (a tag no biome definition carries), unless
5 `extreme_hills` or 4 `taiga`, a lily of the valley on `nextBoolean`, else a dandelion
(`sapling_worldgen::meal_flower`, test `bone_meal_flowers_follow_the_biomes_tags`). Not drilled live.
`FlowerBlock::onFertilized` (14715ca10): the same 48 walks from the cell over the flower, y by
(nextInt(3) - 1) x nextInt(3) / 2, a walk dropped where the cell under it is no grass block; each free
end gets the flower's own kind, a poppy and a dandelion turning into each other one in eight. Sapling
(`blocks/bonemeal/scatter.rs`, the flower from `sapling_worldgen::meal_flower`), tests
`bonemeal::grass_plants_only_where_its_walks_reach_over_grass`,
`a_flower_spreads_its_kind_up_to_three_cells_over_grass`. Sapling's scatter was tries within two
(one for a flower). BDS sent what came up in one UpdateSubchunkBlocks a sub-chunk (bds-p2meal.txt), as sapling
now does (`send::set_sectioned`).

## In survival

Measured with `bds_command_probe bonemeal2` (`bds-bonemeal2.txt`), eight clicks a plant:

| Plant | What BDS did |
| --- | --- |
| wheat at growth 0 | growth 2, 4, 6, 7; the four clicks after that sent nothing |
| carrots, beetroot | ripe at the second click, then nothing |
| melon stem | growth 3, then two more steps to 7, then nothing |
| sweet berry bush | a stage a click to 3; the click after picked it |
| oak sapling | `age_bit` set at the first click, the particles alone for five more, a tree at the seventh |

A crop's step is 2 to 4, evenly (`l2meal`, bds-l2meal.txt, 2026-11-05: 30 wheat at 0 clicked once
went to 2, 3 and 4, 10, 8 and 12 times, never the Java game's 5); sapling's was 2 to 5, now 2 to 4
(`bonemeal.rs` `CROP_STEP`; sapling-l2meal.txt the same). Sapling grows a bush
one; a ripe crop takes none and spends none, where in creative it still took it. A sapling
takes it as BDS's `SaplingBlock::onFertilized` does (saplings.md): a tree at once in creative,
else 45 in 100 a click to age and then to grow.

## More plants, in survival (2026-11-04, `bds_command_probe t3meal`, bds-t3meal.txt)

Three clicks a plant, the actor two west of it:

| Plant | What BDS did |
| --- | --- |
| vine on stone | nothing, no particles |
| twisting vines, one | 2, 2, 1 more up from the tip; weeping vines 3 down, then particles with no room |
| cocoa | a stage a click to age 2, then nothing |
| moss block | a patch of radius 2: 17 moss and 12 plants on it |
| rooted dirt | hanging roots under it once, then nothing |
| brown mushroom on podzol | particles each click, no growth; the red one grew huge |
| sugar cane three high | its bottom took nothing; its top grew two more |
| bamboo sixteen high | nothing |

Sapling (`blocks/bonemeal/more.rs`, `bonemeal.rs`): cocoa, rooted dirt and the nether vines as above
(a vine grows Java's geometric count from its tip, each cell an age one more, to 25); moss Java's
`moss_patch_bonemeal` on the shard (radius 1 or 2, corners kept three in four, plants six in ten);
cane takes it on its top alone and grows two; full bamboo takes none. Test
`bonemeal::cocoa_rooted_dirt_cane_tops_and_nether_vines_take_it_as_on_bds`. A `/setblock` vine or kelp
of age 0 came up with an age BDS drew (20, 18, 24), which sapling keeps at 0.

## Left

- Sea grass and kelp: the drill bot refused both clicks in water (`NotVisible`, bds-t3water.txt, twice).
  A dispenser settled it (bds-u1meal.txt, 2026-11-04): short sea grass under still water grew tall,
  top then bottom, with `ParticleCropGrowth` and the mouth's `block.click` (-1); tall sea grass took
  none, `block.click.fail` 1000 at the mouth and the meal kept (D13). Kelp grew one a dispense on both
  servers. Sapling does all this (`bonemeal.rs`, `dispensers/behaviours.rs` `meal`; test
  `dispensers::bone_meal_from_a_dispenser_grows_sea_grass_tall_once`). A player's click in water is
  still the bot's to make.
